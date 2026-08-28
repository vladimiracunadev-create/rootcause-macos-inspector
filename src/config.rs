//! Configuración operativa de RootCause.
//!
//! Se mantiene en JSON para evitar dependencias nuevas y porque el proyecto ya
//! usa `serde_json` en exportes e historial. Vive en
//! `~/Library/Application Support/RootCauseInspector/rootcause-config.json`.

use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG_FILE: &str = "rootcause-config.json";
const DEFAULT_APP_DIR: &str = "RootCauseInspector";

/// Configuración completa del producto, tal como se serializa en
/// `rootcause-config.json`. Cada sección tiene `Default`, así que un archivo
/// parcial —o vacío— sigue siendo válido y solo sobrescribe lo que declara.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RootCauseConfig {
    #[serde(default)]
    pub collection: CollectionConfig,
    #[serde(default)]
    pub thresholds: ThresholdsConfig,
    #[serde(default)]
    pub anomaly: AnomalyConfig,
    #[serde(default)]
    pub alerting: AlertingConfig,
    #[serde(default)]
    pub remediation: RemediationConfig,
    #[serde(default)]
    pub resilience: ResilienceConfig,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

/// Modo de interfaz (apariencia). `system` sigue el tema de macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Tema oscuro de marca (azul profundo del icono). Por defecto.
    #[default]
    Dark,
    /// Tema claro de marca.
    Light,
    /// Sigue la apariencia clara/oscura de macOS.
    System,
}

/// Preferencias de la interfaz gráfica: idioma y modo de apariencia.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiConfig {
    /// Idioma de la interfaz (`es` por defecto, `en` disponible).
    #[serde(default)]
    pub language: Lang,
    /// Modo de apariencia (`dark` por defecto).
    #[serde(default)]
    pub theme: ThemeMode,
    /// Genera un reporte forense automáticamente al cambiar el día (opt-in).
    #[serde(default)]
    pub daily_report: bool,
}

/// Ritmo y alcance de la captura: cada cuánto se refresca, cuánto historial
/// se conserva y cuánta verificación de firma cabe en una captura.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    #[serde(default = "default_refresh_interval_secs")]
    pub refresh_interval_secs: u64,
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,
    #[serde(default = "default_incident_limit")]
    pub incident_limit: usize,
    /// Verifica la firma de código de los binarios sospechosos (`codesign`).
    /// Tiene coste: se aplica solo a un puñado de procesos por captura.
    #[serde(default = "default_true")]
    pub verify_signatures: bool,
    /// Máximo de binarios a los que se les verifica firma por captura.
    #[serde(default = "default_signature_budget")]
    pub signature_budget: usize,
}

impl Default for CollectionConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: default_refresh_interval_secs(),
            history_limit: default_history_limit(),
            incident_limit: default_incident_limit(),
            verify_signatures: true,
            signature_budget: default_signature_budget(),
        }
    }
}

/// Umbrales de severidad agrupados por superficie.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThresholdsConfig {
    #[serde(default)]
    pub process: ProcessThresholds,
    #[serde(default)]
    pub cache: CacheThresholds,
    #[serde(default)]
    pub xprotect: XProtectThresholds,
}

