//! Interfaz gráfica (egui/eframe).
//!
//! ## Por qué hay un hilo de trabajo
//!
//! Una captura de RootCause invoca `lsof`, `spctl`, `csrutil`, `codesign` y
//! recorre carpetas de cachés. Eso tarda entre décimas de segundo y varios
//! segundos, según el equipo. Hacerlo en el hilo de la interfaz congelaría la
//! ventana en cada refresco.
//!
//! Por eso el motor ([`InspectorService`]) vive en un hilo propio que recibe
//! [`Command`] y devuelve [`Response`] por canales. La interfaz nunca bloquea:
//! pinta el último estado conocido, muestra que hay un trabajo en curso y
//! recoge el resultado cuando llega.
//!
//! ## Cómo está organizada la vista
//!
//! Doce secciones en una barra lateral, agrupadas por pregunta: qué pasa ahora
//! (Resumen, Procesos), quién habla con fuera (Conexiones, Red), qué sobrevive
//! a un reinicio (Persistencia), qué defensas hay (Seguridad, Privacidad), qué
//! ocupa espacio (Almacenamiento) y qué pasó antes (Historial).

use crate::config::{RootCauseConfig, ThemeMode};
use crate::i18n::{self, Lang};
use crate::meta;
use crate::models::{
    AuditRecord, CodeSignature, EventRecord, HardwareInfo, IncidentSummary, NetworkScan,
    PersistenceEntry, Severity, SnapshotRow, SystemSnapshot,
};
use crate::services::inspector::InspectorService;
use crate::services::macos::EnvironmentReport;
use eframe::egui::{
    self, Align, Color32, Context, Frame, Layout, Margin, Response, RichText, Rounding, Sense,
    Stroke, Ui, Vec2,
};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

// ── Paleta ──────────────────────────────────────────────────────────────────

/// Colores de la interfaz. Dos variantes de la misma identidad: el azul de
/// marca (#1f6feb) se mantiene en ambas para que la app se reconozca igual.
#[derive(Clone, Copy)]
struct Palette {
    bg: Color32,
    panel: Color32,
    card: Color32,
    border: Color32,
    text: Color32,
    dim: Color32,
    accent: Color32,
    ok: Color32,
    warn: Color32,
    crit: Color32,
}

const DARK: Palette = Palette {
    bg: Color32::from_rgb(13, 17, 23),
    panel: Color32::from_rgb(18, 23, 31),
    card: Color32::from_rgb(24, 30, 40),
    border: Color32::from_rgb(40, 48, 61),
    text: Color32::from_rgb(226, 232, 240),
    dim: Color32::from_rgb(139, 152, 170),
    accent: Color32::from_rgb(31, 111, 235),
    ok: Color32::from_rgb(63, 185, 80),
    warn: Color32::from_rgb(226, 168, 44),
    crit: Color32::from_rgb(233, 84, 84),
};

const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(246, 248, 251),
    panel: Color32::from_rgb(255, 255, 255),
    card: Color32::from_rgb(255, 255, 255),
    border: Color32::from_rgb(216, 222, 231),
    text: Color32::from_rgb(23, 30, 40),
    dim: Color32::from_rgb(101, 112, 128),
    accent: Color32::from_rgb(31, 111, 235),
    ok: Color32::from_rgb(35, 134, 54),
    warn: Color32::from_rgb(154, 103, 0),
    crit: Color32::from_rgb(191, 47, 47),
};

/// Paleta activa del frame. La GUI de egui corre en un solo hilo, así que un
/// estático mutable protegido por función es suficiente y evita cablear la
/// paleta por cada función de dibujo.
static mut ACTIVE_DARK: bool = true;

fn pal() -> Palette {
    // SAFETY: solo se lee y escribe desde el hilo de la interfaz.
    if unsafe { ACTIVE_DARK } {
        DARK
    } else {
        LIGHT
    }
}

fn set_dark(dark: bool) {
    unsafe { ACTIVE_DARK = dark };
}

/// `true` si macOS está en apariencia oscura.
fn system_prefers_dark() -> bool {
    crate::services::macos::run_capture("/usr/bin/defaults", &["read", "-g", "AppleInterfaceStyle"])
        .map(|value| value.trim().eq_ignore_ascii_case("Dark"))
        .unwrap_or(false)
}

// ── Hilo de trabajo ─────────────────────────────────────────────────────────

/// Peticiones que la interfaz envía al motor.
enum Command {
    Refresh,
    History(usize),
    Incidents(usize),
    Audits(usize),
    DeepNetwork,
    AcceptPersistence,
    AcceptSecurity,
    AcceptTcc,
    AcceptNetwork,
    CleanCaches { dry_run: bool },
    Report,
    Export,
    BackupHistory,
    Kill(u32),
    Reveal(String),
    BlockIp(String),
    SecurityEvents(u32),
    LoginItems,
    SaveConfig(Box<RootCauseConfig>),
    Shutdown,
}

/// Resultados que el motor devuelve a la interfaz.
///
/// Se llama `EngineEvent` y no `Response` para no chocar con `egui::Response`,
/// que es el tipo de retorno de todos los widgets de este módulo.
enum EngineEvent {
    Ready {
        hardware: Box<HardwareInfo>,
        config: Box<RootCauseConfig>,
        db_path: String,
        history_line: String,
        environment: Box<EnvironmentReport>,
    },
    Snapshot(Box<SystemSnapshot>),
    History(Vec<SnapshotRow>),
    Incidents(Vec<IncidentSummary>),
    Audits(Vec<AuditRecord>),
    Network(Box<NetworkScan>),
    Events(Vec<EventRecord>),
    LoginItems(Vec<PersistenceEntry>),
    Message(String),
    Failed(String),
}

