//! Detección de comportamiento anómalo (heurísticas locales, V1).
//!
//! Ninguna de estas heurísticas sabe qué es un malware. Todas responden a la
//! misma idea: **una distorsión sostenida de un recurso es el primer indicio de
//! que algo está pasando**, sea una amenaza o una fuga de memoria.
//!
//! Dos principios gobiernan el módulo:
//!
//! * **Sostenido, no instantáneo.** Un pico de CPU no dispara nada; un pico que
//!   dura N muestras seguidas, sí. Por eso el tracker guarda historial por PID.
//! * **Correlación antes que sospecha.** Cada señal suma puntaje; el veredicto
//!   sale de la suma, no de una sola condición. Así un compilador legítimo no
//!   acaba en rojo por consumir CPU.
//!
//! Todo es local: no hay firmas descargadas ni consultas a servicios externos.

use crate::config::AnomalyConfig;
use crate::models::{
    AnomalyEvent, CodeSignature, ConnectionInsight, IncidentEvidence, PersistenceChange,
    PersistenceEntry, ProcessInsight, RiskLevel,
};
use crate::services::network;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Entradas de una ronda de detección.
pub struct DetectionInput<'a> {
    pub collected_at: DateTime<Utc>,
    pub processes: &'a [ProcessInsight],
    pub connections: &'a [ConnectionInsight],
    pub config: &'a AnomalyConfig,
}

/// Historial mínimo por proceso para distinguir un pico de una tendencia.
#[derive(Debug, Default, Clone)]
struct ProcessHistory {
    cpu_streak: u8,
    write_streak: u8,
    memory_baseline_mb: f32,
    memory_growth_streak: u8,
    /// Marca de la última vez que se vio este *nombre* con este PID.
    last_seen: Option<DateTime<Utc>>,
}

/// Rastro de reapariciones de un mismo nombre de proceso con PIDs distintos.
#[derive(Debug, Default, Clone)]
struct RespawnTrace {
    last_pid: u32,
    changes: u8,
    window_start: Option<DateTime<Utc>>,
}

/// Acumula historial entre capturas. Vive dentro del `InspectorService`.
#[derive(Debug, Default)]
pub struct AnomalyTracker {
    history: HashMap<u32, ProcessHistory>,
    respawns: HashMap<String, RespawnTrace>,
}