/// Parámetros del motor de anomalías: umbrales sostenidos, listas de
/// confianza y qué superficies se comparan contra baseline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_anomaly_cpu_sustained_percent")]
    pub cpu_sustained_percent: f32,
    #[serde(default = "default_anomaly_cpu_sustained_samples")]
    pub cpu_sustained_samples: u8,
    #[serde(default = "default_anomaly_memory_growth_mb")]
    pub memory_growth_mb: f32,
    #[serde(default = "default_anomaly_memory_growth_samples")]
    pub memory_growth_samples: u8,
    #[serde(default = "default_anomaly_aggressive_write_mb")]
    pub aggressive_write_mb: f32,
    #[serde(default = "default_anomaly_aggressive_write_samples")]
    pub aggressive_write_samples: u8,
    #[serde(default = "default_anomaly_public_destination_count")]
    pub public_destination_count: usize,
    #[serde(default = "default_anomaly_local_scan_destination_count")]
    pub local_scan_destination_count: usize,
    #[serde(default = "default_anomaly_respawn_window_secs")]
    pub respawn_window_secs: u64,
    #[serde(default = "default_anomaly_respawn_count")]
    pub respawn_count: u8,
    #[serde(default = "default_suspicious_path_keywords")]
    pub suspicious_path_keywords: Vec<String>,
    #[serde(default = "default_trusted_process_names")]
    pub trusted_process_names: Vec<String>,
    #[serde(default = "default_trusted_path_prefixes")]
    pub trusted_path_prefixes: Vec<String>,
    #[serde(default = "default_suspicious_parent_names")]
    pub suspicious_parent_names: Vec<String>,
    #[serde(default = "default_shell_interpreters")]
    pub shell_interpreters: Vec<String>,
    /// Vigila cambios en LaunchAgents/LaunchDaemons/login items vs baseline.
    #[serde(default = "default_true")]
    pub watch_persistence: bool,
    /// Vigila que Gatekeeper, SIP, FileVault y el firewall no cambien de estado.
    #[serde(default = "default_true")]
    pub watch_security_controls: bool,
    /// Vigila permisos TCC nuevos sobre servicios sensibles.
    #[serde(default = "default_true")]
    pub watch_tcc: bool,
    /// Vigila la aparición de equipos nuevos en el segmento de red local.
    #[serde(default = "default_true")]
    pub watch_network_devices: bool,
    /// Marca como anomalía todo binario sin firmar fuera de rutas del sistema.
    #[serde(default = "default_true")]
    pub watch_unsigned_binaries: bool,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cpu_sustained_percent: default_anomaly_cpu_sustained_percent(),
            cpu_sustained_samples: default_anomaly_cpu_sustained_samples(),
            memory_growth_mb: default_anomaly_memory_growth_mb(),
            memory_growth_samples: default_anomaly_memory_growth_samples(),
            aggressive_write_mb: default_anomaly_aggressive_write_mb(),
            aggressive_write_samples: default_anomaly_aggressive_write_samples(),
            public_destination_count: default_anomaly_public_destination_count(),
            local_scan_destination_count: default_anomaly_local_scan_destination_count(),
            respawn_window_secs: default_anomaly_respawn_window_secs(),
            respawn_count: default_anomaly_respawn_count(),
            suspicious_path_keywords: default_suspicious_path_keywords(),
            trusted_process_names: default_trusted_process_names(),
            trusted_path_prefixes: default_trusted_path_prefixes(),
            suspicious_parent_names: default_suspicious_parent_names(),
            shell_interpreters: default_shell_interpreters(),
            watch_persistence: true,
            watch_security_controls: true,
            watch_tcc: true,
            watch_network_devices: true,
            watch_unsigned_binaries: true,
        }
    }
}

/// Umbrales por proceso para CPU, memoria y escritura de disco.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessThresholds {
    #[serde(default = "default_process_cpu_warning")]
    pub cpu_warning_percent: f32,
    #[serde(default = "default_process_cpu_critical")]
    pub cpu_critical_percent: f32,
    #[serde(default = "default_process_memory_warning")]
    pub memory_warning_mb: f32,
    #[serde(default = "default_process_memory_critical")]
    pub memory_critical_mb: f32,
    #[serde(default = "default_process_io_warning")]
    pub io_write_warning_mb: f32,
    #[serde(default = "default_process_io_critical")]
    pub io_write_critical_mb: f32,
}

impl Default for ProcessThresholds {
    fn default() -> Self {
        Self {
            cpu_warning_percent: default_process_cpu_warning(),
            cpu_critical_percent: default_process_cpu_critical(),
            memory_warning_mb: default_process_memory_warning(),
            memory_critical_mb: default_process_memory_critical(),
            io_write_warning_mb: default_process_io_warning(),
            io_write_critical_mb: default_process_io_critical(),
        }
    }
}