/// Arranca el hilo que posee el motor y devuelve los extremos de los canales.
///
/// Si el motor no se puede inicializar, el hilo responde `Failed` y termina:
/// la interfaz sigue abierta y explica el problema en vez de cerrarse de golpe.
fn spawn_worker(ctx: Context) -> (Sender<Command>, Receiver<EngineEvent>) {
    let (command_tx, command_rx) = channel::<Command>();
    let (response_tx, response_rx) = channel::<EngineEvent>();

    std::thread::spawn(move || {
        let mut service = match InspectorService::new() {
            Ok(service) => service,
            Err(error) => {
                let _ = response_tx.send(EngineEvent::Failed(format!(
                    "No se pudo inicializar el motor: {error}"
                )));
                ctx.request_repaint();
                return;
            }
        };

        let _ = response_tx.send(EngineEvent::Ready {
            hardware: Box::new(service.get_hardware_info()),
            config: Box::new(service.config().clone()),
            db_path: service.db_path().display().to_string(),
            history_line: service.latest_history_line(),
            environment: Box::new(service.environment()),
        });
        ctx.request_repaint();

        while let Ok(command) = command_rx.recv() {
            let response = match command {
                Command::Shutdown => break,
                Command::Refresh => match service.collect_snapshot() {
                    Ok(snapshot) => {
                        service.notify_if_critical(&snapshot);
                        EngineEvent::Snapshot(Box::new(snapshot))
                    }
                    Err(error) => EngineEvent::Failed(format!("Captura fallida: {error}")),
                },
                Command::History(limit) => EngineEvent::History(service.load_history(limit)),
                Command::Incidents(limit) => EngineEvent::Incidents(service.load_incidents(limit)),
                Command::Audits(limit) => EngineEvent::Audits(service.load_audits(limit)),
                Command::DeepNetwork => EngineEvent::Network(Box::new(service.scan_network(true))),
                Command::AcceptPersistence => match service.accept_persistence_baseline() {
                    Ok(count) => EngineEvent::Message(format!(
                        "Baseline de persistencia aceptada ({count} entradas)"
                    )),
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
                Command::AcceptSecurity => match service.accept_security_baseline() {
                    Ok(count) => EngineEvent::Message(format!(
                        "Baseline de seguridad aceptada ({count} controles)"
                    )),
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
                Command::AcceptTcc => match service.accept_tcc_baseline() {
                    Ok(count) => EngineEvent::Message(format!(
                        "Baseline de privacidad aceptada ({count} permisos)"
                    )),
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
                Command::AcceptNetwork => match service.accept_network_baseline() {
                    Ok(count) => EngineEvent::Message(format!(
                        "Red conocida aceptada ({count} dispositivos)"
                    )),
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
                Command::CleanCaches { dry_run } => {
                    let result = service.clean_caches(dry_run);
                    EngineEvent::Message(if dry_run {
                        format!(
                            "Simulación: se liberarían {:.1} MB en {} entradas",
                            result.freed_mb, result.deleted_count
                        )
                    } else {
                        format!(
                            "Limpieza hecha: {:.1} MB liberados · {} en uso saltadas",
                            result.freed_mb, result.skipped_in_use
                        )
                    })
                }
                Command::Report => match service.collect_snapshot() {
                    Ok(snapshot) => match service.generate_report(&snapshot) {
                        Ok(path) => EngineEvent::Message(format!("Reporte generado en {path}")),
                        Err(error) => EngineEvent::Failed(error.to_string()),
                    },
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
                Command::BackupHistory => match service.export_history_backup() {
                    Ok(path) => EngineEvent::Message(format!("Copia del historial en {path}")),
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
                Command::Export => match service.collect_snapshot() {
                    Ok(snapshot) => match service.export_snapshot(&snapshot) {
                        Ok(path) => EngineEvent::Message(format!("Captura exportada en {path}")),
                        Err(error) => EngineEvent::Failed(error.to_string()),
                    },
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
                Command::Kill(pid) => match service.terminate_process(pid) {
                    Ok(message) => EngineEvent::Message(message),
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
                Command::Reveal(path) => match service.reveal_in_finder(&path) {
                    Ok(message) => EngineEvent::Message(message),
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
                Command::BlockIp(ip) => match service.suggest_block_ip(&ip) {
                    Ok(message) => EngineEvent::Message(message),
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
                Command::SecurityEvents(minutes) => {
                    EngineEvent::Events(service.security_events(minutes))
                }
                Command::LoginItems => EngineEvent::LoginItems(service.login_items()),
                Command::SaveConfig(config) => match service.save_config(&config) {
                    Ok(()) => EngineEvent::Message("Configuración guardada".to_owned()),
                    Err(error) => EngineEvent::Failed(error.to_string()),
                },
            };

            let _ = response_tx.send(response);
            ctx.request_repaint();
        }
    });

    (command_tx, response_rx)
}

// ── Secciones ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Overview,
    Processes,
    Connections,
    Network,
    Persistence,
    Security,
    Privacy,
    Storage,
    History,
    Config,
    Manual,
    About,
}

impl Tab {
    /// Secciones en el orden en que aparecen en la barra lateral.
    const ALL: [Tab; 12] = [
        Tab::Overview,
        Tab::Processes,
        Tab::Connections,
        Tab::Network,
        Tab::Persistence,
        Tab::Security,
        Tab::Privacy,
        Tab::Storage,
        Tab::History,
        Tab::Config,
        Tab::Manual,
        Tab::About,
    ];

    fn title(self) -> &'static str {
        match self {
            Tab::Overview => i18n::tr("Resumen", "Overview"),
            Tab::Processes => i18n::tr("Procesos", "Processes"),
            Tab::Connections => i18n::tr("Conexiones", "Connections"),
            Tab::Network => i18n::tr("Red", "Network"),
            Tab::Persistence => i18n::tr("Persistencia", "Persistence"),
            Tab::Security => i18n::tr("Seguridad", "Security"),
            Tab::Privacy => i18n::tr("Privacidad", "Privacy"),
            Tab::Storage => i18n::tr("Almacenamiento", "Storage"),
            Tab::History => i18n::tr("Historial", "History"),
            Tab::Config => i18n::tr("Configuración", "Settings"),
            Tab::Manual => i18n::tr("Manual", "Manual"),
            Tab::About => i18n::tr("Acerca", "About"),
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Tab::Overview => i18n::tr(
                "Veredicto del equipo y señales dominantes",
                "System verdict and dominant signals",
            ),
            Tab::Processes => i18n::tr(
                "Qué se está ejecutando y con qué firma",
                "What is running and how it is signed",
            ),
            Tab::Connections => i18n::tr(
                "Qué proceso habla con el exterior",
                "Which process talks to the outside",
            ),
            Tab::Network => i18n::tr(
                "Equipos cercanos del segmento local",
                "Nearby devices on the local segment",
            ),
            Tab::Persistence => i18n::tr(
                "LaunchAgents, LaunchDaemons y tareas que sobreviven al reinicio",
                "LaunchAgents, LaunchDaemons and tasks that survive a reboot",
            ),
            Tab::Security => i18n::tr(
                "Gatekeeper, SIP, FileVault, firewall y XProtect",
                "Gatekeeper, SIP, FileVault, firewall and XProtect",
            ),
            Tab::Privacy => i18n::tr(
                "Permisos TCC concedidos a las aplicaciones",
                "TCC permissions granted to applications",
            ),
            Tab::Storage => i18n::tr(
                "Cachés y temporales del sistema",
                "System caches and temp files",
            ),
            Tab::History => i18n::tr(
                "Capturas anteriores, incidentes y auditoría",
                "Previous snapshots, incidents and audit trail",
            ),
            Tab::Config => i18n::tr(
                "Apariencia, idioma y umbrales",
                "Appearance, language and thresholds",
            ),
            Tab::Manual => i18n::tr("Qué hace cada sección", "What each section does"),
            Tab::About => i18n::tr("Versión, licencia y equipo", "Version, license and machine"),
        }
    }

    /// Grupo bajo el que se agrupa en la barra lateral (`None` = sin encabezado).
    fn group(self) -> Option<&'static str> {
        match self {
            Tab::Overview => Some(i18n::tr("ESTADO", "STATUS")),
            Tab::Connections => Some(i18n::tr("RED", "NETWORK")),
            Tab::Persistence => Some(i18n::tr("SUPERFICIE DE ATAQUE", "ATTACK SURFACE")),
            Tab::Storage => Some(i18n::tr("MANTENIMIENTO", "MAINTENANCE")),
            Tab::Config => Some(i18n::tr("AJUSTES", "SETTINGS")),
            _ => None,
        }
    }
}

// ── Aplicación ──────────────────────────────────────────────────────────────

pub struct RootCauseApp {
    commands: Sender<Command>,
    responses: Receiver<EngineEvent>,

    tab: Tab,
    snapshot: Option<SystemSnapshot>,
    hardware: HardwareInfo,
    config: RootCauseConfig,
    db_path: String,
    environment: Option<EnvironmentReport>,

    history: Vec<SnapshotRow>,
    incidents: Vec<IncidentSummary>,
    audits: Vec<AuditRecord>,
    deep_network: Option<NetworkScan>,
    events: Vec<EventRecord>,
    login_items: Vec<PersistenceEntry>,

    cpu_series: Vec<f32>,
    memory_series: Vec<f32>,
    write_series: Vec<f32>,

    status: String,
    status_is_error: bool,
    busy: bool,
    engine_ready: bool,
    last_refresh: Instant,

    process_filter: String,
    severity_filter: Option<Severity>,
    persistence_only_changes: bool,
    tcc_only_sensitive: bool,
    /// Confirmación en dos pasos para la limpieza de cachés.
    clean_armed: bool,
    events_minutes: u32,
}

impl RootCauseApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (commands, responses) = spawn_worker(cc.egui_ctx.clone());
        configure_style(&cc.egui_ctx);

        // La primera captura se pide de inmediato: abrir la app y ver una
        // pantalla vacía sería un mal primer segundo.
        let _ = commands.send(Command::Refresh);
        let _ = commands.send(Command::History(60));

        Self {
            commands,
            responses,
            tab: Tab::Overview,
            snapshot: None,
            hardware: HardwareInfo::default(),
            config: RootCauseConfig::default(),
            db_path: String::new(),
            environment: None,
            history: Vec::new(),
            incidents: Vec::new(),
            audits: Vec::new(),
            deep_network: None,
            events: Vec::new(),
            login_items: Vec::new(),
            cpu_series: Vec::new(),
            memory_series: Vec::new(),
            write_series: Vec::new(),
            status: "Iniciando el motor de inspección…".to_owned(),
            status_is_error: false,
            busy: true,
            engine_ready: false,
            last_refresh: Instant::now(),
            process_filter: String::new(),
            severity_filter: None,
            persistence_only_changes: false,
            tcc_only_sensitive: true,
            clean_armed: false,
            events_minutes: 30,
        }
    }

    fn send(&mut self, command: Command) {
        self.busy = true;
        if self.commands.send(command).is_err() {
            self.busy = false;
            self.status = "El motor de inspección no está disponible.".to_owned();
            self.status_is_error = true;
        }
    }

    /// Vacía la cola de respuestas del motor.
    fn drain_responses(&mut self) {
        while let Ok(response) = self.responses.try_recv() {
            self.busy = false;
            match response {
                EngineEvent::Ready {
                    hardware,
                    config,
                    db_path,
                    history_line,
                    environment,
                } => {
                    self.hardware = *hardware;
                    i18n::set_lang(config.ui.language);
                    self.config = *config;
                    self.db_path = db_path;
                    self.environment = Some(*environment);
                    self.engine_ready = true;
                    self.status = history_line;
                    self.status_is_error = false;
                }
                EngineEvent::Snapshot(snapshot) => {
                    push_sample(&mut self.cpu_series, snapshot.overview.cpu_usage_percent);
                    push_sample(
                        &mut self.memory_series,
                        percent(
                            snapshot.overview.memory_used_gb,
                            snapshot.overview.memory_total_gb,
                        ),
                    );
                    push_sample(&mut self.write_series, snapshot.overview.io_write_mb_delta);

                    self.status = format!(
                        "Captura {} · {} alertas",
                        snapshot.collected_at.format("%H:%M:%S"),
                        snapshot.alerts.len()
                    );
                    self.status_is_error = false;
                    self.snapshot = Some(*snapshot);
                    self.last_refresh = Instant::now();
                }
                EngineEvent::History(rows) => self.history = rows,
                EngineEvent::Incidents(items) => self.incidents = items,
                EngineEvent::Audits(items) => self.audits = items,
                EngineEvent::Network(scan) => {
                    self.status = format!(
                        "Escaneo profundo: {} equipos ({} nuevos)",
                        scan.total_devices, scan.new_devices
                    );
                    self.status_is_error = false;
                    self.deep_network = Some(*scan);
                }
                EngineEvent::Events(events) => {
                    self.status = format!("{} eventos de seguridad recuperados", events.len());
                    self.status_is_error = false;
                    self.events = events;
                }
                EngineEvent::LoginItems(items) => {
                    self.status = format!("{} login items consultados", items.len());
                    self.status_is_error = false;
                    self.login_items = items;
                }
                EngineEvent::Message(message) => {
                    self.status = message;
                    self.status_is_error = false;
                }
                EngineEvent::Failed(message) => {
                    self.status = message;
                    self.status_is_error = true;
                }
            }
        }
    }

    /// Refresco automático según el intervalo configurado.
    fn maybe_auto_refresh(&mut self) {
        if self.busy || !self.engine_ready {
            return;
        }
        let interval = Duration::from_secs(self.config.collection.refresh_interval_secs.max(2));
        if self.last_refresh.elapsed() >= interval {
            self.last_refresh = Instant::now();
            self.send(Command::Refresh);
        }
    }

    fn apply_theme(&self, ctx: &Context) {
        let dark = match self.config.ui.theme {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => system_prefers_dark(),
        };
        set_dark(dark);

        let palette = pal();
        let mut visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = palette.bg;
        visuals.window_fill = palette.panel;
        visuals.extreme_bg_color = palette.card;
        visuals.override_text_color = Some(palette.text);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
        visuals.widgets.inactive.bg_fill = palette.card;
        visuals.widgets.hovered.bg_fill = palette.border;
        visuals.widgets.active.bg_fill = palette.accent;
        visuals.selection.bg_fill = palette.accent.gamma_multiply(0.45);
        ctx.set_visuals(visuals);
    }
}

impl eframe::App for RootCauseApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.drain_responses();
        self.apply_theme(ctx);
        self.maybe_auto_refresh();

        // Atajos: F5 refresca, ⌘E exporta, ⌘R genera reporte.
        let (refresh, export, report) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::F5),
                input.modifiers.command && input.key_pressed(egui::Key::E),
                input.modifiers.command && input.key_pressed(egui::Key::R),
            )
        });
        if refresh {
            self.send(Command::Refresh);
        }
        if export {
            self.send(Command::Export);
        }
        if report {
            self.send(Command::Report);
        }

        draw_sidebar(self, ctx);
        draw_topbar(self, ctx);
        draw_statusbar(self, ctx);

        egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(pal().bg)
                    .inner_margin(Margin::same(18.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| match self.tab {
                        Tab::Overview => draw_overview(self, ui),
                        Tab::Processes => draw_processes(self, ui),
                        Tab::Connections => draw_connections(self, ui),
                        Tab::Network => draw_network(self, ui),
                        Tab::Persistence => draw_persistence(self, ui),
                        Tab::Security => draw_security(self, ui),
                        Tab::Privacy => draw_privacy(self, ui),
                        Tab::Storage => draw_storage(self, ui),
                        Tab::History => draw_history(self, ui),
                        Tab::Config => draw_config(self, ui),
                        Tab::Manual => draw_manual(ui),
                        Tab::About => draw_about(self, ui),
                    });
            });

        // Repintado suave mientras hay trabajo o para animar el reloj del estado.
        ctx.request_repaint_after(Duration::from_millis(if self.busy { 120 } else { 900 }));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.commands.send(Command::Shutdown);
    }
}