impl AnomalyTracker {
    /// Analiza una captura y devuelve los eventos anómalos, ordenados por
    /// severidad y puntaje.
    pub fn analyze(&mut self, input: DetectionInput<'_>) -> Vec<AnomalyEvent> {
        if !input.config.enabled {
            self.history.clear();
            self.respawns.clear();
            return Vec::new();
        }

        let mut events = Vec::new();
        let public_remotes = network::unique_public_remotes_by_pid(input.connections);
        let private_remotes = unique_private_remotes_by_pid(input.connections);
        let mut active_pids = Vec::new();

        for process in input.processes {
            active_pids.push(process.pid);
            if is_trusted(process, input.config) {
                continue;
            }

            let history = self.history.entry(process.pid).or_default();

            // ── CPU sostenido ────────────────────────────────────────────────
            if process.cpu_percent >= input.config.cpu_sustained_percent {
                history.cpu_streak = history.cpu_streak.saturating_add(1);
            } else {
                history.cpu_streak = 0;
            }
            if history.cpu_streak == input.config.cpu_sustained_samples {
                events.push(process_event(
                    input.collected_at,
                    process,
                    "sustained-cpu",
                    RiskLevel::Medium,
                    55,
                    "CPU sostenido por encima del umbral",
                    format!(
                        "{} lleva {} muestras seguidas por encima del {:.0}% de CPU.",
                        process.name, history.cpu_streak, input.config.cpu_sustained_percent
                    ),
                    "un proceso consume CPU de forma continuada, no en un pico puntual",
                    "Comprueba si el proceso corresponde a una tarea que iniciaste.",
                ));
            }

            // ── Crecimiento de memoria ───────────────────────────────────────
            if history.memory_baseline_mb == 0.0 {
                history.memory_baseline_mb = process.memory_mb;
            } else if process.memory_mb - history.memory_baseline_mb
                >= input.config.memory_growth_mb
            {
                history.memory_growth_streak = history.memory_growth_streak.saturating_add(1);
                if history.memory_growth_streak == input.config.memory_growth_samples {
                    events.push(process_event(
                        input.collected_at,
                        process,
                        "memory-growth",
                        RiskLevel::Medium,
                        50,
                        "Crecimiento sostenido de memoria",
                        format!(
                            "{} creció {:.0} MB sobre su línea base en esta ventana.",
                            process.name,
                            process.memory_mb - history.memory_baseline_mb
                        ),
                        "fuga de memoria o acumulación de datos en el proceso",
                        "Si el crecimiento no se detiene, reinicia el proceso y observa si se repite.",
                    ));
                    history.memory_baseline_mb = process.memory_mb;
                    history.memory_growth_streak = 0;
                }
            } else if process.memory_mb < history.memory_baseline_mb {
                history.memory_baseline_mb = process.memory_mb;
                history.memory_growth_streak = 0;
            }

            // ── Escritura agresiva ───────────────────────────────────────────
            if process.io_write_mb_delta >= input.config.aggressive_write_mb {
                history.write_streak = history.write_streak.saturating_add(1);
            } else {
                history.write_streak = 0;
            }
            if history.write_streak == input.config.aggressive_write_samples {
                events.push(process_event(
                    input.collected_at,
                    process,
                    "aggressive-write",
                    RiskLevel::High,
                    70,
                    "Escritura agresiva sostenida",
                    format!(
                        "{} escribió {:.0} MB en muestras consecutivas.",
                        process.name, process.io_write_mb_delta
                    ),
                    "escritura masiva compatible con cifrado, copia o volcado de datos",
                    "Identifica qué carpeta está creciendo antes de detener el proceso.",
                ));
            }

            // Un binario instalado con normalidad (`/Applications`, `/usr`…)
            // hablando con muchos destinos es lo que hace un navegador o un
            // cliente de sincronización. La señal está en que lo haga algo que
            // vive fuera de esas rutas, así que las heurísticas de red se
            // limitan a ese caso en vez de inundar la vista de falsos positivos.
            let installed_normally = is_trusted_path(&lower_path_of(process), input.config)
                && !matches!(process.signature, Some(CodeSignature::Unsigned));

            // ── Tráfico saliente inusual ─────────────────────────────────────
            if let Some(remotes) = public_remotes
                .get(&process.pid)
                .filter(|_| !installed_normally)
            {
                if remotes.len() >= input.config.public_destination_count {
                    let mut event = process_event(
                        input.collected_at,
                        process,
                        "unusual-outbound",
                        RiskLevel::High,
                        72,
                        "Tráfico saliente inusual",
                        format!(
                            "{} mantiene {} destinos públicos distintos.",
                            process.name,
                            remotes.len()
                        ),
                        "conexiones a múltiples destinos públicos desde un proceso no habitual",
                        "Revisa los destinos y confirma que el proceso debería salir a Internet.",
                    );
                    event.unique_public_remotes = Some(remotes.len());
                    event.evidence.push(IncidentEvidence {
                        kind: "remotes".to_owned(),
                        label: "Destinos".to_owned(),
                        value: remotes.join(", "),
                    });
                    events.push(event);
                }
            }

            // ── Barrido de la red local ──────────────────────────────────────
            if let Some(remotes) = private_remotes
                .get(&process.pid)
                .filter(|_| !installed_normally)
            {
                if remotes.len() >= input.config.local_scan_destination_count {
                    let mut event = process_event(
                        input.collected_at,
                        process,
                        "local-scan",
                        RiskLevel::High,
                        75,
                        "Posible barrido de la red local",
                        format!(
                            "{} contactó {} equipos distintos del segmento local.",
                            process.name,
                            remotes.len()
                        ),
                        "un proceso está enumerando equipos de la red interna",
                        "Confirma si es una herramienta de administración que ejecutaste tú.",
                    );
                    event.unique_private_remotes = Some(remotes.len());
                    events.push(event);
                }
            }

            // ── Ruta de ejecución sospechosa ─────────────────────────────────
            let lower_path = process.exe_path.to_ascii_lowercase();
            if !lower_path.is_empty()
                && input
                    .config
                    .suspicious_path_keywords
                    .iter()
                    .any(|keyword| lower_path.contains(&keyword.to_ascii_lowercase()))
            {
                events.push(process_event(
                    input.collected_at,
                    process,
                    "suspicious-path",
                    RiskLevel::High,
                    68,
                    "Ejecución desde una ruta inusual",
                    format!("{} se ejecuta desde {}.", process.name, process.exe_path),
                    "el software legítimo rara vez se ejecuta desde carpetas temporales o compartidas",
                    "Verifica el origen del binario antes de permitir que siga ejecutándose.",
                ));
            }

            // ── Binario sin firma fuera del sistema ──────────────────────────
            if input.config.watch_unsigned_binaries
                && matches!(process.signature, Some(CodeSignature::Unsigned))
                && !is_trusted_path(&lower_path, input.config)
            {
                events.push(process_event(
                    input.collected_at,
                    process,
                    "unsigned-binary",
                    RiskLevel::Medium,
                    58,
                    "Binario sin firma de código",
                    format!(
                        "{} no tiene firma de código y no vive en una ruta del sistema.",
                        process.name
                    ),
                    "un binario sin firma no puede atribuirse a ningún desarrollador",
                    "Comprueba de dónde salió el binario; en macOS casi todo viene firmado.",
                ));
            }

            history.last_seen = Some(input.collected_at);
        }

        // ── Reapariciones rápidas ────────────────────────────────────────────
        // Se resuelve fuera del bucle a propósito: dentro, un nombre con varias
        // instancias vivas a la vez (`Google Chrome Helper`, `mdworker_shared`)
        // parecería reaparecer en cada iteración y generaría un falso positivo
        // en la primera captura.
        events.extend(self.track_respawns(input.collected_at, input.processes, input.config));

        self.history.retain(|pid, _| active_pids.contains(pid));

        events.sort_by(|left, right| {
            right
                .severity
                .cmp(&left.severity)
                .then_with(|| right.score.cmp(&left.score))
                .then_with(|| left.kind.cmp(&right.kind))
        });
        events
    }

