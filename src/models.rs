//! Modelos de dominio del monitor.
//!
//! Se mantienen deliberadamente explícitos y serializables para cuatro fines:
//! 1. Renderizar la interfaz de forma estable.
//! 2. Exportar evidencia en JSON.
//! 3. Persistir resúmenes históricos en SQLite.
//! 4. Comparar el estado actual contra una baseline "buena conocida".

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Severidad visual para el semáforo principal y para tablas detalladas.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Severity {
    #[default]
    Healthy,
    Warning,
    Critical,
}

impl Severity {
    /// Texto humano para la interfaz.
    pub fn label(self) -> &'static str {
        match self {
            Self::Healthy => "Verde",
            Self::Warning => "Amarillo",
            Self::Critical => "Rojo",
        }
    }
}

/// Severidad específica del módulo de anomalías y riesgo.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum RiskLevel {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Bajo",
            Self::Medium => "Medio",
            Self::High => "Alto",
            Self::Critical => "Crítico",
        }
    }

    pub fn to_severity(self) -> Severity {
        match self {
            Self::Low => Severity::Healthy,
            Self::Medium => Severity::Warning,
            Self::High | Self::Critical => Severity::Critical,
        }
    }
}

/// Estado operativo del propio agente RootCause.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AgentStatus {
    #[default]
    Healthy,
    Degraded,
    Recovered,
}

impl AgentStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Healthy => "Saludable",
            Self::Degraded => "Degradado",
            Self::Recovered => "Recuperado",
        }
    }
}

/// Resumen del estado global del equipo en una instantánea concreta.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemOverview {
    pub cpu_usage_percent: f32,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
    pub network_rx_mb_delta: f32,
    pub network_tx_mb_delta: f32,
    pub io_read_mb_delta: f32,
    pub io_write_mb_delta: f32,
    pub cache_total_mb: f32,
    pub primary_severity: Severity,
    pub primary_reason: String,
}

/// Hallazgo resumido que explica por qué un proceso o condición merece atención.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Alert {
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub pid: Option<u32>,
    pub path: Option<String>,
    pub hint: String,
}

/// Vista enriquecida por proceso.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessInsight {
    pub pid: u32,
    pub name: String,
    pub exe_path: String,
    pub parent_pid: Option<u32>,
    pub user: String,
    pub cpu_percent: f32,
    pub memory_mb: f32,
    pub io_read_mb_delta: f32,
    pub io_write_mb_delta: f32,
    pub status: String,
    pub category: String,
    pub severity: Severity,
    pub score: u8,
    pub can_terminate: bool,
    pub reasons: Vec<String>,
    /// Línea de comandos completa, obtenida bajo demanda para procesos críticos.
    #[serde(default)]
    pub command_line: Option<String>,
    /// Estado de firma de código (`codesign`): Apple, Developer ID, ad-hoc, sin firmar…
    #[serde(default)]
    pub signature: Option<CodeSignature>,
}

/// Resultado resumido de verificar la firma de código de un binario.
///
/// En macOS la firma es la primera línea de confianza: un binario sin firma o
/// con firma ad-hoc corriendo desde una ruta de usuario es una señal mucho más
/// fuerte que el mismo binario firmado por Apple dentro de `/System`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CodeSignature {
    /// Firmado por Apple (software del sistema).
    Apple,
    /// Firmado con un certificado Developer ID / Mac App Store.
    DeveloperId,
    /// Firma ad-hoc (sin autoridad verificable).
    AdHoc,
    /// Sin firma de código.
    Unsigned,
    /// No se pudo determinar (binario inaccesible, permisos, error de `codesign`).
    #[default]
    Unknown,
}

impl CodeSignature {
    pub fn label(self) -> &'static str {
        match self {
            Self::Apple => "Apple",
            Self::DeveloperId => "Developer ID",
            Self::AdHoc => "Ad-hoc",
            Self::Unsigned => "Sin firmar",
            Self::Unknown => "Desconocida",
        }
    }

    /// Riesgo asociado a la firma por sí sola (sin contexto de ruta).
    pub fn risk(self) -> Severity {
        match self {
            Self::Apple | Self::DeveloperId => Severity::Healthy,
            Self::AdHoc | Self::Unknown => Severity::Warning,
            Self::Unsigned => Severity::Critical,
        }
    }
}