// ── Estructura de la ventana ────────────────────────────────────────────────

fn draw_sidebar(app: &mut RootCauseApp, ctx: &Context) {
    let palette = pal();
    egui::SidePanel::left("nav")
        .exact_width(228.0)
        .resizable(false)
        .frame(
            Frame::none()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(12.0, 16.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                draw_logo(ui, 26.0);
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("RootCause").size(16.0).strong());
                    ui.label(
                        RichText::new(format!("macOS Inspector v{}", meta::VERSION))
                            .size(10.0)
                            .color(palette.dim),
                    );
                });
            });
            ui.add_space(14.0);

            for tab in Tab::ALL {
                if let Some(group) = tab.group() {
                    ui.add_space(10.0);
                    ui.label(RichText::new(group).size(9.5).color(palette.dim).strong());
                    ui.add_space(4.0);
                }
                if sidebar_item(ui, tab.title(), app.tab == tab, tab_badge(app, tab)).clicked() {
                    app.tab = tab;
                }
            }

            ui.add_space(12.0);
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.label(
                    RichText::new(i18n::tr("Análisis 100 % local", "100% local analysis"))
                        .size(10.0)
                        .color(palette.dim),
                );
                ui.add_space(6.0);
            });
        });
}

/// Contador que se pinta a la derecha de una sección cuando hay algo que ver.
fn tab_badge(app: &RootCauseApp, tab: Tab) -> Option<(usize, Severity)> {
    let snapshot = app.snapshot.as_ref()?;
    let badge = match tab {
        Tab::Overview => {
            let count = snapshot.alerts.len();
            let severity = snapshot.overview.primary_severity;
            (count, severity)
        }
        Tab::Persistence => {
            let count = snapshot
                .persistence_entries
                .iter()
                .filter(|entry| entry.change_status.is_change())
                .count();
            (count, Severity::Critical)
        }
        Tab::Security => {
            let count = snapshot
                .security_controls
                .iter()
                .filter(|control| control.severity >= Severity::Warning)
                .count();
            (count, Severity::Warning)
        }
        Tab::Privacy => (snapshot.tcc.sensitive_count, Severity::Warning),
        Tab::Processes => (
            snapshot
                .processes
                .iter()
                .filter(|process| process.severity >= Severity::Warning)
                .count(),
            Severity::Warning,
        ),
        _ => (0, Severity::Healthy),
    };

    (badge.0 > 0).then_some(badge)
}

fn sidebar_item(
    ui: &mut Ui,
    label: &str,
    active: bool,
    badge: Option<(usize, Severity)>,
) -> Response {
    let palette = pal();
    let height = 30.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());

    let background = if active {
        palette.accent.gamma_multiply(0.22)
    } else if response.hovered() {
        palette.card
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, Rounding::same(7.0), background);

    if active {
        let bar = egui::Rect::from_min_size(
            rect.left_top() + Vec2::new(0.0, 6.0),
            Vec2::new(3.0, height - 12.0),
        );
        ui.painter()
            .rect_filled(bar, Rounding::same(2.0), palette.accent);
    }

    let text_color = if active { palette.text } else { palette.dim };
    ui.painter().text(
        rect.left_center() + Vec2::new(14.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        text_color,
    );

    if let Some((count, severity)) = badge {
        let color = severity_color(severity);
        let center = rect.right_center() - Vec2::new(16.0, 0.0);
        ui.painter()
            .circle_filled(center, 9.0, color.gamma_multiply(0.25));
        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            count.to_string(),
            egui::FontId::proportional(10.0),
            color,
        );
    }

    response
}

fn draw_topbar(app: &mut RootCauseApp, ctx: &Context) {
    let palette = pal();
    egui::TopBottomPanel::top("topbar")
        .exact_height(64.0)
        .frame(
            Frame::none()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(18.0, 10.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(app.tab.title()).size(18.0).strong());
                    ui.label(
                        RichText::new(app.tab.subtitle())
                            .size(11.0)
                            .color(palette.dim),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if action_button(ui, i18n::tr("Actualizar", "Refresh"), palette.accent)
                        .clicked()
                    {
                        app.send(Command::Refresh);
                    }
                    if action_button(ui, i18n::tr("Reporte", "Report"), palette.card).clicked() {
                        app.send(Command::Report);
                    }
                    if action_button(ui, i18n::tr("Exportar", "Export"), palette.card).clicked() {
                        app.send(Command::Export);
                    }
                    if app.busy {
                        ui.add_space(8.0);
                        ui.add(egui::Spinner::new().size(14.0));
                    }
                });
            });
        });
}

fn draw_statusbar(app: &RootCauseApp, ctx: &Context) {
    let palette = pal();
    egui::TopBottomPanel::bottom("statusbar")
        .exact_height(28.0)
        .frame(
            Frame::none()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(18.0, 6.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let color = if app.status_is_error {
                    palette.crit
                } else {
                    palette.dim
                };
                ui.label(RichText::new(&app.status).size(11.0).color(color));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !app.db_path.is_empty() {
                        ui.label(
                            RichText::new(format!("SQLite: {}", app.db_path))
                                .size(10.0)
                                .color(palette.dim),
                        );
                    }
                });
            });
        });
}

// ── Secciones ───────────────────────────────────────────────────────────────