/// Tamaño a partir del cual una caché se reporta como voluminosa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheThresholds {
    #[serde(default = "default_cache_warning")]
    pub warning_mb: f32,
    #[serde(default = "default_cache_critical")]
    pub critical_mb: f32,
}

impl Default for CacheThresholds {
    fn default() -> Self {
        Self {
            warning_mb: default_cache_warning(),
            critical_mb: default_cache_critical(),
        }
    }
}

/// Antigüedad tolerada de las definiciones antimalware de Apple.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XProtectThresholds {
    #[serde(default = "default_xprotect_warning_days")]
    pub warning_days: i64,
    #[serde(default = "default_xprotect_critical_days")]
    pub critical_days: i64,
}

impl Default for XProtectThresholds {
    fn default() -> Self {
        Self {
            warning_days: default_xprotect_warning_days(),
            critical_days: default_xprotect_critical_days(),
        }
    }
}

/// Política de alertas: cuántas se muestran y cómo se notifica lo crítico.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    #[serde(default = "default_max_alerts")]
    pub max_alerts: usize,
    #[serde(default = "default_true")]
    pub notify_on_critical: bool,
    #[serde(default = "default_notification_cooldown_secs")]
    pub notification_cooldown_secs: u64,
}

impl Default for AlertingConfig {
    fn default() -> Self {
        Self {
            max_alerts: default_max_alerts(),
            notify_on_critical: true,
            notification_cooldown_secs: default_notification_cooldown_secs(),
        }
    }
}

/// Política de intervención. Separa lo que el usuario puede hacer a mano de
/// lo automático, que en este producto está siempre apagado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationConfig {
    #[serde(default = "default_true")]
    pub manual_actions_enabled: bool,
    /// RootCause nunca ejecuta acciones automáticas: el interruptor existe para
    /// dejar explícita la política y se mantiene apagado.
    #[serde(default)]
    pub automatic_actions_enabled: bool,
}

impl Default for RemediationConfig {
    fn default() -> Self {
        Self {
            manual_actions_enabled: true,
            automatic_actions_enabled: false,
        }
    }
}

/// Autoprotección del agente: latido, ventana de reinicios y vigilancia de
/// la integridad del archivo de configuración.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_heartbeat_interval_secs")]
    pub heartbeat_interval_secs: u64,
    #[serde(default = "default_stale_after_secs")]
    pub stale_after_secs: u64,
    #[serde(default = "default_restart_window_secs")]
    pub restart_window_secs: u64,
    #[serde(default = "default_max_restarts_in_window")]
    pub max_restarts_in_window: u8,
    #[serde(default = "default_true")]
    pub watch_config_integrity: bool,
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            heartbeat_interval_secs: default_heartbeat_interval_secs(),
            stale_after_secs: default_stale_after_secs(),
            restart_window_secs: default_restart_window_secs(),
            max_restarts_in_window: default_max_restarts_in_window(),
            watch_config_integrity: true,
        }
    }
}

/// Adaptador IA opcional. Apagado por defecto; la clave nunca se guarda aquí,
/// solo el nombre de la variable de entorno que la contiene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default = "default_ai_model")]
    pub model: String,
    #[serde(default = "default_ai_api_key_env_var")]
    pub api_key_env_var: String,
    #[serde(default = "default_ai_timeout_secs")]
    pub timeout_secs: u64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: String::new(),
            model: default_ai_model(),
            api_key_env_var: default_ai_api_key_env_var(),
            timeout_secs: default_ai_timeout_secs(),
        }
    }
}

/// Carga, valida y guarda la configuración en disco.
#[derive(Debug, Clone)]
pub struct ConfigManager {
    path: PathBuf,
    config: RootCauseConfig,
}