/// Información de hardware del equipo, recopilada una sola vez al iniciar.
#[derive(Debug, Clone, Default)]
pub struct HardwareInfo {
    pub os_name: String,
    pub os_version: String,
    pub host_name: String,
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub cpu_freq_mhz: u64,
    pub total_ram_gb: f32,
    pub architecture: String,
    /// Modelo del equipo según `sysctl hw.model` (p. ej. `Mac15,3`).
    pub model: String,
}

/// Fila resumida del historial SQLite, lista para mostrar en la UI.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SnapshotRow {
    pub id: i64,
    pub collected_at: String,
    pub cpu_usage: f32,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
    pub io_write_mb_delta: f32,
    pub cache_total_mb: f32,
    pub dominant_process: String,
    pub alerts_count: usize,
    pub has_critical: bool,
}

/// Evidencia atómica asociada a un incidente resumido.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IncidentEvidence {
    pub kind: String,
    pub label: String,
    pub value: String,
}

/// Estado de un elemento vigilado respecto a la baseline conocida.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PersistenceChange {
    /// Igual que en la baseline (o sin baseline todavía).
    #[default]
    Unchanged,
    /// Entrada nueva que no estaba en la baseline.
    Added,
    /// Entrada que existía pero cambió de contenido.
    Modified,
    /// Entrada que estaba en la baseline y ya no aparece.
    Removed,
}

impl PersistenceChange {
    /// Etiqueta corta para UI/CLI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Unchanged => "",
            Self::Added => "NUEVA",
            Self::Modified => "MODIFICADA",
            Self::Removed => "ELIMINADA",
        }
    }

    /// `true` si representa un cambio respecto a la baseline.
    pub fn is_change(self) -> bool {
        !matches!(self, Self::Unchanged)
    }
}

/// Ámbito de una entrada de persistencia en macOS.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PersistenceScope {
    /// `~/Library/LaunchAgents` — se ejecuta al iniciar sesión el usuario.
    UserAgent,
    /// `/Library/LaunchAgents` — agentes para todos los usuarios (instalados por apps).
    GlobalAgent,
    /// `/Library/LaunchDaemons` — daemons de root, arrancan con el sistema.
    GlobalDaemon,
    /// `/System/Library/Launch*` — provisto por Apple, protegido por SIP.
    SystemApple,
    /// Login items / elementos de inicio de sesión.
    LoginItem,
    /// `cron`, `/etc/periodic`, `at`.
    Cron,
    /// Otro origen no clasificado.
    #[default]
    Other,
}

impl PersistenceScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::UserAgent => "LaunchAgent (usuario)",
            Self::GlobalAgent => "LaunchAgent (global)",
            Self::GlobalDaemon => "LaunchDaemon (root)",
            Self::SystemApple => "Apple / SIP",
            Self::LoginItem => "Login item",
            Self::Cron => "cron / periodic",
            Self::Other => "Otro",
        }
    }

    /// Peso base de riesgo: un daemon de root pesa más que un agente de usuario.
    pub fn base_risk(self) -> u8 {
        match self {
            Self::GlobalDaemon => 26,
            Self::GlobalAgent => 20,
            Self::UserAgent => 16,
            Self::LoginItem => 12,
            Self::Cron => 18,
            Self::SystemApple => 0,
            Self::Other => 10,
        }
    }
}

/// Entrada observable de persistencia en macOS: LaunchAgent, LaunchDaemon,
/// login item o tarea `cron`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistenceEntry {
    /// Tipo legible (`LaunchAgent`, `LaunchDaemon`, `LoginItem`, `cron`).
    pub entry_kind: String,
    /// Ruta del plist o del archivo que define la persistencia.
    pub location: String,
    /// `Label` del plist o nombre del elemento.
    pub name: String,
    /// Comando efectivo (`Program` + `ProgramArguments`).
    pub command: String,
    #[serde(default)]
    pub scope: PersistenceScope,
    #[serde(default)]
    pub target_path: Option<String>,
    #[serde(default)]
    pub exists_on_disk: bool,
    /// `RunAtLoad` del plist: arranca solo al cargar.
    #[serde(default)]
    pub run_at_load: bool,
    /// `KeepAlive` del plist: se relanza si muere (típico de implantes).
    #[serde(default)]
    pub keep_alive: bool,
    /// `StartInterval` en segundos, si lo define.
    #[serde(default)]
    pub start_interval_secs: Option<u64>,
    /// Firma del binario destino.
    #[serde(default)]
    pub signature: Option<CodeSignature>,
    #[serde(default)]
    pub severity: RiskLevel,
    #[serde(default)]
    pub note: String,
    /// Estado de cambio respecto a la baseline de persistencia conocida.
    #[serde(default)]
    pub change_status: PersistenceChange,
}