    /// Detecta procesos que mueren y renacen con PID nuevo varias veces en poco
    /// tiempo: el patrón de un agente con `KeepAlive` que algo mata, o de un
    /// implante reinstalándose.
    ///
    /// Solo se vigilan los nombres con **una sola instancia viva**. Los nombres
    /// multi-instancia (ayudantes de navegador, workers de Spotlight) van y
    /// vienen constantemente por diseño, y vigilarlos convertiría la heurística
    /// en una fábrica de ruido.
    fn track_respawns(
        &mut self,
        collected_at: DateTime<Utc>,
        processes: &[ProcessInsight],
        config: &AnomalyConfig,
    ) -> Vec<AnomalyEvent> {
        let mut by_name: HashMap<&str, Vec<&ProcessInsight>> = HashMap::new();
        for process in processes {
            by_name
                .entry(process.name.as_str())
                .or_default()
                .push(process);
        }

        let mut events = Vec::new();
        for (name, instances) in by_name {
            if instances.len() != 1 {
                // Multi-instancia: se olvida cualquier rastro previo para que un
                // proceso que pasa de una a varias instancias no arrastre estado.
                self.respawns.remove(name);
                continue;
            }
            let process = instances[0];
            if is_trusted(process, config) {
                continue;
            }

            let trace = self.respawns.entry(name.to_owned()).or_default();

            if trace.last_pid == 0 {
                trace.last_pid = process.pid;
                trace.window_start = Some(collected_at);
                continue;
            }

            // La ventana caduca para que reinicios repartidos en horas no se
            // acumulen hasta disparar un falso positivo.
            if let Some(start) = trace.window_start {
                if (collected_at - start).num_seconds() > config.respawn_window_secs as i64 {
                    trace.changes = 0;
                    trace.window_start = Some(collected_at);
                }
            }

            if trace.last_pid == process.pid {
                continue;
            }

            trace.last_pid = process.pid;
            trace.changes = trace.changes.saturating_add(1);

            if trace.changes == config.respawn_count {
                events.push(process_event(
                    collected_at,
                    process,
                    "fast-respawn",
                    RiskLevel::High,
                    66,
                    "Proceso que se relanza repetidamente",
                    format!(
                        "{} cambió de PID {} veces en menos de {} s.",
                        process.name, trace.changes, config.respawn_window_secs
                    ),
                    "un proceso muere y renace: puede ser un agente con KeepAlive o un implante persistente",
                    "Busca el LaunchAgent o LaunchDaemon que lo relanza en el tab Persistencia.",
                ));
            }
        }

        events
    }
}