fn draw_overview(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();
    let Some(snapshot) = app.snapshot.clone() else {
        loading(ui);
        return;
    };

    // Banner de veredicto.
    let severity = snapshot.overview.primary_severity;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            severity_dot(ui, severity, 16.0);
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(verdict_title(severity))
                        .size(19.0)
                        .strong()
                        .color(severity_color(severity)),
                );
                ui.label(
                    RichText::new(&snapshot.overview.primary_reason)
                        .size(12.5)
                        .color(palette.text),
                );
            });
        });
    });

    ui.add_space(12.0);

    // Métricas principales.
    ui.horizontal_wrapped(|ui| {
        metric_card(
            ui,
            i18n::tr("CPU", "CPU"),
            &format!("{:.1}%", snapshot.overview.cpu_usage_percent),
            &app.cpu_series,
            palette.accent,
        );
        metric_card(
            ui,
            i18n::tr("Memoria", "Memory"),
            &format!(
                "{:.1} / {:.1} GB",
                snapshot.overview.memory_used_gb, snapshot.overview.memory_total_gb
            ),
            &app.memory_series,
            palette.ok,
        );
        metric_card(
            ui,
            i18n::tr("Escritura", "Disk write"),
            &format!("{:.1} MB", snapshot.overview.io_write_mb_delta),
            &app.write_series,
            palette.warn,
        );
    });

    ui.add_space(14.0);

    // Estado de superficie en una línea.
    ui.horizontal_wrapped(|ui| {
        let controls_off = snapshot
            .security_controls
            .iter()
            .filter(|control| control.severity >= Severity::Warning)
            .count();
        summary_pill(
            ui,
            i18n::tr("Controles con aviso", "Controls flagged"),
            &controls_off.to_string(),
            if controls_off == 0 {
                Severity::Healthy
            } else {
                Severity::Warning
            },
        );
        let persistence_changes = snapshot
            .persistence_entries
            .iter()
            .filter(|entry| entry.change_status.is_change())
            .count();
        summary_pill(
            ui,
            i18n::tr("Cambios de persistencia", "Persistence changes"),
            &persistence_changes.to_string(),
            if persistence_changes == 0 {
                Severity::Healthy
            } else {
                Severity::Critical
            },
        );
        summary_pill(
            ui,
            i18n::tr("Permisos sensibles", "Sensitive permissions"),
            &snapshot.tcc.sensitive_count.to_string(),
            if snapshot.tcc.sensitive_count == 0 {
                Severity::Healthy
            } else {
                Severity::Warning
            },
        );
        summary_pill(
            ui,
            i18n::tr("XProtect", "XProtect"),
            &format!("{} d", snapshot.xprotect.freshest_age_days.max(0)),
            snapshot.xprotect.severity,
        );
    });

    ui.add_space(16.0);
    section(
        ui,
        i18n::tr("Alertas de esta captura", "Alerts in this snapshot"),
    );

    if snapshot.alerts.is_empty() {
        empty(
            ui,
            i18n::tr(
                "Sin alertas: no se detectaron señales relevantes.",
                "No alerts: no relevant signals detected.",
            ),
        );
    } else {
        for alert in &snapshot.alerts {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    severity_dot(ui, alert.severity, 9.0);
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&alert.title).size(13.0).strong());
                        if !alert.detail.is_empty() {
                            ui.label(RichText::new(&alert.detail).size(11.5).color(palette.dim));
                        }
                        if !alert.hint.is_empty() {
                            ui.label(
                                RichText::new(format!("→ {}", alert.hint))
                                    .size(11.0)
                                    .color(palette.accent),
                            );
                        }
                    });
                });
            });
            ui.add_space(6.0);
        }
    }

    if let Some(incident) = snapshot.incident.as_ref() {
        ui.add_space(12.0);
        section(ui, i18n::tr("Incidente dominante", "Dominant incident"));
        card(ui, |ui| {
            ui.label(RichText::new(&incident.title).size(14.0).strong());
            ui.label(
                RichText::new(&incident.summary)
                    .size(12.0)
                    .color(palette.dim),
            );
            if !incident.root_cause_hypothesis.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "{}: {}",
                        i18n::tr("Hipótesis", "Hypothesis"),
                        incident.root_cause_hypothesis
                    ))
                    .size(11.5),
                );
            }
            for action in &incident.recommended_actions {
                ui.label(
                    RichText::new(format!("→ {action}"))
                        .size(11.5)
                        .color(palette.accent),
                );
            }
        });
    }
}

fn draw_processes(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();
    let Some(snapshot) = app.snapshot.clone() else {
        loading(ui);
        return;
    };

    ui.horizontal(|ui| {
        ui.label(i18n::tr("Filtrar:", "Filter:"));
        ui.add(egui::TextEdit::singleline(&mut app.process_filter).desired_width(220.0));
        ui.add_space(12.0);
        for (label, value) in [
            (i18n::tr("Todos", "All"), None),
            (i18n::tr("Aviso+", "Warning+"), Some(Severity::Warning)),
            (i18n::tr("Críticos", "Critical"), Some(Severity::Critical)),
        ] {
            if ui
                .selectable_label(app.severity_filter == value, label)
                .clicked()
            {
                app.severity_filter = value;
            }
        }
    });
    ui.add_space(10.0);

    let filter = app.process_filter.to_ascii_lowercase();
    let rows: Vec<_> = snapshot
        .processes
        .iter()
        .filter(|process| {
            app.severity_filter
                .map(|minimum| process.severity >= minimum)
                .unwrap_or(true)
                && (filter.is_empty()
                    || process.name.to_ascii_lowercase().contains(&filter)
                    || process.exe_path.to_ascii_lowercase().contains(&filter))
        })
        .take(120)
        .collect();

    if rows.is_empty() {
        empty(
            ui,
            i18n::tr("Ningún proceso coincide.", "No process matches."),
        );
        return;
    }

    table_header(
        ui,
        &[
            ("", 18.0),
            ("PROCESO", 200.0),
            ("PID", 60.0),
            ("CPU", 60.0),
            ("RAM", 80.0),
            ("FIRMA", 110.0),
            ("CATEGORÍA", 150.0),
        ],
    );

    let mut kill_target = None;
    for process in rows {
        card(ui, |ui| {
            ui.horizontal(|ui| {
                severity_dot(ui, process.severity, 8.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&process.name).size(12.5).strong());
                        ui.label(
                            RichText::new(format!("#{}", process.pid))
                                .size(11.0)
                                .color(palette.dim),
                        );
                        ui.label(
                            RichText::new(format!("{:.1}% CPU", process.cpu_percent)).size(11.0),
                        );
                        ui.label(RichText::new(format!("{:.0} MB", process.memory_mb)).size(11.0));
                        if let Some(signature) = process.signature {
                            signature_pill(ui, signature);
                        }
                        ui.label(
                            RichText::new(&process.category)
                                .size(10.5)
                                .color(palette.dim),
                        );
                    });
                    if !process.exe_path.is_empty() {
                        ui.label(
                            RichText::new(&process.exe_path)
                                .size(10.5)
                                .color(palette.dim),
                        );
                    }
                    ui.label(
                        RichText::new(process.reasons.join(" · "))
                            .size(10.5)
                            .color(severity_color(process.severity)),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if process.can_terminate
                        && ui
                            .add(egui::Button::new(
                                RichText::new(i18n::tr("Finalizar", "Terminate")).size(11.0),
                            ))
                            .clicked()
                    {
                        kill_target = Some(process.pid);
                    }
                });
            });
        });
        ui.add_space(5.0);
    }

    if let Some(pid) = kill_target {
        app.send(Command::Kill(pid));
    }
}

fn draw_connections(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();
    let Some(snapshot) = app.snapshot.clone() else {
        loading(ui);
        return;
    };

    note(
        ui,
        i18n::tr(
            "Sin privilegios de administrador, lsof solo muestra los sockets de tu usuario.",
            "Without administrator privileges, lsof only shows your own user's sockets.",
        ),
    );
    ui.add_space(10.0);

    if snapshot.connections.is_empty() {
        empty(
            ui,
            i18n::tr("Sin conexiones observadas.", "No connections observed."),
        );
        return;
    }

    let mut block_target = None;
    for connection in snapshot.connections.iter().take(120) {
        card(ui, |ui| {
            ui.horizontal(|ui| {
                severity_dot(ui, connection.severity, 8.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&connection.process_name).size(12.5).strong());
                        ui.label(
                            RichText::new(format!("#{}", connection.pid))
                                .size(11.0)
                                .color(palette.dim),
                        );
                        ui.label(
                            RichText::new(&connection.protocol)
                                .size(11.0)
                                .color(palette.dim),
                        );
                        if connection.is_listening {
                            pill(ui, i18n::tr("ESCUCHA", "LISTEN"), palette.warn);
                        }
                        if connection.is_public_remote {
                            pill(ui, i18n::tr("IP PÚBLICA", "PUBLIC IP"), palette.accent);
                        }
                    });
                    ui.label(
                        RichText::new(format!(
                            "{} → {} {}",
                            connection.local_address,
                            if connection.remote_address.is_empty() {
                                "—"
                            } else {
                                &connection.remote_address
                            },
                            connection.state
                        ))
                        .size(11.0)
                        .color(palette.dim),
                    );
                    ui.label(
                        RichText::new(&connection.reason)
                            .size(10.5)
                            .color(palette.dim),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if connection.is_public_remote
                        && ui
                            .add(egui::Button::new(
                                RichText::new(i18n::tr("Regla de bloqueo", "Block rule"))
                                    .size(11.0),
                            ))
                            .clicked()
                    {
                        block_target = Some(connection.remote_address.clone());
                    }
                });
            });
        });
        ui.add_space(5.0);
    }

    if let Some(ip) = block_target {
        app.send(Command::BlockIp(ip));
    }
}

fn draw_network(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();
    let scan = app.deep_network.clone().or_else(|| {
        app.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.network.clone())
    });
    let Some(scan) = scan else {
        loading(ui);
        return;
    };

    card(ui, |ui| {
        ui.horizontal(|ui| {
            fact(ui, i18n::tr("Interfaz", "Interface"), &scan.adapter_name);
            fact(ui, "IP", &scan.local_ip);
            fact(
                ui,
                i18n::tr("Puerta de enlace", "Gateway"),
                &scan.gateway_ip,
            );
            fact(
                ui,
                i18n::tr("Equipos", "Devices"),
                &scan.total_devices.to_string(),
            );
            fact(ui, i18n::tr("Nuevos", "New"), &scan.new_devices.to_string());
        });
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if action_button(
            ui,
            i18n::tr("Escaneo profundo", "Deep scan"),
            palette.accent,
        )
        .clicked()
        {
            app.send(Command::DeepNetwork);
        }
        if action_button(
            ui,
            i18n::tr("Aceptar red conocida", "Accept known network"),
            palette.card,
        )
        .clicked()
        {
            app.send(Command::AcceptNetwork);
        }
    });
    note(
        ui,
        i18n::tr(
            "El escaneo profundo hace ping a todo el segmento: es ruidoso y tarda; úsalo solo cuando lo necesites.",
            "A deep scan pings the whole segment: it is noisy and slow, use it only when needed.",
        ),
    );

    ui.add_space(12.0);
    for device in &scan.devices {
        card(ui, |ui| {
            ui.horizontal(|ui| {
                severity_dot(ui, device.severity, 8.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&device.ip).size(12.5).strong());
                        ui.label(RichText::new(&device.mac).size(11.0).color(palette.dim));
                        if !device.vendor.is_empty() {
                            pill(ui, &device.vendor, palette.accent);
                        }
                        if device.is_gateway {
                            pill(ui, i18n::tr("ROUTER", "GATEWAY"), palette.warn);
                        }
                        if device.is_self {
                            pill(ui, i18n::tr("ESTE MAC", "THIS MAC"), palette.ok);
                        }
                        if device.change_status.is_change() {
                            pill(ui, device.change_status.label(), palette.crit);
                        }
                    });
                    ui.label(RichText::new(&device.reason).size(10.5).color(palette.dim));
                });
            });
        });
        ui.add_space(5.0);
    }

    limitations(ui, &scan.limitations);
}