/// Ítem observado en una superficie vigilada (persistencia, red, seguridad…).
/// Es la unidad del motor genérico de detección de cambios contra baseline.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WatchedItem {
    /// Clave estable que identifica el ítem a lo largo del tiempo.
    pub key: String,
    /// Valor cuyo cambio se detecta (comparado literal contra la baseline).
    pub value: String,
    /// Nombre legible para UI/CLI.
    pub label: String,
    /// Detalle contextual (ruta, comando, estado).
    pub detail: String,
    /// Estado de cambio respecto a la baseline (se rellena tras el diff).
    #[serde(default)]
    pub change_status: PersistenceChange,
}

/// Evento atómico del módulo de detección anómala.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnomalyEvent {
    pub event_id: String,
    pub detected_at: DateTime<Utc>,
    #[serde(default)]
    pub severity: RiskLevel,
    #[serde(default)]
    pub score: u16,
    #[serde(default)]
    pub status: String,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub process_name: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub parent_pid: Option<u32>,
    #[serde(default)]
    pub parent_name: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub exe_path: Option<String>,
    #[serde(default)]
    pub cpu_percent: Option<f32>,
    #[serde(default)]
    pub memory_mb: Option<f32>,
    #[serde(default)]
    pub io_write_mb_delta: Option<f32>,
    #[serde(default)]
    pub unique_public_remotes: Option<usize>,
    #[serde(default)]
    pub unique_private_remotes: Option<usize>,
    pub summary: String,
    pub root_cause_hypothesis: String,
    pub recommended_action: String,
    #[serde(default)]
    pub evidence: Vec<IncidentEvidence>,
}

/// Resumen persistible de un incidente o degradación detectada.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IncidentSummary {
    pub incident_id: String,
    pub fingerprint: String,
    pub collected_at: DateTime<Utc>,
    pub severity: Severity,
    pub kind: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub root_cause_hypothesis: String,
    pub probable_causes: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub evidence: Vec<IncidentEvidence>,
    #[serde(default)]
    pub risk_level: Option<RiskLevel>,
    #[serde(default)]
    pub risk_score: u16,
    #[serde(default)]
    pub anomaly_count: usize,
    #[serde(default)]
    pub anomaly_types: Vec<String>,
    #[serde(default)]
    pub anomaly_events: Vec<AnomalyEvent>,
    #[serde(default)]
    pub ai_advice: Option<AiIncidentAdvice>,
}

/// Respuesta opcional de un adaptador IA desacoplado del motor principal.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiIncidentAdvice {
    pub provider: String,
    pub model: String,
    pub summary: String,
    pub probable_causes: Vec<String>,
    pub suggested_actions: Vec<String>,
    pub confidence: String,
    pub warnings: Vec<String>,
    pub generated_at: String,
}

/// Registro de acciones ejecutadas desde la app o el CLI.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditRecord {
    pub occurred_at: String,
    pub action: String,
    pub target: String,
    pub success: bool,
    pub detail: String,
}

/// Estado resumido de resiliencia del propio agente.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentHealth {
    #[serde(default)]
    pub status: AgentStatus,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub last_start_at: String,
    #[serde(default)]
    pub last_heartbeat_at: String,
    #[serde(default)]
    pub last_clean_shutdown_at: Option<String>,
    #[serde(default)]
    pub config_fingerprint: String,
    #[serde(default)]
    pub config_changed: bool,
    #[serde(default)]
    pub unexpected_shutdown_detected: bool,
    #[serde(default)]
    pub consecutive_unexpected_stops: u32,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// Elemento medible dentro de cachés o carpetas temporales de macOS.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheEntry {
    pub path: String,
    pub size_mb: f32,
    pub file_count: u64,
    pub severity: Severity,
    pub note: String,
    /// `true` si RootCause considera seguro vaciar esta ruta.
    #[serde(default)]
    pub safe_to_clean: bool,
}