impl ConfigManager {
    /// Carga la configuración del usuario.
    ///
    /// Nunca falla: si el archivo no existe se usan los valores por defecto en
    /// silencio, y si existe pero es inválido se usan igualmente devolviendo una
    /// advertencia, para que la interfaz la muestre en vez de callarla.
    pub fn load_or_default(app_name: &str) -> (Self, Option<String>) {
        let path = config_path(app_name);
        let path_display = path.display().to_string();
        match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<RootCauseConfig>(&raw) {
                Ok(config) => (Self { path, config }, None),
                Err(error) => (
                    Self {
                        path,
                        config: RootCauseConfig::default(),
                    },
                    Some(format!(
                        "Configuración inválida en {path_display}. Se usan valores por defecto: {error}"
                    )),
                ),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
                Self {
                    path,
                    config: RootCauseConfig::default(),
                },
                None,
            ),
            Err(error) => (
                Self {
                    path,
                    config: RootCauseConfig::default(),
                },
                Some(format!(
                    "No se pudo leer {path_display}. Se usan valores por defecto: {error}"
                )),
            ),
        }
    }

    /// Escribe la configuración por defecto si todavía no hay archivo. Devuelve
    /// la ruta en ambos casos.
    pub fn write_default_if_missing(app_name: &str) -> anyhow::Result<PathBuf> {
        let path = config_path(app_name);
        if path.exists() {
            return Ok(path);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, example_config_json()?)?;
        Ok(path)
    }

    /// Ruta del archivo de configuración en uso.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Configuración efectiva ya cargada.
    pub fn config(&self) -> &RootCauseConfig {
        &self.config
    }

    /// Serializa `config` al disco en `path`. Crea el directorio padre si falta.
    pub fn save_to_path(path: &Path, config: &RootCauseConfig) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(config)?;
        fs::write(path, json)?;
        Ok(())
    }
}

/// Ruta de la configuración: `~/Library/Application Support/<app>/…`.
pub fn config_path(app_name: &str) -> PathBuf {
    let resolved_app = if app_name.trim().is_empty() {
        DEFAULT_APP_DIR
    } else {
        app_name
    };
    dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(resolved_app)
        .join(DEFAULT_CONFIG_FILE)
}

/// Configuración por defecto serializada: es lo que escribe `config init`.
pub fn example_config_json() -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(&RootCauseConfig::default())?)
}

fn default_refresh_interval_secs() -> u64 {
    5
}

fn default_history_limit() -> usize {
    1_000
}

fn default_incident_limit() -> usize {
    300
}

fn default_signature_budget() -> usize {
    12
}

fn default_process_cpu_warning() -> f32 {
    30.0
}

fn default_process_cpu_critical() -> f32 {
    65.0
}

fn default_process_memory_warning() -> f32 {
    1_000.0
}

fn default_process_memory_critical() -> f32 {
    2_500.0
}

fn default_process_io_warning() -> f32 {
    40.0
}

fn default_process_io_critical() -> f32 {
    200.0
}

fn default_anomaly_cpu_sustained_percent() -> f32 {
    55.0
}

fn default_anomaly_cpu_sustained_samples() -> u8 {
    3
}

fn default_anomaly_memory_growth_mb() -> f32 {
    250.0
}

fn default_anomaly_memory_growth_samples() -> u8 {
    2
}

fn default_anomaly_aggressive_write_mb() -> f32 {
    120.0
}

fn default_anomaly_aggressive_write_samples() -> u8 {
    2
}

fn default_anomaly_public_destination_count() -> usize {
    4
}

fn default_anomaly_local_scan_destination_count() -> usize {
    8
}

fn default_anomaly_respawn_window_secs() -> u64 {
    180
}

fn default_anomaly_respawn_count() -> u8 {
    2
}

/// Rutas desde las que un binario en ejecución merece una segunda mirada.
/// En macOS lo normal es ejecutar desde `/Applications`, `/System` o `/usr`;
/// ejecutar desde `/tmp`, `/Users/Shared` o `~/Downloads` no lo es.
fn default_suspicious_path_keywords() -> Vec<String> {
    vec![
        "/tmp/".to_owned(),
        "/private/tmp/".to_owned(),
        "/var/tmp/".to_owned(),
        "/users/shared/".to_owned(),
        "/downloads/".to_owned(),
        "/.hidden".to_owned(),
        "/library/application support/.".to_owned(),
    ]
}