fn draw_persistence(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();
    let Some(snapshot) = app.snapshot.clone() else {
        loading(ui);
        return;
    };

    ui.horizontal(|ui| {
        if action_button(
            ui,
            i18n::tr("Aceptar baseline", "Accept baseline"),
            palette.accent,
        )
        .clicked()
        {
            app.send(Command::AcceptPersistence);
        }
        if action_button(
            ui,
            i18n::tr("Consultar login items", "Query login items"),
            palette.card,
        )
        .clicked()
        {
            app.send(Command::LoginItems);
        }
        ui.checkbox(
            &mut app.persistence_only_changes,
            i18n::tr("Solo cambios", "Changes only"),
        );
    });
    note(
        ui,
        i18n::tr(
            "Consultar login items pide el permiso de Automatización de macOS. Aceptar la baseline marca el estado actual como legítimo.",
            "Querying login items requests macOS Automation permission. Accepting the baseline marks the current state as legitimate.",
        ),
    );
    ui.add_space(10.0);

    let mut entries: Vec<PersistenceEntry> = snapshot.persistence_entries.clone();
    entries.extend(app.login_items.clone());
    let rows: Vec<_> = entries
        .iter()
        .filter(|entry| !app.persistence_only_changes || entry.change_status.is_change())
        .take(200)
        .collect();

    if rows.is_empty() {
        empty(
            ui,
            i18n::tr("Sin entradas que mostrar.", "No entries to show."),
        );
        return;
    }

    let mut reveal_target = None;
    for entry in rows {
        card(ui, |ui| {
            ui.horizontal(|ui| {
                severity_dot(ui, entry.severity.to_severity(), 8.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&entry.name).size(12.5).strong());
                        pill(ui, entry.scope.label(), palette.accent);
                        if entry.change_status.is_change() {
                            pill(ui, entry.change_status.label(), palette.crit);
                        }
                        if entry.keep_alive {
                            pill(ui, "KeepAlive", palette.warn);
                        }
                        if let Some(signature) = entry.signature {
                            signature_pill(ui, signature);
                        }
                    });
                    if !entry.command.is_empty() {
                        ui.label(RichText::new(&entry.command).size(11.0).color(palette.text));
                    }
                    ui.label(RichText::new(&entry.location).size(10.5).color(palette.dim));
                    ui.label(
                        RichText::new(&entry.note)
                            .size(10.5)
                            .color(severity_color(entry.severity.to_severity())),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if entry.location.starts_with('/')
                        && ui
                            .add(egui::Button::new(
                                RichText::new(i18n::tr("Revelar", "Reveal")).size(11.0),
                            ))
                            .clicked()
                    {
                        reveal_target = Some(entry.location.clone());
                    }
                });
            });
        });
        ui.add_space(5.0);
    }

    if let Some(path) = reveal_target {
        app.send(Command::Reveal(path));
    }
}

fn draw_security(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();
    let Some(snapshot) = app.snapshot.clone() else {
        loading(ui);
        return;
    };

    ui.horizontal(|ui| {
        if action_button(
            ui,
            i18n::tr("Aceptar baseline", "Accept baseline"),
            palette.accent,
        )
        .clicked()
        {
            app.send(Command::AcceptSecurity);
        }
        if action_button(
            ui,
            i18n::tr("Ver eventos recientes", "Recent events"),
            palette.card,
        )
        .clicked()
        {
            let minutes = app.events_minutes;
            app.send(Command::SecurityEvents(minutes));
        }
        ui.add(
            egui::DragValue::new(&mut app.events_minutes)
                .range(5..=720)
                .suffix(" min"),
        );
    });
    ui.add_space(12.0);

    section(ui, i18n::tr("Controles nativos", "Native controls"));
    for control in &snapshot.security_controls {
        card(ui, |ui| {
            ui.horizontal(|ui| {
                severity_dot(ui, control.severity, 8.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&control.name).size(12.5).strong());
                        pill(ui, &control.status, severity_color(control.severity));
                        if control.change_status.is_change() {
                            pill(ui, control.change_status.label(), palette.crit);
                        }
                    });
                    ui.label(
                        RichText::new(&control.explanation)
                            .size(11.0)
                            .color(palette.dim),
                    );
                    ui.label(
                        RichText::new(format!("$ {}", control.evidence))
                            .size(10.0)
                            .color(palette.dim)
                            .monospace(),
                    );
                });
            });
        });
        ui.add_space(5.0);
    }

    ui.add_space(12.0);
    section(
        ui,
        i18n::tr(
            "Definiciones antimalware de Apple",
            "Apple malware definitions",
        ),
    );
    card(ui, |ui| {
        ui.horizontal(|ui| {
            severity_dot(ui, snapshot.xprotect.severity, 8.0);
            ui.add_space(6.0);
            ui.label(RichText::new(&snapshot.xprotect.headline).size(12.0));
        });
        ui.add_space(6.0);
        for definition in &snapshot.xprotect.definitions {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&definition.component).size(11.5).strong());
                ui.label(RichText::new(format!("v{}", definition.version)).size(11.0));
                ui.label(
                    RichText::new(&definition.note)
                        .size(11.0)
                        .color(severity_color(definition.severity)),
                );
            });
        }
    });

    if !app.events.is_empty() {
        ui.add_space(12.0);
        section(
            ui,
            i18n::tr("Eventos del log unificado", "Unified log events"),
        );
        for event in app.events.iter().take(60) {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&event.timestamp)
                        .size(10.5)
                        .color(palette.dim)
                        .monospace(),
                );
                ui.label(RichText::new(&event.provider).size(10.5).strong());
                ui.label(
                    RichText::new(truncate(&event.message, 130))
                        .size(10.5)
                        .color(palette.dim),
                );
            });
        }
    }

    limitations(ui, &snapshot.xprotect.limitations);
}

fn draw_privacy(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();
    let Some(snapshot) = app.snapshot.clone() else {
        loading(ui);
        return;
    };

    card(ui, |ui| {
        ui.horizontal(|ui| {
            severity_dot(
                ui,
                if snapshot.tcc.readable { Severity::Healthy } else { Severity::Warning },
                10.0,
            );
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&snapshot.tcc.headline).size(13.0).strong());
                if !snapshot.tcc.full_disk_access {
                    ui.label(
                        RichText::new(i18n::tr(
                            "RootCause necesita Acceso total al disco para leer TCC.db: Ajustes del Sistema → Privacidad y seguridad → Acceso total al disco.",
                            "RootCause needs Full Disk Access to read TCC.db: System Settings → Privacy & Security → Full Disk Access.",
                        ))
                        .size(11.5)
                        .color(palette.warn),
                    );
                }
            });
        });
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if action_button(
            ui,
            i18n::tr("Aceptar baseline", "Accept baseline"),
            palette.accent,
        )
        .clicked()
        {
            app.send(Command::AcceptTcc);
        }
        ui.checkbox(
            &mut app.tcc_only_sensitive,
            i18n::tr("Solo permisos sensibles", "Sensitive permissions only"),
        );
    });
    ui.add_space(10.0);

    let rows: Vec<_> = snapshot
        .tcc
        .permissions
        .iter()
        .filter(|permission| {
            permission.allowed
                && (!app.tcc_only_sensitive
                    || crate::services::tcc::is_sensitive(&permission.service))
        })
        .take(200)
        .collect();

    if rows.is_empty() {
        empty(
            ui,
            i18n::tr("Sin permisos que mostrar.", "No permissions to show."),
        );
    } else {
        for permission in rows {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    severity_dot(ui, permission.severity, 8.0);
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&permission.service_label).size(12.5).strong());
                            pill(
                                ui,
                                &permission.decision,
                                severity_color(permission.severity),
                            );
                            pill(ui, &permission.database, palette.accent);
                            if permission.change_status.is_change() {
                                pill(ui, permission.change_status.label(), palette.crit);
                            }
                        });
                        ui.label(
                            RichText::new(&permission.client)
                                .size(11.0)
                                .color(palette.text),
                        );
                        if !permission.note.is_empty() {
                            ui.label(
                                RichText::new(&permission.note)
                                    .size(10.5)
                                    .color(palette.dim),
                            );
                        }
                    });
                });
            });
            ui.add_space(5.0);
        }
    }

    limitations(ui, &snapshot.tcc.limitations);
}