/// `true` si el proceso está en la lista de confianza por nombre o por ruta.
fn is_trusted(process: &ProcessInsight, config: &AnomalyConfig) -> bool {
    let lower_name = process.name.to_ascii_lowercase();
    if config
        .trusted_process_names
        .iter()
        .any(|trusted| lower_name == trusted.to_ascii_lowercase())
    {
        return true;
    }
    // Un binario firmado por Apple en una ruta del sistema no necesita
    // heurísticas: si eso está comprometido, el problema es de otro orden.
    matches!(process.signature, Some(CodeSignature::Apple))
        && is_trusted_path(&process.exe_path.to_ascii_lowercase(), config)
}

/// Ruta del ejecutable en minúsculas, para comparaciones sin distinción de caja.
fn lower_path_of(process: &ProcessInsight) -> String {
    process.exe_path.to_ascii_lowercase()
}

fn is_trusted_path(lower_path: &str, config: &AnomalyConfig) -> bool {
    config
        .trusted_path_prefixes
        .iter()
        .any(|prefix| lower_path.starts_with(&prefix.to_ascii_lowercase()))
}

/// Destinos privados distintos por PID, insumo de la heurística de barrido.
fn unique_private_remotes_by_pid(connections: &[ConnectionInsight]) -> HashMap<u32, Vec<String>> {
    let mut table: HashMap<u32, Vec<String>> = HashMap::new();
    for connection in connections
        .iter()
        .filter(|item| !item.is_public_remote && !item.is_listening)
    {
        let Some(ip) = network::extract_ip(&connection.remote_address) else {
            continue;
        };
        if ip.is_empty() || ip.starts_with("127.") || ip == "::1" {
            continue;
        }
        let entry = table.entry(connection.pid).or_default();
        if !entry.contains(&ip) {
            entry.push(ip);
        }
    }
    table
}

/// Constructor común de eventos ligados a un proceso.
#[allow(clippy::too_many_arguments)]
fn process_event(
    detected_at: DateTime<Utc>,
    process: &ProcessInsight,
    kind: &str,
    severity: RiskLevel,
    score: u16,
    title: &str,
    summary: String,
    hypothesis: &str,
    action: &str,
) -> AnomalyEvent {
    AnomalyEvent {
        event_id: format!(
            "anom-{}-{}-{}",
            detected_at.timestamp_millis(),
            kind,
            process.pid
        ),
        detected_at,
        severity,
        score,
        status: "open".to_owned(),
        kind: kind.to_owned(),
        title: title.to_owned(),
        process_name: Some(process.name.clone()),
        pid: Some(process.pid),
        parent_pid: process.parent_pid,
        user: Some(process.user.clone()),
        exe_path: Some(process.exe_path.clone()),
        cpu_percent: Some(process.cpu_percent),
        memory_mb: Some(process.memory_mb),
        io_write_mb_delta: Some(process.io_write_mb_delta),
        summary,
        root_cause_hypothesis: hypothesis.to_owned(),
        recommended_action: action.to_owned(),
        evidence: vec![
            IncidentEvidence {
                kind: "process".to_owned(),
                label: "Proceso".to_owned(),
                value: format!("{} ({})", process.name, process.pid),
            },
            IncidentEvidence {
                kind: "path".to_owned(),
                label: "Ruta".to_owned(),
                value: if process.exe_path.is_empty() {
                    "sin ruta visible".to_owned()
                } else {
                    process.exe_path.clone()
                },
            },
        ],
        ..Default::default()
    }
}