/// Resumen de cachés y temporales relevantes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheOverview {
    pub total_mb: f32,
    pub roots_scanned: Vec<String>,
    pub top_entries: Vec<CacheEntry>,
    pub limitations: Vec<String>,
}

/// Resultado de una limpieza de cachés del usuario.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheCleanResult {
    pub freed_mb: f32,
    pub deleted_count: u64,
    pub skipped_in_use: u64,
    pub skipped_recent: u64,
    pub error_count: u64,
    pub dry_run: bool,
}

/// Conexión observada a partir de `lsof -i`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionInsight {
    pub protocol: String,
    pub local_address: String,
    pub remote_address: String,
    pub state: String,
    pub pid: u32,
    pub process_name: String,
    pub exe_path: String,
    pub user: String,
    pub severity: Severity,
    pub reason: String,
    pub is_public_remote: bool,
    /// `true` si el socket está escuchando (puerto abierto en este equipo).
    #[serde(default)]
    pub is_listening: bool,
}

/// Dispositivo observado en la red local a partir de la tabla ARP/NDP y,
/// opcionalmente, de un barrido activo del segmento.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkDevice {
    pub ip: String,
    pub mac: String,
    #[serde(default)]
    pub hostname: String,
    /// Fabricante aproximado deducido del prefijo OUI de la MAC.
    #[serde(default)]
    pub vendor: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub interface: String,
    #[serde(default)]
    pub is_gateway: bool,
    #[serde(default)]
    pub is_self: bool,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub change_status: PersistenceChange,
}

/// Resultado de una exploración de la red local (equipos cercanos).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkScan {
    pub scanned_at: String,
    pub adapter_name: String,
    pub local_ip: String,
    #[serde(default)]
    pub local_mac: String,
    /// Prefijo del segmento, p. ej. `192.168.1` (asume /24 para el barrido).
    pub subnet_prefix: String,
    pub gateway_ip: String,
    #[serde(default)]
    pub deep: bool,
    pub devices: Vec<NetworkDevice>,
    pub total_devices: usize,
    #[serde(default)]
    pub new_devices: usize,
    #[serde(default)]
    pub limitations: Vec<String>,
}

/// Estado de un control de seguridad nativo de macOS.
///
/// Es la unidad del tab **Seguridad**: cada control responde "¿está activo?" con
/// evidencia textual del comando que lo consultó. Un control apagado no es por
/// sí solo una infección, pero sí una superficie abierta que merece explicación.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityControl {
    /// Identificador estable (`gatekeeper`, `sip`, `filevault`, `firewall`…).
    pub id: String,
    /// Nombre legible del control.
    pub name: String,
    /// Estado resumido en una palabra (`Activado`, `Desactivado`, `Desconocido`).
    pub status: String,
    /// `true` si el control está en su estado protegido.
    pub enabled: bool,
    pub severity: Severity,
    /// Salida cruda o fragmento del comando consultado, como evidencia.
    pub evidence: String,
    /// Qué significa y qué hacer si está apagado.
    pub explanation: String,
    #[serde(default)]
    pub change_status: PersistenceChange,
}

/// Versión de una base de firmas antimalware de Apple (XProtect y familia).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MalwareDefinition {
    /// `XProtect`, `XProtect Remediator`, `MRT`, `Gatekeeper (config-data)`.
    pub component: String,
    pub version: String,
    /// Fecha de modificación del bundle en disco (RFC 3339).
    pub last_modified: String,
    /// Días transcurridos desde la última actualización.
    pub age_days: i64,
    pub severity: Severity,
    pub path: String,
    pub note: String,
}

/// Estado del subsistema antimalware de Apple, con su antigüedad.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct XProtectStatus {
    pub definitions: Vec<MalwareDefinition>,
    /// Antigüedad de la definición más reciente, en días.
    pub freshest_age_days: i64,
    pub severity: Severity,
    pub headline: String,
    #[serde(default)]
    pub limitations: Vec<String>,
}