fn draw_storage(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();
    let Some(snapshot) = app.snapshot.clone() else {
        loading(ui);
        return;
    };

    card(ui, |ui| {
        ui.label(
            RichText::new(format!(
                "{}: {:.0} MB",
                i18n::tr(
                    "Total medido en cachés y temporales",
                    "Total measured in caches and temp"
                ),
                snapshot.caches.total_mb
            ))
            .size(14.0)
            .strong(),
        );
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if action_button(
            ui,
            i18n::tr("Simular limpieza", "Simulate cleanup"),
            palette.card,
        )
        .clicked()
        {
            app.clean_armed = true;
            app.send(Command::CleanCaches { dry_run: true });
        }
        if app.clean_armed
            && action_button(
                ui,
                i18n::tr("Limpiar de verdad", "Clean for real"),
                palette.crit,
            )
            .clicked()
        {
            app.clean_armed = false;
            app.send(Command::CleanCaches { dry_run: false });
        }
    });
    note(
        ui,
        i18n::tr(
            "La limpieza solo toca ~/Library/Caches, solo lo no usado en 24 h, y salta lo que esté en uso. Se pide dos veces a propósito.",
            "Cleanup only touches ~/Library/Caches, only items untouched for 24 h, and skips anything in use. It asks twice on purpose.",
        ),
    );

    ui.add_space(12.0);
    for entry in &snapshot.caches.top_entries {
        card(ui, |ui| {
            ui.horizontal(|ui| {
                severity_dot(ui, entry.severity, 8.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{:.0} MB", entry.size_mb))
                                .size(12.5)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                entry.file_count,
                                i18n::tr("archivos", "files")
                            ))
                            .size(11.0)
                            .color(palette.dim),
                        );
                        if entry.safe_to_clean {
                            pill(ui, i18n::tr("SEGURO", "SAFE"), palette.ok);
                        }
                    });
                    ui.label(RichText::new(&entry.path).size(11.0).color(palette.text));
                    ui.label(RichText::new(&entry.note).size(10.5).color(palette.dim));
                });
            });
        });
        ui.add_space(5.0);
    }

    limitations(ui, &snapshot.caches.limitations);
}

fn draw_history(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();

    ui.horizontal(|ui| {
        if action_button(
            ui,
            i18n::tr("Recargar historial", "Reload history"),
            palette.accent,
        )
        .clicked()
        {
            app.send(Command::History(60));
        }
        if action_button(
            ui,
            i18n::tr("Ver incidentes", "View incidents"),
            palette.card,
        )
        .clicked()
        {
            app.send(Command::Incidents(20));
        }
        if action_button(ui, i18n::tr("Ver auditoría", "View audit"), palette.card).clicked() {
            app.send(Command::Audits(40));
        }
        if action_button(
            ui,
            i18n::tr("Copia del historial", "Backup history"),
            palette.card,
        )
        .clicked()
        {
            app.send(Command::BackupHistory);
        }
    });
    ui.add_space(12.0);

    section(ui, i18n::tr("Capturas guardadas", "Saved snapshots"));
    if app.history.is_empty() {
        empty(
            ui,
            i18n::tr("Historial vacío todavía.", "History is still empty."),
        );
    } else {
        for row in app.history.iter().take(60) {
            ui.horizontal(|ui| {
                severity_dot(
                    ui,
                    if row.has_critical {
                        Severity::Critical
                    } else {
                        Severity::Healthy
                    },
                    7.0,
                );
                ui.label(RichText::new(&row.collected_at).size(11.0).monospace());
                ui.label(RichText::new(format!("CPU {:.1}%", row.cpu_usage)).size(11.0));
                ui.label(
                    RichText::new(format!(
                        "RAM {:.1}/{:.1} GB",
                        row.memory_used_gb, row.memory_total_gb
                    ))
                    .size(11.0),
                );
                ui.label(
                    RichText::new(format!("{} alertas", row.alerts_count))
                        .size(11.0)
                        .color(palette.dim),
                );
                ui.label(
                    RichText::new(&row.dominant_process)
                        .size(11.0)
                        .color(palette.dim),
                );
            });
        }
    }

    if !app.incidents.is_empty() {
        ui.add_space(14.0);
        section(
            ui,
            i18n::tr("Incidentes persistidos", "Persisted incidents"),
        );
        for incident in app.incidents.iter().take(20) {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    severity_dot(ui, incident.severity, 8.0);
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&incident.title).size(12.5).strong());
                        ui.label(
                            RichText::new(incident.collected_at.to_rfc3339())
                                .size(10.5)
                                .color(palette.dim),
                        );
                        ui.label(
                            RichText::new(&incident.summary)
                                .size(11.0)
                                .color(palette.dim),
                        );
                    });
                });
            });
            ui.add_space(5.0);
        }
    }

    if !app.audits.is_empty() {
        ui.add_space(14.0);
        section(ui, i18n::tr("Auditoría de acciones", "Action audit trail"));
        for record in app.audits.iter().take(40) {
            ui.horizontal(|ui| {
                severity_dot(
                    ui,
                    if record.success {
                        Severity::Healthy
                    } else {
                        Severity::Warning
                    },
                    7.0,
                );
                ui.label(
                    RichText::new(&record.occurred_at)
                        .size(10.5)
                        .monospace()
                        .color(palette.dim),
                );
                ui.label(RichText::new(&record.action).size(11.0).strong());
                ui.label(RichText::new(&record.target).size(11.0).color(palette.dim));
                ui.label(
                    RichText::new(truncate(&record.detail, 80))
                        .size(10.5)
                        .color(palette.dim),
                );
            });
        }
    }
}

fn draw_config(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();
    let mut changed = false;

    section(
        ui,
        i18n::tr("Apariencia e idioma", "Appearance and language"),
    );
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(i18n::tr("Tema:", "Theme:"));
            for (mode, label) in [
                (ThemeMode::Dark, i18n::tr("Oscuro", "Dark")),
                (ThemeMode::Light, i18n::tr("Claro", "Light")),
                (ThemeMode::System, i18n::tr("Sistema", "System")),
            ] {
                if ui
                    .selectable_label(app.config.ui.theme == mode, label)
                    .clicked()
                {
                    app.config.ui.theme = mode;
                    changed = true;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(i18n::tr("Idioma:", "Language:"));
            for lang in [Lang::Es, Lang::En] {
                if ui
                    .selectable_label(app.config.ui.language == lang, lang.native_name())
                    .clicked()
                {
                    app.config.ui.language = lang;
                    i18n::set_lang(lang);
                    changed = true;
                }
            }
        });
    });

    ui.add_space(12.0);
    section(ui, i18n::tr("Captura", "Collection"));
    card(ui, |ui| {
        changed |= slider_row(
            ui,
            i18n::tr("Intervalo de refresco (s)", "Refresh interval (s)"),
            &mut app.config.collection.refresh_interval_secs,
            2..=60,
        );
        changed |= ui
            .checkbox(
                &mut app.config.collection.verify_signatures,
                i18n::tr(
                    "Verificar firma de código (codesign)",
                    "Verify code signature (codesign)",
                ),
            )
            .changed();
    });

    ui.add_space(12.0);
    section(ui, i18n::tr("Umbrales de proceso", "Process thresholds"));
    card(ui, |ui| {
        changed |= threshold_row(
            ui,
            i18n::tr("CPU aviso (%)", "CPU warning (%)"),
            &mut app.config.thresholds.process.cpu_warning_percent,
            5.0..=100.0,
        );
        changed |= threshold_row(
            ui,
            i18n::tr("CPU crítico (%)", "CPU critical (%)"),
            &mut app.config.thresholds.process.cpu_critical_percent,
            5.0..=100.0,
        );
        changed |= threshold_row(
            ui,
            i18n::tr("Memoria aviso (MB)", "Memory warning (MB)"),
            &mut app.config.thresholds.process.memory_warning_mb,
            100.0..=16_000.0,
        );
        changed |= threshold_row(
            ui,
            i18n::tr("Escritura crítica (MB)", "Write critical (MB)"),
            &mut app.config.thresholds.process.io_write_critical_mb,
            10.0..=2_000.0,
        );
    });

    ui.add_space(12.0);
    section(ui, i18n::tr("Qué se vigila", "What is watched"));
    card(ui, |ui| {
        changed |= ui
            .checkbox(
                &mut app.config.anomaly.enabled,
                i18n::tr("Detección de anomalías", "Anomaly detection"),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut app.config.anomaly.watch_persistence,
                i18n::tr("Cambios de persistencia", "Persistence changes"),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut app.config.anomaly.watch_security_controls,
                i18n::tr("Controles de seguridad", "Security controls"),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut app.config.anomaly.watch_tcc,
                i18n::tr("Permisos TCC", "TCC permissions"),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut app.config.anomaly.watch_network_devices,
                i18n::tr("Equipos nuevos en la red", "New network devices"),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut app.config.anomaly.watch_unsigned_binaries,
                i18n::tr("Binarios sin firmar", "Unsigned binaries"),
            )
            .changed();
    });

    ui.add_space(12.0);
    section(ui, i18n::tr("Acciones", "Actions"));
    card(ui, |ui| {
        changed |= ui
            .checkbox(
                &mut app.config.remediation.manual_actions_enabled,
                i18n::tr(
                    "Permitir acciones manuales (finalizar procesos)",
                    "Allow manual actions (terminate processes)",
                ),
            )
            .changed();
        ui.label(
            RichText::new(i18n::tr(
                "RootCause nunca ejecuta acciones automáticas. Toda intervención parte de un clic tuyo y queda auditada.",
                "RootCause never runs automatic actions. Every intervention starts with your click and is audited.",
            ))
            .size(10.5)
            .color(palette.dim),
        );
    });

    ui.add_space(14.0);
    if action_button(
        ui,
        i18n::tr("Guardar configuración", "Save settings"),
        palette.accent,
    )
    .clicked()
        || changed
    {
        let config = Box::new(app.config.clone());
        app.send(Command::SaveConfig(config));
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new(format!("{} {}", i18n::tr("Archivo:", "File:"), app.db_path))
            .size(10.5)
            .color(palette.dim),
    );
}