/// Evento por un cambio en una entrada de persistencia respecto a la baseline.
///
/// Se separa del tracker porque no depende de historial en memoria: la
/// comparación la hace el motor de baseline sobre SQLite.
pub fn persistence_change_event(
    detected_at: DateTime<Utc>,
    entry: &PersistenceEntry,
) -> Option<AnomalyEvent> {
    let (severity, score, title, verb) = match entry.change_status {
        PersistenceChange::Added => (
            RiskLevel::High,
            78_u16,
            "Persistencia nueva detectada",
            "apareció",
        ),
        PersistenceChange::Modified => (
            RiskLevel::High,
            74,
            "Persistencia modificada",
            "cambió de comando",
        ),
        PersistenceChange::Removed => (
            RiskLevel::Medium,
            42,
            "Persistencia eliminada",
            "desapareció",
        ),
        PersistenceChange::Unchanged => return None,
    };

    // Una entrada nueva que además es sospechosa por sí misma escala a crítica:
    // dos señales independientes apuntando al mismo sitio.
    let severity = if entry.severity >= RiskLevel::High && entry.change_status.is_change() {
        RiskLevel::Critical
    } else {
        severity
    };

    Some(AnomalyEvent {
        event_id: format!(
            "anom-{}-persistence-{}",
            detected_at.timestamp_millis(),
            entry.name
        ),
        detected_at,
        severity,
        score,
        status: "open".to_owned(),
        kind: "persistence-change".to_owned(),
        title: title.to_owned(),
        exe_path: entry.target_path.clone(),
        summary: format!(
            "{} '{}' ({}) {} respecto a la baseline conocida.",
            entry.entry_kind,
            entry.name,
            entry.scope.label(),
            verb
        ),
        root_cause_hypothesis:
            "algo instaló, modificó o quitó un mecanismo de arranque automático".to_owned(),
        recommended_action:
            "Abre el plist, comprueba qué binario ejecuta y acepta la baseline solo si lo reconoces."
                .to_owned(),
        evidence: vec![
            IncidentEvidence {
                kind: "location".to_owned(),
                label: "Ubicación".to_owned(),
                value: entry.location.clone(),
            },
            IncidentEvidence {
                kind: "command".to_owned(),
                label: "Comando".to_owned(),
                value: entry.command.clone(),
            },
            IncidentEvidence {
                kind: "change".to_owned(),
                label: "Cambio".to_owned(),
                value: entry.change_status.label().to_owned(),
            },
        ],
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PersistenceScope;

    fn process(pid: u32, name: &str, cpu: f32) -> ProcessInsight {
        ProcessInsight {
            pid,
            name: name.to_owned(),
            exe_path: format!("/Users/x/bin/{name}"),
            cpu_percent: cpu,
            memory_mb: 100.0,
            ..Default::default()
        }
    }

    fn config() -> AnomalyConfig {
        AnomalyConfig::default()
    }

    #[test]
    fn un_pico_de_cpu_no_dispara_nada() {
        let mut tracker = AnomalyTracker::default();
        let config = config();
        let processes = vec![process(10, "ruidoso", 90.0)];

        let events = tracker.analyze(DetectionInput {
            collected_at: Utc::now(),
            processes: &processes,
            connections: &[],
            config: &config,
        });
        assert!(
            events.iter().all(|event| event.kind != "sustained-cpu"),
            "una sola muestra no debería declarar CPU sostenido"
        );
    }

    #[test]
    fn cpu_sostenida_dispara_tras_las_muestras_configuradas() {
        let mut tracker = AnomalyTracker::default();
        let config = config();
        let processes = vec![process(10, "ruidoso", 90.0)];

        let mut fired = false;
        for _ in 0..config.cpu_sustained_samples {
            let events = tracker.analyze(DetectionInput {
                collected_at: Utc::now(),
                processes: &processes,
                connections: &[],
                config: &config,
            });
            fired = events.iter().any(|event| event.kind == "sustained-cpu");
        }
        assert!(fired, "debe disparar al alcanzar la racha configurada");
    }

    #[test]
    fn la_racha_se_rompe_si_la_cpu_baja() {
        let mut tracker = AnomalyTracker::default();
        let config = config();

        for cpu in [90.0, 90.0, 5.0, 90.0] {
            let processes = vec![process(10, "ruidoso", cpu)];
            let events = tracker.analyze(DetectionInput {
                collected_at: Utc::now(),
                processes: &processes,
                connections: &[],
                config: &config,
            });
            assert!(events.iter().all(|event| event.kind != "sustained-cpu"));
        }
    }

    #[test]
    fn la_deteccion_desactivada_no_produce_eventos() {
        let mut tracker = AnomalyTracker::default();
        let mut config = config();
        config.enabled = false;
        let processes = vec![process(10, "ruidoso", 99.0)];

        let events = tracker.analyze(DetectionInput {
            collected_at: Utc::now(),
            processes: &processes,
            connections: &[],
            config: &config,
        });
        assert!(events.is_empty());
    }

    #[test]
    fn ruta_sospechosa_se_reporta_de_inmediato() {
        let mut tracker = AnomalyTracker::default();
        let config = config();
        let mut suspicious = process(11, "helper", 1.0);
        suspicious.exe_path = "/tmp/helper".to_owned();

        let events = tracker.analyze(DetectionInput {
            collected_at: Utc::now(),
            processes: &[suspicious],
            connections: &[],
            config: &config,
        });
        assert!(events.iter().any(|event| event.kind == "suspicious-path"));
    }

    #[test]
    fn binario_de_apple_en_ruta_del_sistema_se_ignora() {
        let mut tracker = AnomalyTracker::default();
        let config = config();
        let mut trusted = process(12, "helper", 99.0);
        trusted.exe_path = "/usr/libexec/helper".to_owned();
        trusted.signature = Some(CodeSignature::Apple);

        for _ in 0..5 {
            let events = tracker.analyze(DetectionInput {
                collected_at: Utc::now(),
                processes: &[trusted.clone()],
                connections: &[],
                config: &config,
            });
            assert!(events.is_empty());
        }
    }

    #[test]
    fn muchos_destinos_publicos_disparan_trafico_inusual() {
        let mut tracker = AnomalyTracker::default();
        let config = config();
        let connections: Vec<ConnectionInsight> = ["8.8.8.8", "1.1.1.1", "9.9.9.9", "4.4.4.4"]
            .iter()
            .map(|ip| ConnectionInsight {
                pid: 20,
                remote_address: format!("{ip}:443"),
                is_public_remote: true,
                ..Default::default()
            })
            .collect();

        let events = tracker.analyze(DetectionInput {
            collected_at: Utc::now(),
            processes: &[process(20, "raro", 1.0)],
            connections: &connections,
            config: &config,
        });
        let event = events
            .iter()
            .find(|event| event.kind == "unusual-outbound")
            .expect("debe detectar tráfico inusual");
        assert_eq!(event.unique_public_remotes, Some(4));
    }

    #[test]
    fn varias_instancias_del_mismo_nombre_no_son_una_reaparicion() {
        let mut tracker = AnomalyTracker::default();
        let config = config();

        // Dos ayudantes vivos a la vez con el mismo nombre y PIDs distintos:
        // el caso exacto que antes producía un falso positivo en la 1.ª captura.
        let processes = vec![process(10, "Helper", 1.0), process(11, "Helper", 1.0)];
        for _ in 0..4 {
            let events = tracker.analyze(DetectionInput {
                collected_at: Utc::now(),
                processes: &processes,
                connections: &[],
                config: &config,
            });
            assert!(events.iter().all(|event| event.kind != "fast-respawn"));
        }
    }

    #[test]
    fn un_proceso_unico_que_cambia_de_pid_si_es_una_reaparicion() {
        let mut tracker = AnomalyTracker::default();
        let config = config();

        let mut fired = false;
        for pid in [10, 11, 12] {
            let events = tracker.analyze(DetectionInput {
                collected_at: Utc::now(),
                processes: &[process(pid, "agente", 1.0)],
                connections: &[],
                config: &config,
            });
            fired |= events.iter().any(|event| event.kind == "fast-respawn");
        }
        assert!(
            fired,
            "un proceso de instancia única que cambia de PID debe reportarse"
        );
    }

    #[test]
    fn un_navegador_instalado_con_normalidad_no_dispara_trafico_inusual() {
        let mut tracker = AnomalyTracker::default();
        let config = config();
        let mut browser = process(21, "Google Chrome Helper", 1.0);
        browser.exe_path = "/Applications/Google Chrome.app/Contents/MacOS/Helper".to_owned();

        let connections: Vec<ConnectionInsight> = ["8.8.8.8", "1.1.1.1", "9.9.9.9", "4.4.4.4"]
            .iter()
            .map(|ip| ConnectionInsight {
                pid: 21,
                remote_address: format!("{ip}:443"),
                is_public_remote: true,
                ..Default::default()
            })
            .collect();

        let events = tracker.analyze(DetectionInput {
            collected_at: Utc::now(),
            processes: &[browser],
            connections: &connections,
            config: &config,
        });
        assert!(
            events.iter().all(|event| event.kind != "unusual-outbound"),
            "una app instalada en /Applications hablando con varios destinos es lo normal"
        );
    }

    #[test]
    fn un_binario_sin_firmar_en_applications_si_dispara_trafico_inusual() {
        let mut tracker = AnomalyTracker::default();
        let config = config();
        let mut suspicious = process(22, "helper", 1.0);
        suspicious.exe_path = "/Applications/Raro.app/Contents/MacOS/helper".to_owned();
        suspicious.signature = Some(CodeSignature::Unsigned);

        let connections: Vec<ConnectionInsight> = ["8.8.8.8", "1.1.1.1", "9.9.9.9", "4.4.4.4"]
            .iter()
            .map(|ip| ConnectionInsight {
                pid: 22,
                remote_address: format!("{ip}:443"),
                is_public_remote: true,
                ..Default::default()
            })
            .collect();

        let events = tracker.analyze(DetectionInput {
            collected_at: Utc::now(),
            processes: &[suspicious],
            connections: &connections,
            config: &config,
        });
        assert!(events.iter().any(|event| event.kind == "unusual-outbound"));
    }

    #[test]
    fn persistencia_nueva_y_sospechosa_escala_a_critica() {
        let entry = PersistenceEntry {
            entry_kind: "LaunchDaemon".to_owned(),
            name: "com.fake.helper".to_owned(),
            location: "/Library/LaunchDaemons/com.fake.helper.plist".to_owned(),
            command: "/tmp/.helper".to_owned(),
            scope: PersistenceScope::GlobalDaemon,
            severity: RiskLevel::High,
            change_status: PersistenceChange::Added,
            ..Default::default()
        };
        let event = persistence_change_event(Utc::now(), &entry).expect("debe generar evento");
        assert_eq!(event.severity, RiskLevel::Critical);
        assert_eq!(event.kind, "persistence-change");
    }

    #[test]
    fn persistencia_sin_cambios_no_genera_evento() {
        let entry = PersistenceEntry::default();
        assert!(persistence_change_event(Utc::now(), &entry).is_none());
    }

    #[test]
    fn destinos_privados_se_agrupan_sin_loopback() {
        let connections = vec![
            ConnectionInsight {
                pid: 5,
                remote_address: "192.168.1.10:22".to_owned(),
                ..Default::default()
            },
            ConnectionInsight {
                pid: 5,
                remote_address: "127.0.0.1:5432".to_owned(),
                ..Default::default()
            },
        ];
        let table = unique_private_remotes_by_pid(&connections);
        assert_eq!(table[&5], vec!["192.168.1.10".to_owned()]);
    }
}