/// Permiso concedido en la base TCC (Transparency, Consent and Control).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TccPermission {
    /// Servicio TCC (`kTCCServiceSystemPolicyAllFiles`, `kTCCServiceMicrophone`…).
    pub service: String,
    /// Nombre legible del servicio.
    pub service_label: String,
    /// Bundle id o ruta del cliente que tiene el permiso.
    pub client: String,
    /// `allowed`, `denied`, `limited`, `unknown`.
    pub decision: String,
    /// `true` si el permiso está concedido.
    pub allowed: bool,
    /// `usuario` o `sistema`, según de qué TCC.db proviene.
    pub database: String,
    /// Fecha de la última modificación del registro, si está disponible.
    #[serde(default)]
    pub last_modified: String,
    pub severity: Severity,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub change_status: PersistenceChange,
}

/// Resultado de leer las bases TCC del sistema y del usuario.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TccOverview {
    pub permissions: Vec<TccPermission>,
    /// `true` si se pudo leer al menos una base TCC.
    pub readable: bool,
    /// `true` si RootCause tiene Acceso total al disco (necesario para TCC).
    pub full_disk_access: bool,
    pub headline: String,
    #[serde(default)]
    pub sensitive_count: usize,
    #[serde(default)]
    pub limitations: Vec<String>,
}

/// Evento reciente del log unificado de macOS relevante para seguridad.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventRecord {
    pub timestamp: String,
    /// Proceso o subsistema emisor (`syspolicyd`, `XProtect`, `sudo`…).
    pub provider: String,
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub severity: Severity,
}

/// Estado de un servicio de launchd relevante para el diagnóstico.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceState {
    pub name: String,
    pub display_name: String,
    /// `Cargado`, `Detenido`, `No encontrado`.
    pub status: String,
    /// Dominio launchd (`system`, `gui/501`).
    pub start_type: String,
    /// PID si está corriendo.
    #[serde(default)]
    pub pid: Option<u32>,
    /// Último código de salida reportado por launchd.
    #[serde(default)]
    pub last_exit_code: Option<i32>,
}

/// Instantánea completa que la UI consume y que también puede exportarse.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemSnapshot {
    pub collected_at: DateTime<Utc>,
    pub overview: SystemOverview,
    pub alerts: Vec<Alert>,
    #[serde(default)]
    pub agent_health: AgentHealth,
    pub processes: Vec<ProcessInsight>,
    pub caches: CacheOverview,
    pub connections: Vec<ConnectionInsight>,
    #[serde(default)]
    pub network: Option<NetworkScan>,
    #[serde(default)]
    pub events: Vec<EventRecord>,
    pub services: Vec<ServiceState>,
    #[serde(default)]
    pub persistence_entries: Vec<PersistenceEntry>,
    #[serde(default)]
    pub security_controls: Vec<SecurityControl>,
    #[serde(default)]
    pub xprotect: XProtectStatus,
    #[serde(default)]
    pub tcc: TccOverview,
    #[serde(default)]
    pub anomalies: Vec<AnomalyEvent>,
    #[serde(default)]
    pub incident: Option<IncidentSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severidad_ordena_de_menor_a_mayor() {
        assert!(Severity::Healthy < Severity::Warning);
        assert!(Severity::Warning < Severity::Critical);
    }

    #[test]
    fn riesgo_alto_mapea_a_severidad_critica() {
        assert_eq!(RiskLevel::High.to_severity(), Severity::Critical);
        assert_eq!(RiskLevel::Medium.to_severity(), Severity::Warning);
        assert_eq!(RiskLevel::Low.to_severity(), Severity::Healthy);
    }

    #[test]
    fn firma_sin_firmar_es_critica_y_apple_es_sana() {
        assert_eq!(CodeSignature::Unsigned.risk(), Severity::Critical);
        assert_eq!(CodeSignature::Apple.risk(), Severity::Healthy);
    }

    #[test]
    fn daemon_root_pesa_mas_que_agente_de_usuario() {
        assert!(
            PersistenceScope::GlobalDaemon.base_risk() > PersistenceScope::UserAgent.base_risk()
        );
        assert_eq!(PersistenceScope::SystemApple.base_risk(), 0);
    }

    #[test]
    fn cambio_no_alterado_no_cuenta_como_cambio() {
        assert!(!PersistenceChange::Unchanged.is_change());
        assert!(PersistenceChange::Added.is_change());
        assert_eq!(PersistenceChange::Modified.label(), "MODIFICADA");
    }
}