fn default_trusted_process_names() -> Vec<String> {
    vec![
        "kernel_task".to_owned(),
        "launchd".to_owned(),
        "windowserver".to_owned(),
        "mds".to_owned(),
        "mds_stores".to_owned(),
        "mdworker_shared".to_owned(),
        "kernelmanagerd".to_owned(),
        "backupd".to_owned(),
    ]
}

fn default_trusted_path_prefixes() -> Vec<String> {
    vec![
        "/system/".to_owned(),
        "/usr/".to_owned(),
        "/bin/".to_owned(),
        "/sbin/".to_owned(),
        "/applications/".to_owned(),
        "/library/apple/".to_owned(),
    ]
}

/// Padres que rara vez deberían lanzar binarios nuevos: si un shell o un
/// intérprete lanzado por una app de ofimática o un navegador arranca algo,
/// merece explicación.
fn default_suspicious_parent_names() -> Vec<String> {
    vec![
        "bash".to_owned(),
        "zsh".to_owned(),
        "sh".to_owned(),
        "osascript".to_owned(),
        "python3".to_owned(),
        "perl".to_owned(),
        "ruby".to_owned(),
        "curl".to_owned(),
        "microsoft word".to_owned(),
        "microsoft excel".to_owned(),
    ]
}

fn default_shell_interpreters() -> Vec<String> {
    vec![
        "bash".to_owned(),
        "zsh".to_owned(),
        "sh".to_owned(),
        "osascript".to_owned(),
        "python".to_owned(),
        "python3".to_owned(),
        "perl".to_owned(),
        "ruby".to_owned(),
        "node".to_owned(),
    ]
}

fn default_cache_warning() -> f32 {
    2_048.0
}

fn default_cache_critical() -> f32 {
    8_192.0
}

fn default_xprotect_warning_days() -> i64 {
    30
}

fn default_xprotect_critical_days() -> i64 {
    90
}

fn default_max_alerts() -> usize {
    8
}

fn default_notification_cooldown_secs() -> u64 {
    90
}

fn default_heartbeat_interval_secs() -> u64 {
    15
}

fn default_stale_after_secs() -> u64 {
    90
}

fn default_restart_window_secs() -> u64 {
    600
}

fn default_max_restarts_in_window() -> u8 {
    3
}

fn default_ai_timeout_secs() -> u64 {
    25
}

fn default_ai_model() -> String {
    "gpt-4.1-mini".to_owned()
}

fn default_ai_api_key_env_var() -> String {
    "ROOTCAUSE_AI_API_KEY".to_owned()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valores_por_defecto_son_razonables() {
        let cfg = RootCauseConfig::default();
        assert_eq!(cfg.collection.refresh_interval_secs, 5);
        assert!(
            cfg.thresholds.process.cpu_critical_percent
                > cfg.thresholds.process.cpu_warning_percent
        );
        assert!(cfg.thresholds.cache.critical_mb > cfg.thresholds.cache.warning_mb);
        assert!(cfg.thresholds.xprotect.critical_days > cfg.thresholds.xprotect.warning_days);
        assert!(cfg.anomaly.enabled);
        assert!(cfg.anomaly.cpu_sustained_samples >= 2);
        assert!(!cfg.anomaly.suspicious_path_keywords.is_empty());
        assert!(cfg.resilience.enabled);
        assert!(!cfg.ai.enabled);
        assert!(!cfg.remediation.automatic_actions_enabled);
    }

    #[test]
    fn la_config_viaja_a_json_y_vuelve() {
        let cfg = RootCauseConfig::default();
        let json = serde_json::to_string(&cfg).expect("serializa");
        let back: RootCauseConfig = serde_json::from_str(&json).expect("deserializa");
        assert_eq!(
            back.collection.refresh_interval_secs,
            cfg.collection.refresh_interval_secs
        );
        assert_eq!(back.ui.theme, ThemeMode::Dark);
    }

    #[test]
    fn json_vacio_cae_a_defaults_sin_romper() {
        let back: RootCauseConfig = serde_json::from_str("{}").expect("deserializa vacío");
        assert_eq!(back.collection.history_limit, 1_000);
        assert!(back.anomaly.watch_persistence);
    }
}