fn draw_manual(ui: &mut Ui) {
    let palette = pal();

    card(ui, |ui| {
        ui.label(
            RichText::new(i18n::tr(
                "RootCause es un sensor forense, no un antivirus.",
                "RootCause is a forensic sensor, not an antivirus.",
            ))
            .size(14.0)
            .strong(),
        );
        ui.label(
            RichText::new(i18n::tr(
                "No elimina malware ni bloquea por firma. Observa el equipo, detecta distorsiones y cambios contra un estado bueno conocido, y explica dónde mirar con evidencia. Complementa a tu antivirus o EDR; no lo sustituye.",
                "It does not remove malware or block by signature. It observes the machine, detects distortions and changes against a known-good state, and explains where to look with evidence. It complements your antivirus or EDR; it does not replace it.",
            ))
            .size(12.0)
            .color(palette.dim),
        );
    });

    ui.add_space(12.0);
    section(
        ui,
        i18n::tr("Qué hace cada sección", "What each section does"),
    );

    let entries: [(&str, &str); 10] = [
        (
            Tab::Overview.title(),
            i18n::tr(
                "El veredicto de la captura: semáforo, tendencias de CPU/memoria/escritura y las alertas priorizadas.",
                "The snapshot verdict: traffic light, CPU/memory/write trends and prioritised alerts.",
            ),
        ),
        (
            Tab::Processes.title(),
            i18n::tr(
                "Qué se ejecuta, con qué consumo y con qué firma de código. Un binario sin firmar fuera del sistema es una señal fuerte.",
                "What is running, with what usage and what code signature. An unsigned binary outside the system is a strong signal.",
            ),
        ),
        (
            Tab::Connections.title(),
            i18n::tr(
                "Sockets activos por proceso. Marca destinos públicos y puertos a la escucha expuestos a toda la red.",
                "Active sockets per process. Flags public destinations and listening ports exposed to the whole network.",
            ),
        ),
        (
            Tab::Network.title(),
            i18n::tr(
                "Equipos del segmento local. Detecta los nuevos contra una baseline; un cambio de MAC en el router es crítico.",
                "Devices on the local segment. Detects new ones against a baseline; a MAC change on the router is critical.",
            ),
        ),
        (
            Tab::Persistence.title(),
            i18n::tr(
                "LaunchAgents, LaunchDaemons, login items y cron: todo lo que sobrevive a un reinicio. Aquí es donde vive un implante.",
                "LaunchAgents, LaunchDaemons, login items and cron: everything that survives a reboot. This is where an implant lives.",
            ),
        ),
        (
            Tab::Security.title(),
            i18n::tr(
                "Gatekeeper, SIP, FileVault, firewall, SSH y la antigüedad de las firmas de XProtect, cada uno con la evidencia del comando que lo consultó.",
                "Gatekeeper, SIP, FileVault, firewall, SSH and XProtect signature age, each with evidence from the command that queried it.",
            ),
        ),
        (
            Tab::Privacy.title(),
            i18n::tr(
                "Permisos TCC concedidos: quién puede grabar pantalla, leer el teclado o acceder a todo el disco. Requiere Acceso total al disco para leerse.",
                "Granted TCC permissions: who can record the screen, read the keyboard or access the whole disk. Requires Full Disk Access to read.",
            ),
        ),
        (
            Tab::Storage.title(),
            i18n::tr(
                "Cachés y temporales medidos por raíz, con limpieza segura de dos pasos limitada a ~/Library/Caches.",
                "Caches and temp files measured per root, with a safe two-step cleanup limited to ~/Library/Caches.",
            ),
        ),
        (
            Tab::History.title(),
            i18n::tr(
                "Capturas anteriores, incidentes persistidos y la auditoría de cada acción ejecutada desde la app o el CLI.",
                "Previous snapshots, persisted incidents and the audit trail of every action run from the app or CLI.",
            ),
        ),
        (
            i18n::tr("Baselines", "Baselines"),
            i18n::tr(
                "La primera captura se guarda en silencio como estado bueno conocido. Después, todo cambio se reporta hasta que lo aceptas explícitamente.",
                "The first snapshot is stored silently as the known-good state. After that, every change is reported until you accept it explicitly.",
            ),
        ),
    ];

    for (title, description) in entries {
        card(ui, |ui| {
            ui.label(RichText::new(title).size(12.5).strong());
            ui.label(RichText::new(description).size(11.5).color(palette.dim));
        });
        ui.add_space(6.0);
    }

    ui.add_space(10.0);
    section(
        ui,
        i18n::tr("Permisos que puede pedir", "Permissions it may request"),
    );
    card(ui, |ui| {
        ui.label(
            RichText::new(i18n::tr(
                "· Acceso total al disco: necesario para leer TCC.db y auditar permisos.\n\
                 · Automatización: solo si pulsas «Consultar login items».\n\
                 · Ningún permiso se pide de forma silenciosa ni en segundo plano.",
                "· Full Disk Access: required to read TCC.db and audit permissions.\n\
                 · Automation: only if you click \"Query login items\".\n\
                 · No permission is requested silently or in the background.",
            ))
            .size(11.5),
        );
    });
}

fn draw_about(app: &mut RootCauseApp, ui: &mut Ui) {
    let palette = pal();

    card(ui, |ui| {
        ui.horizontal(|ui| {
            draw_logo(ui, 44.0);
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(meta::DISPLAY_NAME).size(19.0).strong());
                ui.label(
                    RichText::new(format!("v{}", meta::VERSION))
                        .size(12.0)
                        .color(palette.dim),
                );
                ui.label(
                    RichText::new(meta::DESCRIPTION)
                        .size(11.5)
                        .color(palette.dim),
                );
            });
        });
    });

    ui.add_space(12.0);
    section(ui, i18n::tr("Equipo", "Machine"));
    card(ui, |ui| {
        info_row(ui, i18n::tr("Host", "Host"), &app.hardware.host_name);
        info_row(ui, i18n::tr("Modelo", "Model"), &app.hardware.model);
        info_row(
            ui,
            i18n::tr("Sistema", "System"),
            &format!("{} {}", app.hardware.os_name, app.hardware.os_version),
        );
        info_row(ui, "CPU", &app.hardware.cpu_brand);
        info_row(
            ui,
            i18n::tr("Núcleos", "Cores"),
            &format!("{} · {}", app.hardware.cpu_cores, app.hardware.architecture),
        );
        info_row(ui, "RAM", &format!("{:.1} GB", app.hardware.total_ram_gb));
        if app.hardware.cpu_freq_mhz > 0 {
            info_row(
                ui,
                i18n::tr("Frecuencia", "Frequency"),
                &format!("{} MHz", app.hardware.cpu_freq_mhz),
            );
        }
    });

    ui.add_space(12.0);
    section(ui, i18n::tr("Contexto de ejecución", "Execution context"));
    card(ui, |ui| match app.environment.as_ref() {
        Some(environment) => {
            info_row(
                ui,
                i18n::tr("Usuario", "User"),
                &format!("{} (uid {})", environment.user, environment.uid),
            );
            info_row(
                ui,
                i18n::tr("Privilegios", "Privileges"),
                if environment.is_root {
                    i18n::tr("root", "root")
                } else {
                    i18n::tr(
                        "usuario normal — lsof solo ve tus sockets",
                        "regular user — lsof only sees your sockets",
                    )
                },
            );
            for (path, description, available) in &environment.tools {
                ui.horizontal(|ui| {
                    severity_dot(
                        ui,
                        if *available {
                            Severity::Healthy
                        } else {
                            Severity::Warning
                        },
                        7.0,
                    );
                    ui.label(RichText::new(path).size(10.5).monospace());
                    ui.label(RichText::new(description).size(10.5).color(palette.dim));
                });
            }
        }
        None => {
            ui.label(RichText::new(i18n::tr("Consultando…", "Querying…")).size(11.0));
        }
    });

    ui.add_space(12.0);
    section(ui, i18n::tr("Proyecto", "Project"));
    card(ui, |ui| {
        info_row(ui, i18n::tr("Autor", "Author"), meta::AUTHOR);
        info_row(ui, i18n::tr("Licencia", "License"), meta::LICENSE);
        info_row(ui, "Bundle ID", meta::BUNDLE_ID);
        info_row(
            ui,
            i18n::tr("Idioma", "Language"),
            app.config.ui.language.code(),
        );
        ui.horizontal(|ui| {
            ui.label(RichText::new("GitHub").size(11.5).color(palette.dim));
            ui.hyperlink_to(
                RichText::new("rootcause-macos-inspector").size(11.5),
                meta::GITHUB,
            );
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(i18n::tr("Versión Windows", "Windows edition"))
                    .size(11.5)
                    .color(palette.dim),
            );
            ui.hyperlink_to(
                RichText::new("rootcause-windows-inspector").size(11.5),
                meta::GITHUB_WINDOWS,
            );
        });
    });

    ui.add_space(12.0);
    section(ui, i18n::tr("Atajos de teclado", "Keyboard shortcuts"));
    card(ui, |ui| {
        info_row(
            ui,
            "F5",
            i18n::tr("Actualizar la captura", "Refresh the snapshot"),
        );
        info_row(
            ui,
            "⌘E",
            i18n::tr("Exportar la captura a JSON", "Export snapshot to JSON"),
        );
        info_row(
            ui,
            "⌘R",
            i18n::tr("Generar reporte forense", "Generate forensic report"),
        );
    });
}

