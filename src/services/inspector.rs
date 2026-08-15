//! Servicio principal de inspección.
//!
//! Orquesta la captura: recoge métricas y superficies de macOS, aplica reglas,
//! compara contra las baselines, persiste evidencia y expone las pocas acciones
//! seguras que el producto ofrece a GUI y CLI.
//!
//! Un principio recorre todo el archivo: **una superficie que falla no tumba la
//! captura**. Si `lsof` no está, si TCC no se puede leer o si `spctl` no
//! responde, esa sección queda vacía y se explica; el resto de la foto sigue
//! siendo útil.

use crate::config::{ConfigManager, RootCauseConfig};
use crate::meta;
use crate::models::{
    AiIncidentAdvice, Alert, AnomalyEvent, AuditRecord, CacheCleanResult, HardwareInfo,
    IncidentSummary, NetworkScan, PersistenceChange, PersistenceEntry, ProcessInsight,
    SecurityControl, Severity, SnapshotRow, SystemOverview, SystemSnapshot, TccOverview,
};
use crate::services::{
    ai::AiAdvisor,
    anomaly::{persistence_change_event, AnomalyTracker, DetectionInput},
    baseline::{self, NETWORK_SURFACE_ID, SECURITY_SURFACE, TCC_SURFACE},
    launchd, macos, netscan, network,
    persistence::{persistence_entry_key, PersistenceStore},
    report,
    resilience::ResilienceMonitor,
    rules, security, tcc, temp_scan,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::{Networks, Pid, System};

/// Estado incremental necesario para calcular deltas de E/S entre muestreos.
///
/// `seeded` distingue la primera vez que se ve un PID: `sysinfo` devuelve el
/// total acumulado desde que el proceso arrancó, así que restar contra cero
/// haría pasar por "escritura del intervalo" toda la vida del proceso. La
/// primera muestra siembra el contador y reporta delta 0.
#[derive(Default)]
struct ProcessIoBaseline {
    seeded: bool,
    read_total_bytes: u64,
    write_total_bytes: u64,
}

/// Motor principal del software.
pub struct InspectorService {
    system: System,
    networks: Networks,
    process_baselines: HashMap<u32, ProcessIoBaseline>,
    protected_names: HashSet<String>,
    own_pid: u32,
    store: PersistenceStore,
    config: RootCauseConfig,
    config_path: PathBuf,
    config_warning: Option<String>,
    resilience_monitor: ResilienceMonitor,
    anomaly_tracker: AnomalyTracker,
    /// Caché de firmas por ruta: `codesign` es caro y un binario no cambia de
    /// firma mientras corre.
    signature_cache: HashMap<String, crate::models::CodeSignature>,
}

impl InspectorService {
    /// Inicializa recursos persistentes y el estado de monitoreo.
    pub fn new() -> Result<Self> {
        let mut system = System::new_all();
        system.refresh_all();
        let networks = Networks::new_with_refreshed_list();

        // Procesos que nunca deben finalizarse desde la herramienta: matarlos
        // deja el equipo inutilizable o provoca un pánico del kernel.
        let protected_names = [
            "kernel_task",
            "launchd",
            "windowserver",
            "loginwindow",
            "opendirectoryd",
            "securityd",
            "syspolicyd",
            "configd",
            "powerd",
            "hidd",
            "coreaudiod",
            "notifyd",
            "diskarbitrationd",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();

        let own_pid = std::process::id();
        let store = PersistenceStore::new(meta::APP_DIR)?;
        let (config_manager, config_warning) = ConfigManager::load_or_default(meta::APP_DIR);
        let resilience_monitor = ResilienceMonitor::new(
            meta::APP_DIR,
            config_manager.path(),
            &config_manager.config().resilience,
        )?;

        let service = Self {
            system,
            networks,
            process_baselines: HashMap::new(),
            protected_names,
            own_pid,
            store,
            config: config_manager.config().clone(),
            config_path: config_manager.path().to_path_buf(),
            config_warning,
            resilience_monitor,
            anomaly_tracker: AnomalyTracker::default(),
            signature_cache: HashMap::new(),
        };

        for record in service.resilience_monitor.startup_audits() {
            let _ = service.store.record_audit(&record);
        }

        Ok(service)
    }

    // ── Accesores ───────────────────────────────────────────────────────────

    pub fn config(&self) -> &RootCauseConfig {
        &self.config
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn db_path(&self) -> &Path {
        self.store.db_path()
    }

    pub fn write_default_config_if_missing(&self) -> Result<String> {
        Ok(ConfigManager::write_default_if_missing(meta::APP_DIR)?
            .display()
            .to_string())
    }

    /// Persiste `config` en disco y actualiza el estado interno del motor.
    pub fn save_config(&mut self, config: &RootCauseConfig) -> Result<()> {
        ConfigManager::save_to_path(&self.config_path, config)?;
        self.config = config.clone();
        Ok(())
    }

    pub fn load_history(&self, limit: usize) -> Vec<SnapshotRow> {
        self.store.load_recent(limit).unwrap_or_default()
    }

    pub fn load_incidents(&self, limit: usize) -> Vec<IncidentSummary> {
        self.store.load_recent_incidents(limit).unwrap_or_default()
    }

    pub fn load_audits(&self, limit: usize) -> Vec<AuditRecord> {
        self.store.load_recent_audits(limit).unwrap_or_default()
    }

    /// Contexto de ejecución: usuario, privilegios y utilidades disponibles.
    pub fn environment(&self) -> macos::EnvironmentReport {
        macos::environment()
    }

    /// Notifica por el centro de notificaciones si la captura trae una señal
    /// crítica y la política de alertas lo permite. Es una comodidad, no el
    /// canal principal: la alerta ya está en la captura y en el historial.
    pub fn notify_if_critical(&self, snapshot: &SystemSnapshot) {
        if !self.config.alerting.notify_on_critical {
            return;
        }
        let Some(alert) = snapshot
            .alerts
            .iter()
            .find(|alert| alert.severity == Severity::Critical)
        else {
            return;
        };
        macos::notify(&format!("RootCause · {}", alert.title), &alert.detail);
    }

    pub fn latest_incident(&self) -> Option<IncidentSummary> {
        self.store.latest_incident().ok().flatten()
    }

    /// Frase rápida del último historial, para la barra de estado.
    pub fn latest_history_line(&self) -> String {
        self.store
            .latest_summary_line()
            .ok()
            .flatten()
            .unwrap_or_else(|| format!("Historial listo en {}", self.store.db_path().display()))
    }

    /// Información estática del equipo.
    pub fn get_hardware_info(&self) -> HardwareInfo {
        let cpu = self.system.cpus().first();
        HardwareInfo {
            os_name: macos::product_name(),
            os_version: format!("{} ({})", macos::product_version(), macos::build_version()),
            host_name: System::host_name().unwrap_or_default(),
            cpu_brand: cpu
                .map(|cpu| cpu.brand().trim().to_owned())
                .filter(|brand| !brand.is_empty())
                .or_else(|| macos::sysctl("machdep.cpu.brand_string"))
                .unwrap_or_default(),
            cpu_cores: self.system.cpus().len(),
            cpu_freq_mhz: cpu.map(|cpu| cpu.frequency()).unwrap_or(0),
            total_ram_gb: bytes_to_gb(self.system.total_memory()),
            architecture: std::env::consts::ARCH.to_owned(),
            model: macos::sysctl("hw.model").unwrap_or_default(),
        }
    }

    // ── Captura ─────────────────────────────────────────────────────────────

    /// Captura una instantánea completa del sistema.
    pub fn collect_snapshot(&mut self) -> Result<SystemSnapshot> {
        for record in self.resilience_monitor.heartbeat().unwrap_or_default() {
            let _ = self.store.record_audit(&record);
        }

        self.system.refresh_all();
        self.networks.refresh();

        let collected_at = Utc::now();
        let details = macos::process_details();
        let (mut processes, io_totals) = self.collect_processes(&details);

        // Firma de código: solo para el puñado de procesos que más la necesitan.
        if self.config.collection.verify_signatures {
            self.apply_signatures(&mut processes);
        }
        self.reclassify_with_signatures(&mut processes);

        processes.sort_by(|left, right| {
            right
                .severity
                .cmp(&left.severity)
                .then_with(|| right.score.cmp(&left.score))
                .then_with(|| right.cpu_percent.total_cmp(&left.cpu_percent))
        });

        let process_paths: HashMap<u32, String> = processes
            .iter()
            .map(|process| (process.pid, process.exe_path.clone()))
            .collect();

        let connections = macos::lsof_connections()
            .map(|raw| network::parse_lsof_field_output(&raw, &process_paths))
            .unwrap_or_default();

        let caches = temp_scan::scan(&self.config.thresholds.cache);
        let xprotect = security::scan_xprotect(&self.config.thresholds.xprotect, collected_at);

        let mut security_controls = security::scan_controls();
        let security_change_events =
            self.detect_security_changes(collected_at, &mut security_controls);

        let mut tcc_overview = tcc::scan();
        let tcc_change_events = self.detect_tcc_changes(collected_at, &mut tcc_overview);

        let mut persistence_entries =
            launchd::scan_persistence(false, self.config.collection.verify_signatures);
        let persistence_change_events =
            self.detect_persistence_changes(collected_at, &mut persistence_entries);

        let mut network_scan = netscan::scan(false, &collected_at.to_rfc3339());
        let network_change_events = self.detect_network_changes(collected_at, &mut network_scan);

        let services = self.collect_services();

        let mut overview = SystemOverview {
            cpu_usage_percent: self.system.global_cpu_usage(),
            memory_used_gb: bytes_to_gb(self.system.used_memory()),
            memory_total_gb: bytes_to_gb(self.system.total_memory()),
            network_rx_mb_delta: self
                .networks
                .list()
                .values()
                .map(|data| bytes_to_mb(data.received()))
                .sum(),
            network_tx_mb_delta: self
                .networks
                .list()
                .values()
                .map(|data| bytes_to_mb(data.transmitted()))
                .sum(),
            io_read_mb_delta: bytes_to_mb(io_totals.0),
            io_write_mb_delta: bytes_to_mb(io_totals.1),
            cache_total_mb: caches.total_mb,
            primary_severity: Severity::Healthy,
            primary_reason: "Sin señales fuertes en esta muestra".to_owned(),
        };

        let mut anomalies = self.anomaly_tracker.analyze(DetectionInput {
            collected_at,
            processes: &processes,
            connections: &connections,
            config: &self.config.anomaly,
        });

        // Los cambios contra baseline se añaden y todo se reordena junto: así un
        // cambio de alta severidad no queda fuera del recorte de alertas por
        // haberse detectado en otra fase.
        let mut change_events = persistence_change_events;
        change_events.extend(security_change_events);
        change_events.extend(tcc_change_events);
        change_events.extend(network_change_events);
        if !change_events.is_empty() {
            anomalies.extend(change_events);
            anomalies.sort_by(|left, right| {
                right
                    .severity
                    .cmp(&left.severity)
                    .then_with(|| right.score.cmp(&left.score))
                    .then_with(|| left.kind.cmp(&right.kind))
            });
        }

        let mut alerts = rules::build_alerts(
            rules::AlertBuildInputs {
                processes: &processes,
                connections: &connections,
                cache_entries: &caches.top_entries,
                security_controls: &security_controls,
                persistence_entries: &persistence_entries,
                xprotect: &xprotect,
                tcc: &tcc_overview,
                anomalies: &anomalies,
            },
            &mut overview,
            self.config.alerting.max_alerts,
        );

        if let Some(warning) = self.config_warning.as_ref() {
            alerts.push(Alert {
                severity: Severity::Warning,
                title: "Configuración con respaldo".to_owned(),
                detail: warning.clone(),
                pid: None,
                path: Some(self.config_path.display().to_string()),
                hint: "Corrige el JSON o genera uno limpio con `rootcause config init`".to_owned(),
            });
        }

        let mut snapshot = SystemSnapshot {
            collected_at,
            overview,
            alerts,
            agent_health: self.resilience_monitor.health().clone(),
            processes,
            caches,
            connections,
            network: Some(network_scan),
            events: Vec::new(),
            services,
            persistence_entries,
            security_controls,
            xprotect,
            tcc: tcc_overview,
            anomalies,
            incident: None,
        };

        apply_agent_health(&mut snapshot, self.config.alerting.max_alerts);

        if let Some(incident) = rules::derive_incident(&snapshot) {
            snapshot.incident = Some(incident.clone());
            let _ = self
                .store
                .persist_incident(&incident, self.config.collection.incident_limit);
        }

        if let Err(error) = self
            .store
            .persist_snapshot(&snapshot, self.config.collection.history_limit)
        {
            snapshot.alerts.push(Alert {
                severity: Severity::Warning,
                title: "Persistencia con advertencia".to_owned(),
                detail: format!("No se pudo guardar el historial SQLite: {error}"),
                pid: None,
                path: None,
                hint: "La app sigue funcionando; solo se pierde este punto del historial"
                    .to_owned(),
            });
        }

        Ok(snapshot)
    }

    /// Recorre los procesos vivos y calcula deltas de E/S contra la muestra
    /// anterior. Devuelve `(procesos, (bytes leídos, bytes escritos))`.
    fn collect_processes(
        &mut self,
        details: &HashMap<u32, macos::ProcessDetail>,
    ) -> (Vec<ProcessInsight>, (u64, u64)) {
        let mut processes = Vec::new();
        let mut active_pids = HashSet::new();
        let mut total_read = 0_u64;
        let mut total_write = 0_u64;

        for process in self.system.processes().values() {
            let pid = process.pid().as_u32();
            active_pids.insert(pid);

            let name = process.name().to_string_lossy().into_owned();
            let exe_path = process
                .exe()
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            let memory_mb = bytes_to_mb(process.memory());
            let cpu_percent = process.cpu_usage();
            let disk_usage = process.disk_usage();

            let baseline = self.process_baselines.entry(pid).or_default();
            let (read_delta, write_delta) = if baseline.seeded {
                (
                    disk_usage
                        .total_read_bytes
                        .saturating_sub(baseline.read_total_bytes),
                    disk_usage
                        .total_written_bytes
                        .saturating_sub(baseline.write_total_bytes),
                )
            } else {
                baseline.seeded = true;
                (0, 0)
            };
            baseline.read_total_bytes = disk_usage.total_read_bytes;
            baseline.write_total_bytes = disk_usage.total_written_bytes;

            total_read = total_read.saturating_add(read_delta);
            total_write = total_write.saturating_add(write_delta);

            let write_delta_mb = bytes_to_mb(write_delta);
            let detail = details.get(&pid);
            let (severity, score, reasons, category) = rules::classify_process(
                &name,
                &exe_path,
                cpu_percent,
                memory_mb,
                write_delta_mb,
                None,
                &self.config.thresholds.process,
            );

            let can_terminate = self.can_terminate_process(pid, &name);
            processes.push(ProcessInsight {
                pid,
                name,
                exe_path,
                parent_pid: process.parent().map(Pid::as_u32),
                user: detail.map(|item| item.user.clone()).unwrap_or_default(),
                cpu_percent,
                memory_mb,
                io_read_mb_delta: bytes_to_mb(read_delta),
                io_write_mb_delta: write_delta_mb,
                status: format!("{:?}", process.status()),
                category,
                severity,
                score,
                can_terminate,
                reasons,
                command_line: detail.map(|item| item.command_line.clone()),
                signature: None,
            });
        }

        self.process_baselines
            .retain(|pid, _| active_pids.contains(pid));

        (processes, (total_read, total_write))
    }

    /// Verifica la firma de los procesos que más lo justifican, dentro del
    /// presupuesto configurado.
    ///
    /// El criterio de selección importa tanto como la verificación: se prioriza
    /// lo que ya destaca (severidad) y lo que vive fuera de las rutas del
    /// sistema, que es donde una firma ausente significa algo.
    fn apply_signatures(&mut self, processes: &mut [ProcessInsight]) {
        let budget = self.config.collection.signature_budget;
        let trusted_prefixes = &self.config.anomaly.trusted_path_prefixes;

        let mut targets: Vec<usize> = (0..processes.len())
            .filter(|index| {
                let process = &processes[*index];
                if process.exe_path.is_empty() {
                    return false;
                }
                let lower = process.exe_path.to_ascii_lowercase();
                let outside_system = !trusted_prefixes
                    .iter()
                    .any(|prefix| lower.starts_with(&prefix.to_ascii_lowercase()));
                outside_system || process.severity >= Severity::Warning
            })
            .collect();

        targets.sort_by(|left, right| {
            processes[*right]
                .severity
                .cmp(&processes[*left].severity)
                .then_with(|| processes[*right].score.cmp(&processes[*left].score))
        });

        for index in targets.into_iter().take(budget) {
            let path = processes[index].exe_path.clone();
            let signature = match self.signature_cache.get(&path) {
                Some(cached) => *cached,
                None => {
                    let computed = macos::code_signature(&path);
                    self.signature_cache.insert(path, computed);
                    computed
                }
            };
            processes[index].signature = Some(signature);
        }
    }

    /// Vuelve a clasificar los procesos a los que se les acaba de resolver la
    /// firma: es una señal fuerte y debe entrar en el puntaje.
    fn reclassify_with_signatures(&self, processes: &mut [ProcessInsight]) {
        for process in processes.iter_mut() {
            if process.signature.is_none() {
                continue;
            }
            let (severity, score, reasons, category) = rules::classify_process(
                &process.name,
                &process.exe_path,
                process.cpu_percent,
                process.memory_mb,
                process.io_write_mb_delta,
                process.signature,
                &self.config.thresholds.process,
            );
            process.severity = severity;
            process.score = score;
            process.reasons = reasons;
            process.category = category;
        }
    }

    /// Estado de los servicios de launchd que más contexto aportan.
    fn collect_services(&self) -> Vec<crate::models::ServiceState> {
        const WATCHED: &[(&str, &str)] = &[
            ("com.apple.softwareupdated", "Actualización de software"),
            ("com.apple.mds", "Spotlight (indexación)"),
            ("com.apple.backupd", "Time Machine"),
            (
                "com.apple.TimeMachine.Protected",
                "Time Machine (snapshots)",
            ),
            ("com.openssh.sshd", "Acceso remoto SSH"),
            ("com.apple.screensharing", "Compartir pantalla"),
        ];

        let jobs = launchd::launchctl_jobs();
        WATCHED
            .iter()
            .map(|(label, display)| {
                let job = jobs.iter().find(|(_, _, name)| name == label);
                match job {
                    Some((pid, status, _)) => crate::models::ServiceState {
                        name: (*label).to_owned(),
                        display_name: (*display).to_owned(),
                        status: if pid.is_some() {
                            "En ejecución".to_owned()
                        } else {
                            "Cargado".to_owned()
                        },
                        start_type: "launchd".to_owned(),
                        pid: *pid,
                        last_exit_code: Some(*status),
                    },
                    None => crate::models::ServiceState {
                        name: (*label).to_owned(),
                        display_name: (*display).to_owned(),
                        status: "No cargado".to_owned(),
                        start_type: "launchd".to_owned(),
                        pid: None,
                        last_exit_code: None,
                    },
                }
            })
            .collect()
    }

    // ── Baselines ───────────────────────────────────────────────────────────

    /// Compara la persistencia contra la baseline y anota los cambios.
    /// Devuelve `true` si existía una baseline previa.
    fn diff_persistence_baseline(&self, entries: &mut Vec<PersistenceEntry>) -> bool {
        let Ok(baseline) = self.store.load_persistence_baseline() else {
            return false;
        };

        if baseline.is_empty() {
            let _ = self.store.replace_persistence_baseline(entries);
            return false;
        }

        let mut current_keys = HashSet::new();
        for entry in entries.iter_mut() {
            let key = persistence_entry_key(entry);
            current_keys.insert(key.clone());
            entry.change_status = match baseline.get(&key) {
                None => PersistenceChange::Added,
                Some(base) if base.command != entry.command => PersistenceChange::Modified,
                Some(_) => PersistenceChange::Unchanged,
            };
        }

        for (key, base) in &baseline {
            if !current_keys.contains(key) {
                let mut removed = base.clone();
                removed.change_status = PersistenceChange::Removed;
                removed.note = "Estaba en la baseline y ya no aparece.".to_owned();
                entries.push(removed);
            }
        }

        true
    }

    fn detect_persistence_changes(
        &self,
        collected_at: DateTime<Utc>,
        entries: &mut Vec<PersistenceEntry>,
    ) -> Vec<AnomalyEvent> {
        let had_baseline = self.diff_persistence_baseline(entries);
        if !had_baseline || !self.config.anomaly.watch_persistence {
            return Vec::new();
        }
        entries
            .iter()
            .filter(|entry| entry.change_status.is_change())
            .filter_map(|entry| persistence_change_event(collected_at, entry))
            .collect()
    }

    fn detect_security_changes(
        &self,
        collected_at: DateTime<Utc>,
        controls: &mut [SecurityControl],
    ) -> Vec<AnomalyEvent> {
        let mut items = security::control_watch_items(controls);
        let had_baseline = baseline::diff_surface(&self.store, SECURITY_SURFACE.id, &mut items);

        let status_by_key: HashMap<&str, PersistenceChange> = items
            .iter()
            .map(|item| (item.key.as_str(), item.change_status))
            .collect();
        for control in controls.iter_mut() {
            if let Some(status) = status_by_key.get(control.id.as_str()) {
                control.change_status = *status;
            }
        }

        if !had_baseline || !self.config.anomaly.watch_security_controls {
            return Vec::new();
        }
        items
            .iter()
            .filter(|item| item.change_status.is_change())
            .filter_map(|item| {
                baseline::surface_change_event(collected_at, &SECURITY_SURFACE, item)
            })
            .collect()
    }

    fn detect_tcc_changes(
        &self,
        collected_at: DateTime<Utc>,
        overview: &mut TccOverview,
    ) -> Vec<AnomalyEvent> {
        if !overview.readable {
            return Vec::new();
        }

        let mut items = tcc::permission_watch_items(overview);
        let had_baseline = baseline::diff_surface(&self.store, TCC_SURFACE.id, &mut items);

        let status_by_key: HashMap<String, PersistenceChange> = items
            .iter()
            .map(|item| (item.key.clone(), item.change_status))
            .collect();
        for permission in overview.permissions.iter_mut() {
            let key = format!("{}::{}", permission.service, permission.client);
            if let Some(status) = status_by_key.get(&key) {
                permission.change_status = *status;
            }
        }

        if !had_baseline || !self.config.anomaly.watch_tcc {
            return Vec::new();
        }
        items
            .iter()
            .filter(|item| item.change_status.is_change())
            .filter_map(|item| baseline::surface_change_event(collected_at, &TCC_SURFACE, item))
            .collect()
    }

    /// Cruza el escaneo de red con la baseline y devuelve si había una previa.
    fn annotate_network_changes(&self, scan: &mut NetworkScan) -> bool {
        let mut items = netscan::device_watch_items(&scan.devices);
        let had_baseline = baseline::diff_surface(&self.store, NETWORK_SURFACE_ID, &mut items);

        let status_by_key: HashMap<String, PersistenceChange> = items
            .iter()
            .map(|item| (item.key.clone(), item.change_status))
            .collect();
        for device in scan.devices.iter_mut() {
            if let Some(status) = status_by_key.get(&netscan::device_key(device)) {
                device.change_status = *status;
            }
            netscan::classify_device(device);
        }

        let present: HashSet<String> = scan.devices.iter().map(netscan::device_key).collect();
        for item in items
            .iter()
            .filter(|item| item.change_status == PersistenceChange::Removed)
        {
            if !present.contains(&item.key) {
                scan.devices.push(netscan::device_from_watch_item(item));
            }
        }

        scan.total_devices = scan.devices.iter().filter(|device| !device.is_self).count();
        scan.new_devices = scan
            .devices
            .iter()
            .filter(|device| device.change_status == PersistenceChange::Added)
            .count();
        had_baseline
    }

    fn detect_network_changes(
        &self,
        collected_at: DateTime<Utc>,
        scan: &mut NetworkScan,
    ) -> Vec<AnomalyEvent> {
        let had_baseline = self.annotate_network_changes(scan);
        if !had_baseline
            || !self.config.anomaly.enabled
            || !self.config.anomaly.watch_network_devices
        {
            return Vec::new();
        }
        scan.devices
            .iter()
            .filter_map(|device| netscan::new_device_event(collected_at, device))
            .collect()
    }

    // ── Acciones bajo demanda ───────────────────────────────────────────────

    /// Persistencia anotada con su estado de cambio. Uso principal: CLI.
    pub fn persistence_with_changes(&self, include_apple: bool) -> Vec<PersistenceEntry> {
        let mut entries =
            launchd::scan_persistence(include_apple, self.config.collection.verify_signatures);
        self.diff_persistence_baseline(&mut entries);
        entries
    }

    /// Fija el estado actual de persistencia como la nueva baseline conocida.
    pub fn accept_persistence_baseline(&self) -> Result<usize> {
        let entries = launchd::scan_persistence(false, false);
        let count = entries.len();
        self.store.replace_persistence_baseline(&entries)?;
        self.audit(
            "accept-persistence-baseline",
            &format!("{count} entradas"),
            Some("Baseline de persistencia actualizada"),
            None,
        );
        Ok(count)
    }

    /// Fija el estado actual de los controles de seguridad como baseline.
    pub fn accept_security_baseline(&self) -> Result<usize> {
        let controls = security::scan_controls();
        let items = security::control_watch_items(&controls);
        let count = items.len();
        self.store.replace_baseline(SECURITY_SURFACE.id, &items)?;
        self.audit(
            "accept-security-baseline",
            &format!("{count} controles"),
            Some("Baseline de seguridad actualizada"),
            None,
        );
        Ok(count)
    }

    /// Fija los permisos TCC sensibles actuales como baseline.
    pub fn accept_tcc_baseline(&self) -> Result<usize> {
        let overview = tcc::scan();
        if !overview.readable {
            return Err(anyhow!(
                "No se pueden leer los permisos TCC: concede Acceso total al disco a RootCause"
            ));
        }
        let items = tcc::permission_watch_items(&overview);
        let count = items.len();
        self.store.replace_baseline(TCC_SURFACE.id, &items)?;
        self.audit(
            "accept-tcc-baseline",
            &format!("{count} permisos"),
            Some("Baseline de permisos TCC actualizada"),
            None,
        );
        Ok(count)
    }

    /// Explora la red local, ya cotejada contra la baseline de red conocida.
    pub fn scan_network(&self, deep: bool) -> NetworkScan {
        let mut scan = netscan::scan(deep, &Utc::now().to_rfc3339());
        self.annotate_network_changes(&mut scan);
        scan
    }

    /// Fija los equipos actuales como la nueva "red conocida".
    pub fn accept_network_baseline(&self) -> Result<usize> {
        let scan = netscan::scan(false, &Utc::now().to_rfc3339());
        let items = netscan::device_watch_items(&scan.devices);
        let count = items.len();
        self.store.replace_baseline(NETWORK_SURFACE_ID, &items)?;
        self.audit(
            "accept-network-baseline",
            &format!("{count} dispositivos"),
            Some("Baseline de red conocida actualizada"),
            None,
        );
        Ok(count)
    }

    /// Login items del usuario. Provoca el diálogo de permiso de Automatización
    /// de macOS, así que solo se llama cuando alguien lo pide explícitamente.
    pub fn login_items(&self) -> Vec<PersistenceEntry> {
        launchd::login_items()
    }

    /// Eventos recientes de seguridad del log unificado. Es caro (segundos), por
    /// eso no forma parte de la captura periódica.
    pub fn security_events(&self, minutes: u32) -> Vec<crate::models::EventRecord> {
        macos::security_log_events(minutes, 60)
            .unwrap_or_default()
            .into_iter()
            .map(|(timestamp, provider, message)| {
                let lower = message.to_ascii_lowercase();
                let severity = if lower.contains("denied") || lower.contains("blocked") {
                    Severity::Warning
                } else {
                    Severity::Healthy
                };
                crate::models::EventRecord {
                    timestamp,
                    provider,
                    level: "info".to_owned(),
                    message,
                    severity,
                }
            })
            .collect()
    }

    /// Vacía cachés del usuario. `dry_run` simula sin borrar.
    pub fn clean_caches(&self, dry_run: bool) -> CacheCleanResult {
        let result = temp_scan::clean_user_caches(24, dry_run);
        if !dry_run {
            self.audit(
                "clean-caches",
                &format!("{} entradas", result.deleted_count),
                Some(&format!(
                    "Limpieza de ~/Library/Caches: {:.1} MB liberados, {} en uso saltados",
                    result.freed_mb, result.skipped_in_use
                )),
                None,
            );
        }
        result
    }

    /// Finaliza un proceso si la política local lo permite.
    pub fn terminate_process(&self, pid: u32) -> Result<String> {
        if !self.config.remediation.manual_actions_enabled {
            let error = anyhow!("Las acciones manuales están desactivadas por configuración");
            self.audit("terminate-process", &pid.to_string(), None, Some(&error));
            return Err(error);
        }

        let result = (|| {
            if pid == self.own_pid {
                return Err(anyhow!("La aplicación no se permite finalizar a sí misma"));
            }
            let process = self
                .system
                .process(Pid::from(pid as usize))
                .ok_or_else(|| anyhow!("El proceso ya no existe"))?;
            let name = process.name().to_string_lossy().into_owned();
            if !self.can_terminate_process(pid, &name) {
                return Err(anyhow!("Proceso protegido por política local"));
            }
            macos::terminate_process(pid)
        })();

        self.audit(
            "terminate-process",
            &pid.to_string(),
            result.as_ref().ok().map(String::as_str),
            result.as_ref().err(),
        );
        result
    }

    /// Revela un archivo en el Finder: la acción "segura" sobre una persistencia
    /// sospechosa. RootCause muestra dónde está; borrarlo es decisión del usuario.
    pub fn reveal_in_finder(&self, path: &str) -> Result<String> {
        let result = macos::reveal_in_finder(path);
        self.audit(
            "reveal-in-finder",
            path,
            result.as_ref().ok().map(String::as_str),
            result.as_ref().err(),
        );
        result
    }

    /// Regla de firewall sugerida para bloquear una IP.
    ///
    /// Deliberadamente **no** la aplica: modificar `pf` requiere root y editar
    /// la configuración global del firewall del equipo. RootCause entrega el
    /// comando exacto y lo audita; ejecutarlo es una decisión consciente.
    pub fn suggest_block_ip(&self, ip_or_endpoint: &str) -> Result<String> {
        let ip = network::extract_ip(ip_or_endpoint)
            .ok_or_else(|| anyhow!("No se pudo extraer una IP de '{ip_or_endpoint}'"))?;
        let suggestion = format!(
            "Para bloquear {ip} con el firewall de macOS, ejecuta como administrador:\n  \
             echo \"block drop quick from any to {ip}\" | sudo pfctl -a rootcause -f -\n  \
             sudo pfctl -E\nPara revertirlo:\n  sudo pfctl -a rootcause -F rules"
        );
        self.audit("suggest-block-ip", &ip, Some("Regla pf sugerida"), None);
        Ok(suggestion)
    }

    /// Genera y guarda un reporte forense en Markdown.
    pub fn generate_report(&self, snapshot: &SystemSnapshot) -> Result<String> {
        let content = report::build_report(snapshot, &self.get_hardware_info());
        let path = report::save_report(&content).context("No se pudo guardar el reporte")?;
        self.audit(
            "generate-report",
            &path.display().to_string(),
            Some("Reporte forense generado"),
            None,
        );
        Ok(path.display().to_string())
    }

    /// Exporta una instantánea a JSON en Descargas o Documentos.
    pub fn export_snapshot(&self, snapshot: &SystemSnapshot) -> Result<String> {
        let path = self.store.export_path();
        let json = serde_json::to_string_pretty(snapshot)?;
        fs::write(&path, json)
            .with_context(|| format!("No se pudo escribir {}", path.display()))?;
        Ok(path.display().to_string())
    }

    pub fn export_history_backup(&self) -> Result<String> {
        Ok(self
            .store
            .export_history_backup(self.config.collection.history_limit)?
            .display()
            .to_string())
    }

    /// Ejecuta el adaptador IA opcional sobre el incidente más reciente.
    pub fn explain_latest_incident_with_ai(&self) -> Result<AiIncidentAdvice> {
        let incident = self
            .latest_incident()
            .ok_or_else(|| anyhow!("No hay incidentes persistidos para enriquecer"))?;
        let result = AiAdvisor::new(self.config.ai.clone()).summarize_incident(&incident);

        match &result {
            Ok(advice) => {
                let _ = self.store.update_incident_ai(&incident.incident_id, advice);
                self.audit(
                    "ai-explain-latest",
                    &incident.incident_id,
                    Some(&advice.summary),
                    None,
                );
            }
            Err(error) => self.audit(
                "ai-explain-latest",
                &incident.incident_id,
                None,
                Some(error),
            ),
        }
        result
    }

    /// Política de finalización: nunca a sí mismo, nunca a los procesos que
    /// sostienen la sesión gráfica o el arranque.
    fn can_terminate_process(&self, pid: u32, name: &str) -> bool {
        if pid <= 1 || pid == self.own_pid {
            return false;
        }
        !self.protected_names.contains(&name.to_ascii_lowercase())
    }

    fn audit(
        &self,
        action: &str,
        target: &str,
        success_message: Option<&str>,
        error: Option<&anyhow::Error>,
    ) {
        let record = AuditRecord {
            occurred_at: Utc::now().to_rfc3339(),
            action: action.to_owned(),
            target: target.to_owned(),
            success: error.is_none(),
            detail: success_message
                .map(str::to_owned)
                .or_else(|| error.map(ToString::to_string))
                .unwrap_or_default(),
        };
        let _ = self.store.record_audit(&record);
    }
}

impl Drop for InspectorService {
    fn drop(&mut self) {
        if let Ok(record) = self.resilience_monitor.shutdown() {
            let _ = self.store.record_audit(&record);
        }
    }
}

/// Refleja la salud del agente en las alertas y en el veredicto global.
fn apply_agent_health(snapshot: &mut SystemSnapshot, max_alerts: usize) {
    use crate::models::AgentStatus;
    let health = &snapshot.agent_health;

    match health.status {
        AgentStatus::Healthy => {}
        AgentStatus::Recovered => {
            if snapshot.overview.primary_severity < Severity::Warning {
                snapshot.overview.primary_severity = Severity::Warning;
                snapshot.overview.primary_reason = health.summary.clone();
            }
            snapshot.alerts.insert(
                0,
                Alert {
                    severity: Severity::Warning,
                    title: "Agente recuperado tras un cierre abrupto".to_owned(),
                    detail: health.summary.clone(),
                    pid: None,
                    path: None,
                    hint: "Revisa la auditoría si la detención no fue esperada.".to_owned(),
                },
            );
        }
        AgentStatus::Degraded => {
            if snapshot.overview.primary_severity < Severity::Warning {
                snapshot.overview.primary_severity = Severity::Warning;
                snapshot.overview.primary_reason = health.summary.clone();
            }
            snapshot.alerts.insert(
                0,
                Alert {
                    severity: Severity::Warning,
                    title: "La resiliencia del agente requiere revisión".to_owned(),
                    detail: health.summary.clone(),
                    pid: None,
                    path: None,
                    hint: "Valida los cambios de configuración y evita reinicios repetidos."
                        .to_owned(),
                },
            );
        }
    }

    snapshot.alerts.truncate(max_alerts);
}

fn bytes_to_mb(bytes: u64) -> f32 {
    bytes as f32 / (1024.0 * 1024.0)
}

fn bytes_to_gb(bytes: u64) -> f32 {
    bytes as f32 / (1024.0 * 1024.0 * 1024.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AgentHealth, AgentStatus};

    #[test]
    fn conversiones_de_tamano() {
        assert!((bytes_to_mb(1_048_576) - 1.0).abs() < f32::EPSILON);
        assert!((bytes_to_gb(1_073_741_824) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn un_agente_degradado_eleva_el_veredicto() {
        let mut snapshot = SystemSnapshot {
            agent_health: AgentHealth {
                status: AgentStatus::Degraded,
                summary: "La configuración cambió".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        };
        apply_agent_health(&mut snapshot, 8);

        assert_eq!(snapshot.overview.primary_severity, Severity::Warning);
        assert_eq!(snapshot.alerts.len(), 1);
        assert!(snapshot.alerts[0].title.contains("resiliencia"));
    }

    #[test]
    fn un_agente_sano_no_agrega_ruido() {
        let mut snapshot = SystemSnapshot::default();
        apply_agent_health(&mut snapshot, 8);
        assert!(snapshot.alerts.is_empty());
        assert_eq!(snapshot.overview.primary_severity, Severity::Healthy);
    }
}