// ── Componentes de dibujo ───────────────────────────────────────────────────

fn card<R>(ui: &mut Ui, contents: impl FnOnce(&mut Ui) -> R) -> R {
    let palette = pal();
    Frame::none()
        .fill(palette.card)
        .stroke(Stroke::new(1.0, palette.border))
        .rounding(Rounding::same(9.0))
        .inner_margin(Margin::symmetric(14.0, 11.0))
        .show(ui, contents)
        .inner
}

fn section(ui: &mut Ui, title: &str) {
    let palette = pal();
    ui.label(RichText::new(title).size(12.0).strong().color(palette.dim));
    ui.add_space(6.0);
}

fn note(ui: &mut Ui, text: &str) {
    let palette = pal();
    ui.add_space(6.0);
    ui.label(RichText::new(text).size(10.5).color(palette.dim).italics());
}

fn empty(ui: &mut Ui, message: &str) {
    let palette = pal();
    card(ui, |ui| {
        ui.label(RichText::new(message).size(12.0).color(palette.dim));
    });
}

fn loading(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        ui.add(egui::Spinner::new().size(28.0));
        ui.add_space(10.0);
        ui.label(
            RichText::new(i18n::tr(
                "Recogiendo la primera captura del sistema…",
                "Collecting the first system snapshot…",
            ))
            .size(12.5)
            .color(pal().dim),
        );
    });
}

fn limitations(ui: &mut Ui, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let palette = pal();
    ui.add_space(12.0);
    section(ui, i18n::tr("Limitaciones honestas", "Honest limitations"));
    for item in items {
        ui.label(
            RichText::new(format!("· {item}"))
                .size(10.5)
                .color(palette.dim),
        );
    }
}

fn action_button(ui: &mut Ui, label: &str, background: Color32) -> Response {
    let palette = pal();
    let text_color = if background == palette.accent || background == palette.crit {
        Color32::WHITE
    } else {
        palette.text
    };
    ui.add(
        egui::Button::new(RichText::new(label).size(11.5).color(text_color))
            .fill(background)
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(0.0, 26.0)),
    )
}

fn pill(ui: &mut Ui, text: &str, color: Color32) {
    Frame::none()
        .fill(color.gamma_multiply(0.18))
        .rounding(Rounding::same(9.0))
        .inner_margin(Margin::symmetric(7.0, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(9.5).color(color).strong());
        });
}

fn signature_pill(ui: &mut Ui, signature: CodeSignature) {
    pill(ui, signature.label(), severity_color(signature.risk()));
}

fn summary_pill(ui: &mut Ui, label: &str, value: &str, severity: Severity) {
    let palette = pal();
    Frame::none()
        .fill(palette.card)
        .stroke(Stroke::new(1.0, palette.border))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(label).size(10.0).color(palette.dim));
                ui.label(
                    RichText::new(value)
                        .size(15.0)
                        .strong()
                        .color(severity_color(severity)),
                );
            });
        });
    ui.add_space(8.0);
}

fn metric_card(ui: &mut Ui, label: &str, value: &str, series: &[f32], color: Color32) {
    let palette = pal();
    Frame::none()
        .fill(palette.card)
        .stroke(Stroke::new(1.0, palette.border))
        .rounding(Rounding::same(9.0))
        .inner_margin(Margin::symmetric(14.0, 11.0))
        .show(ui, |ui| {
            ui.set_min_width(210.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(label).size(10.5).color(palette.dim));
                ui.label(RichText::new(value).size(18.0).strong());
                ui.add_space(4.0);
                sparkline(ui, series, color);
            });
        });
    ui.add_space(10.0);
}

/// Mini gráfico de tendencia. Sin ejes ni leyenda a propósito: solo responde
/// "¿esto sube, baja o está plano?".
fn sparkline(ui: &mut Ui, series: &[f32], color: Color32) {
    let width = 180.0;
    let height = 30.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());

    if series.len() < 2 {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "—",
            egui::FontId::proportional(11.0),
            pal().dim,
        );
        return;
    }

    let maximum = series.iter().cloned().fold(1.0_f32, f32::max);
    let step = rect.width() / (series.len() - 1) as f32;
    let points: Vec<egui::Pos2> = series
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let x = rect.left() + index as f32 * step;
            let y = rect.bottom() - (value / maximum).clamp(0.0, 1.0) * rect.height();
            egui::pos2(x, y)
        })
        .collect();

    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.6, color)));
}

fn severity_dot(ui: &mut Ui, severity: Severity, size: f32) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let color = severity_color(severity);
    ui.painter().circle_filled(rect.center(), size / 2.0, color);
    ui.painter().circle_stroke(
        rect.center(),
        size / 2.0 + 2.0,
        Stroke::new(1.0, color.gamma_multiply(0.35)),
    );
    response.on_hover_text(severity.label());
}

fn draw_logo(ui: &mut Ui, size: f32) {
    let palette = pal();
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let center = rect.center();
    let painter = ui.painter();
    painter.circle_stroke(
        center,
        size * 0.42,
        Stroke::new(size * 0.09, palette.accent),
    );
    painter.circle_stroke(
        center,
        size * 0.21,
        Stroke::new(size * 0.09, palette.accent),
    );
    painter.circle_filled(center, size * 0.07, palette.accent);
}

fn table_header(ui: &mut Ui, columns: &[(&str, f32)]) {
    let palette = pal();
    ui.horizontal(|ui| {
        for (label, width) in columns {
            ui.allocate_ui(Vec2::new(*width, 16.0), |ui| {
                ui.label(RichText::new(*label).size(9.5).color(palette.dim).strong());
            });
        }
    });
    ui.add_space(4.0);
}

fn fact(ui: &mut Ui, label: &str, value: &str) {
    let palette = pal();
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(9.5).color(palette.dim));
        ui.label(
            RichText::new(if value.is_empty() { "—" } else { value })
                .size(12.0)
                .strong(),
        );
    });
    ui.add_space(18.0);
}

fn info_row(ui: &mut Ui, label: &str, value: &str) {
    let palette = pal();
    ui.horizontal(|ui| {
        ui.allocate_ui(Vec2::new(150.0, 18.0), |ui| {
            ui.label(RichText::new(label).size(11.0).color(palette.dim));
        });
        ui.label(RichText::new(if value.is_empty() { "—" } else { value }).size(11.5));
    });
}

fn slider_row(
    ui: &mut Ui,
    label: &str,
    value: &mut u64,
    range: std::ops::RangeInclusive<u64>,
) -> bool {
    ui.horizontal(|ui| {
        ui.allocate_ui(Vec2::new(220.0, 20.0), |ui| {
            ui.label(RichText::new(label).size(11.0));
        });
        ui.add(egui::Slider::new(value, range)).changed()
    })
    .inner
}

fn threshold_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    ui.horizontal(|ui| {
        ui.allocate_ui(Vec2::new(220.0, 20.0), |ui| {
            ui.label(RichText::new(label).size(11.0));
        });
        ui.add(egui::Slider::new(value, range)).changed()
    })
    .inner
}

// ── Utilidades ──────────────────────────────────────────────────────────────

fn configure_style(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 5.0);
    style.spacing.button_padding = Vec2::new(10.0, 5.0);
    ctx.set_style(style);
}

fn severity_color(severity: Severity) -> Color32 {
    let palette = pal();
    match severity {
        Severity::Healthy => palette.ok,
        Severity::Warning => palette.warn,
        Severity::Critical => palette.crit,
    }
}

fn verdict_title(severity: Severity) -> &'static str {
    match severity {
        Severity::Healthy => i18n::tr("Equipo sin señales relevantes", "No relevant signals"),
        Severity::Warning => i18n::tr(
            "Hay algo que merece tu atención",
            "Something deserves your attention",
        ),
        Severity::Critical => i18n::tr("Señal crítica detectada", "Critical signal detected"),
    }
}

/// Añade una muestra al búfer circular de tendencia (60 puntos).
fn push_sample(series: &mut Vec<f32>, value: f32) {
    series.push(value);
    if series.len() > 60 {
        series.remove(0);
    }
}

fn percent(part: f32, total: f32) -> f32 {
    if total <= 0.0 {
        0.0
    } else {
        (part / total) * 100.0
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_owned();
    }
    let mut out: String = value.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_buffer_de_tendencia_no_crece_sin_limite() {
        let mut series = Vec::new();
        for value in 0..100 {
            push_sample(&mut series, value as f32);
        }
        assert_eq!(series.len(), 60);
        assert_eq!(series[59], 99.0);
        assert_eq!(series[0], 40.0);
    }

    #[test]
    fn el_porcentaje_tolera_total_cero() {
        assert_eq!(percent(1.0, 0.0), 0.0);
        assert!((percent(2.0, 8.0) - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn recorta_respetando_caracteres_multibyte() {
        assert_eq!(truncate("corto", 10), "corto");
        assert_eq!(truncate("ñññññ", 3), "ññ…");
    }

    #[test]
    fn hay_doce_secciones_y_ninguna_repetida() {
        assert_eq!(Tab::ALL.len(), 12);
        let titles: Vec<&str> = Tab::ALL.iter().map(|tab| tab.title()).collect();
        let mut unique = titles.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(titles.len(), unique.len());
    }

    #[test]
    fn todas_las_secciones_tienen_subtitulo() {
        assert!(Tab::ALL.iter().all(|tab| !tab.subtitle().is_empty()));
    }
}
