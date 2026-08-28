# 05 · Referencia técnica

> Catálogo de consulta: tipos, funciones, constantes, comandos, rutas y códigos de error,
> con su firma real y su ubicación. Para entender *cómo* funcionan por dentro, ver
> [06 · Explicación profunda](06-deep-code-explanation.md).

---

## 1. Constantes del producto (`src/meta.rs`)

| Constante | Tipo | Valor | Usada por |
|---|---|---|---|
| `VERSION` | `&str` | `env!("CARGO_PKG_VERSION")` → `0.1.0` | CLI, GUI, reporte, título de ventana |
| `DISPLAY_NAME` | `&str` | `"RootCause macOS Inspector"` | CLI, GUI, reporte |
| `DESCRIPTION` | `&str` | Descripción larga del producto | Sección Acerca |
| `AUTHOR` | `&str` | `"Vladimir Acuña"` | Ayuda del CLI, Acerca |
| `GITHUB` | `&str` | URL del repositorio | Acerca |
| `GITHUB_WINDOWS` | `&str` | URL de la edición Windows | Acerca |
| `LICENSE` | `&str` | `"Apache License 2.0"` | Acerca |
| `BUNDLE_ID` | `&str` | `"dev.vladimiracuna.rootcause"` | Acerca; coincide con `Info.plist` |
| `APP_DIR` | `&str` | `"RootCauseInspector"` | Carpeta de datos del usuario |

## 2. Constantes internas relevantes

| Constante | Archivo | Valor | Significado |
|---|---|---|---|
| `DEFAULT_CONFIG_FILE` | `config.rs` | `"rootcause-config.json"` | Nombre del archivo de configuración |
| `DEFAULT_APP_DIR` | `config.rs` | `"RootCauseInspector"` | Respaldo si `app_name` llega vacío |
| `LAUNCH_DIRS` | `launchd.rs` | 5 entradas | Carpetas de launchd vigiladas |
| `SUSPICIOUS` | `launchd.rs` | 5 rutas | Rutas temporales o compartidas |
| `INTERPRETERS` | `launchd.rs` | 6 nombres | Intérpretes que elevan el riesgo de un plist |
| `REQUIRED_TOOLS` | `macos.rs` | 7 pares | Utilidades que el entorno declara |
| `DEFINITION_PATHS` | `security.rs` | 4 pares | Bundles de definiciones de Apple |
| `SENSITIVE_SERVICES` | `tcc.rs` | 9 servicios | Servicios TCC considerados sensibles |
| `OUI_TABLE` | `netscan.rs` | 26 prefijos | Fabricantes reconocidos por MAC |
| `CACHE_ROOTS` | `temp_scan.rs` | 7 raíces | Carpetas de caché medidas |
| `MAX_ENTRIES_PER_ROOT` | `temp_scan.rs` | `40_000` | Tope de entradas por raíz |
| `SUSPICIOUS_PATHS` | `rules.rs` | 5 rutas | Rutas que suman 24 puntos a un proceso |
| `INSTALLERS`, `BROWSERS`, `DEV` | `rules.rs` | 5 / 6 / 8 | Palabras clave de categorización |
| `WATCHED` | `inspector.rs` | 6 pares | Servicios de launchd que se muestran |
| `SECURITY_SURFACE`, `TCC_SURFACE` | `baseline.rs` | `SurfaceSpec` | Textos y riesgo por superficie |
| `NETWORK_SURFACE_ID` | `baseline.rs` | `"network-device"` | Clave de la baseline de red |
| `DARK`, `LIGHT` | `app.rs` | `Palette` | Las dos paletas de la interfaz |

## 3. Tipos de dominio (`src/models.rs`)

### 3.1 Enumeraciones

| Tipo | Variantes | Orden | Métodos |
|---|---|---|---|
| `Severity` | `Healthy`, `Warning`, `Critical` | Sí (`Ord`) | `label()` |
| `RiskLevel` | `Low`, `Medium`, `High`, `Critical` | Sí | `label()`, `to_severity()` |
| `AgentStatus` | `Healthy`, `Degraded`, `Recovered` | No | `label()` |
| `CodeSignature` | `Apple`, `DeveloperId`, `AdHoc`, `Unsigned`, `Unknown` | No | `label()`, `risk()` |
| `PersistenceChange` | `Unchanged`, `Added`, `Modified`, `Removed` | No | `label()`, `is_change()` |
| `PersistenceScope` | `UserAgent`, `GlobalAgent`, `GlobalDaemon`, `SystemApple`, `LoginItem`, `Cron`, `Other` | No | `label()`, `base_risk()` |

**Tabla de conversión `RiskLevel::to_severity`**

| `RiskLevel` | `Severity` |
|---|---|
| `Low` | `Healthy` |
| `Medium` | `Warning` |
| `High` | `Critical` |
| `Critical` | `Critical` |

**Tabla de riesgo base por ámbito (`PersistenceScope::base_risk`)**

| Ámbito | Puntos | Motivo |
|---|---:|---|
| `GlobalDaemon` | 26 | Corre como root al arrancar el equipo |
| `GlobalAgent` | 20 | Se ejecuta para cualquier usuario |
| `Cron` | 18 | Recurrente y poco visible |
| `UserAgent` | 16 | Solo el usuario que inicia sesión |
| `LoginItem` | 12 | Visible en Ajustes |
| `Other` | 10 | Origen no clasificado |
| `SystemApple` | 0 | Provisto por Apple y protegido por SIP |

**Tabla de riesgo por firma (`CodeSignature::risk`)**

| Firma | `Severity` |
|---|---|
| `Apple`, `DeveloperId` | `Healthy` |
| `AdHoc`, `Unknown` | `Warning` |
| `Unsigned` | `Critical` |

### 3.2 Estructuras principales

| Tipo | Campos destacados | Dónde se produce | Dónde se consume |
|---|---|---|---|
| `SystemSnapshot` | `collected_at`, `overview`, `alerts`, `agent_health`, `processes`, `caches`, `connections`, `network`, `events`, `services`, `persistence_entries`, `security_controls`, `xprotect`, `tcc`, `anomalies`, `incident` | `InspectorService::collect_snapshot` | GUI, CLI, JSON, reporte, SQLite |
| `SystemOverview` | `cpu_usage_percent`, `memory_used_gb`, `memory_total_gb`, deltas de red y E/S, `cache_total_mb`, `primary_severity`, `primary_reason` | `collect_snapshot` + `rules::build_alerts` | Semáforo y tarjetas de métricas |
| `ProcessInsight` | `pid`, `name`, `exe_path`, `parent_pid`, `user`, `cpu_percent`, `memory_mb`, deltas E/S, `status`, `category`, `severity`, `score`, `can_terminate`, `reasons`, `command_line`, `signature` | `collect_processes` | Tabla de procesos, heurísticas |
| `PersistenceEntry` | `entry_kind`, `location`, `name`, `command`, `scope`, `target_path`, `exists_on_disk`, `run_at_load`, `keep_alive`, `start_interval_secs`, `signature`, `severity`, `note`, `change_status` | `launchd::scan_persistence` | Sección Persistencia, baseline |
| `SecurityControl` | `id`, `name`, `status`, `enabled`, `severity`, `evidence`, `explanation`, `change_status` | `security::scan_controls` | Sección Seguridad, baseline |
| `TccPermission` | `service`, `service_label`, `client`, `decision`, `allowed`, `database`, `last_modified`, `severity`, `note`, `change_status` | `tcc::read_database` | Sección Privacidad, baseline |
| `ConnectionInsight` | `protocol`, `local_address`, `remote_address`, `state`, `pid`, `process_name`, `exe_path`, `user`, `severity`, `reason`, `is_public_remote`, `is_listening` | `network::parse_lsof_field_output` | Sección Conexiones, heurísticas |
| `NetworkDevice` | `ip`, `mac`, `hostname`, `vendor`, `state`, `interface`, `is_gateway`, `is_self`, `severity`, `reason`, `change_status` | `netscan::parse_arp_table` | Sección Red, baseline |
| `AnomalyEvent` | `event_id`, `detected_at`, `severity`, `score`, `status`, `kind`, `title`, contexto del proceso, `summary`, `root_cause_hypothesis`, `recommended_action`, `evidence` | `anomaly.rs`, `baseline.rs`, `netscan.rs` | Alertas e incidentes |
| `IncidentSummary` | `incident_id`, `fingerprint`, `collected_at`, `severity`, `kind`, `title`, `summary`, `root_cause_hypothesis`, `probable_causes`, `recommended_actions`, `evidence`, `risk_level`, `risk_score`, `anomaly_count`, `anomaly_types`, `anomaly_events`, `ai_advice` | `rules::derive_incident` | Tabla `incidents`, CLI, IA |
| `WatchedItem` | `key`, `value`, `label`, `detail`, `change_status` | Cada superficie vigilada | `baseline::diff_surface` |
| `AuditRecord` | `occurred_at`, `action`, `target`, `success`, `detail` | `InspectorService::audit` | Tabla `audit_log` |
| `AgentHealth` | `status`, `summary`, marcas de tiempo, `config_fingerprint`, `config_changed`, `unexpected_shutdown_detected`, `consecutive_unexpected_stops`, `notes` | `ResilienceMonitor` | Sección Resumen, reporte |

## 4. Funciones por módulo

### 4.1 `src/main.rs`

| Función | Firma | Efectos | Notas |
|---|---|---|---|
| `main` | `fn main()` | Lee `argv`, puede terminar el proceso | Despacha CLI o GUI |
| `launch_gui` | `fn launch_gui() -> eframe::Result<()>` | Abre una ventana nativa | Solo con feature `gui` |
| `rootcause_icon` | `fn rootcause_icon() -> eframe::egui::IconData` | Ninguno | Dibuja el radar 64 × 64 px a mano, sin decodificar recursos |

### 4.2 `src/i18n.rs`

| Función | Firma | Retorno | Riesgo al modificar |
|---|---|---|---|
| `Lang::code` | `fn code(self) -> &'static str` | `"es"` / `"en"` | Se serializa en la configuración |
| `Lang::native_name` | `fn native_name(self) -> &'static str` | `"Español"` / `"English"` | Solo presentación |
| `set_lang` | `fn set_lang(lang: Lang)` | — | Escribe un `AtomicU8` global |
| `current_lang` | `fn current_lang() -> Lang` | Idioma activo | Lectura relajada; barata por frame |
| `tr` | `fn tr(es: &'static str, en: &'static str) -> &'static str` | La variante activa | Cambiar la firma obliga a tocar toda la GUI |

### 4.3 `src/config.rs`

| Función | Firma | Errores | Efectos |
|---|---|---|---|
| `ConfigManager::load_or_default` | `fn load_or_default(app_name: &str) -> (Self, Option<String>)` | Nunca falla | Lee disco; devuelve advertencia si el JSON es inválido |
| `ConfigManager::write_default_if_missing` | `fn write_default_if_missing(app_name: &str) -> anyhow::Result<PathBuf>` | E/S | Crea directorio y archivo si faltan |
| `ConfigManager::path` | `fn path(&self) -> &Path` | — | — |
| `ConfigManager::config` | `fn config(&self) -> &RootCauseConfig` | — | — |
| `ConfigManager::save_to_path` | `fn save_to_path(path: &Path, config: &RootCauseConfig) -> anyhow::Result<()>` | E/S, serialización | Escribe JSON con formato |
| `config_path` | `fn config_path(app_name: &str) -> PathBuf` | — | Usa `dirs::data_local_dir` con respaldos |
| `example_config_json` | `fn example_config_json() -> anyhow::Result<String>` | Serialización | Base de `config init` |

### 4.4 `src/cli.rs`

| Función | Firma | Código de salida |
|---|---|---|
| `run` | `pub fn run(args: &[String]) -> i32` | `0`, `1` o `2` |
| `wants` | `fn wants(args: &[String], flag: &str) -> bool` | — |
| `flag_value` | `fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str>` | — |
| `first_number` | `fn first_number(args: &[String], default: usize) -> usize` | — |
| `service` | `fn service() -> Result<InspectorService, i32>` | `1` si falla |
| `print_json` | `fn print_json<T: Serialize>(value: &T) -> i32` | `0` o `1` |
| `truncate` | `fn truncate(value: &str, max: usize) -> String` | — |

### 4.5 `src/services/macos.rs`

| Función | Firma | Errores | Efectos secundarios |
|---|---|---|---|
| `run_capture` | `fn run_capture(program: &str, args: &[&str]) -> Result<String>` | Falla si el proceso no arranca o el código ≠ 0 | Lanza un proceso |
| `run_combined` | `fn run_combined(program: &str, args: &[&str]) -> Result<String>` | Solo si no arranca | Une `stdout` y `stderr` |
| `command_exists` | `fn command_exists(program: &str) -> bool` | — | Puede lanzar `which` |
| `sysctl` | `fn sysctl(key: &str) -> Option<String>` | — | Lanza `sysctl` |
| `product_version` / `product_name` / `build_version` | `fn … () -> String` | — | Lanzan `sw_vers` |
| `current_user` | `fn current_user() -> String` | — | `$USER` o `id -un` |
| `current_uid` | `fn current_uid() -> u32` | — | `id -u`; respaldo `501` |
| `is_root` | `fn is_root() -> bool` | — | — |
| `environment` | `fn environment() -> EnvironmentReport` | — | Comprueba 7 rutas |
| `EnvironmentReport::missing_tools` | `fn missing_tools(&self) -> Vec<&str>` | — | — |
| `process_details` | `fn process_details() -> HashMap<u32, ProcessDetail>` | Nunca; devuelve vacío | Un solo `ps -axo pid=,user=,command=` |
| `terminate_process` | `fn terminate_process(pid: u32) -> Result<String>` | Permiso, proceso inexistente | **Envía `SIGTERM`**; no escala a `SIGKILL` |
| `reveal_in_finder` | `fn reveal_in_finder(path: &str) -> Result<String>` | E/S | **Abre el Finder** |
| `notify` | `fn notify(title: &str, message: &str)` | Silencioso | Lanza `osascript`; escapa comillas |
| `code_signature` | `fn code_signature(path: &str) -> CodeSignature` | — | Lanza `codesign -dvv` |
| `classify_codesign_output` | `fn classify_codesign_output(output: &str) -> CodeSignature` | — | Función pura, probada |
| `lsof_connections` | `fn lsof_connections() -> Result<String>` | Si `lsof` falta o falla | Prueba `/usr/sbin` y `/usr/bin` |
| `arp_table` | `fn arp_table() -> Result<String>` | Si `arp` falla | — |
| `default_route` | `fn default_route() -> (String, String)` | — | `(interfaz, puerta de enlace)` |
| `interface_addresses` | `fn interface_addresses(interface: &str) -> (String, String)` | — | `(IPv4, MAC)` |
| `discovery_sweep` | `fn discovery_sweep(subnet_prefix: &str)` | — | **254 `ping` en serie**; ruidoso |
| `reverse_dns` | `fn reverse_dns(ip: &str) -> String` | — | `dscacheutil` |
| `security_log_events` | `fn security_log_events(minutes: u32, limit: usize) -> Result<Vec<(String, String, String)>>` | Si `log show` falla | **Costoso**: segundos |

### 4.6 `src/services/launchd.rs`

| Función | Firma | Notas |
|---|---|---|
| `scan_persistence` | `fn scan_persistence(include_apple: bool, verify_signatures: bool) -> Vec<PersistenceEntry>` | Recorre 3 carpetas (5 con `include_apple`) más `cron` |
| `login_items` | `fn login_items() -> Vec<PersistenceEntry>` | **Dispara el permiso de Automatización**; solo bajo petición explícita |
| `classify_entry` | `fn classify_entry(entry: &mut PersistenceEntry)` | Aplica las 7 señales y fija `severity` y `note` |
| `launchctl_jobs` | `fn launchctl_jobs() -> Vec<(Option<u32>, i32, String)>` | `(pid, estado, label)` |
| `loaded_labels` | `fn loaded_labels() -> HashSet<String>` | Base de la detección de SSH |

**Señales de `classify_entry` y su peso**

| # | Señal | Puntos |
|---|---|---:|
| 0 | Riesgo base del ámbito | 0–26 |
| 1 | Binario en ruta temporal o compartida | +30 |
| 2 | Binario oculto (nombre con punto inicial) | +25 |
| 3 | `Label` que imita a `com.apple.*` fuera del sistema | +35 |
| 4 | Binario sin firma / con firma ad-hoc | +30 / +20 |
| 5 | Apunta a un binario que no existe | +12 |
| 6 | `KeepAlive` activo / intervalo ≤ 60 s | +10 / +12 |
| 7 | Ejecuta un intérprete o una descarga | +18 |

Umbrales: `0–24` bajo · `25–54` medio · `55–84` alto · `85+` crítico.

### 4.7 `src/services/security.rs`

| Función | Firma | Comando consultado |
|---|---|---|
| `scan_controls` | `fn scan_controls() -> Vec<SecurityControl>` | Los seis siguientes |
| `gatekeeper` *(privada)* | — | `spctl --status` |
| `system_integrity_protection` *(privada)* | — | `csrutil status` |
| `filevault` *(privada)* | — | `fdesetup status` |
| `application_firewall` *(privada)* | — | `socketfilterfw --getglobalstate`, respaldo `defaults read com.apple.alf globalstate` |
| `firewall_stealth_mode` *(privada)* | — | `socketfilterfw --getstealthmode` |
| `remote_login` *(privada)* | — | `launchctl list` → `com.openssh.sshd` |
| `control_watch_items` | `fn control_watch_items(controls: &[SecurityControl]) -> Vec<WatchedItem>` | — |
| `scan_xprotect` | `fn scan_xprotect(thresholds: &XProtectThresholds, now: DateTime<Utc>) -> XProtectStatus` | Lee 4 `Info.plist` |

#### Severidad cuando un control está apagado

| Control | Severidad si está apagado |
|---|---|
| Gatekeeper | `Critical` |
| SIP | `Critical` |
| FileVault | `Warning` |
| Firewall de aplicaciones | `Warning` |
| Modo encubierto | `Healthy` (nunca sube) |
| SSH activo | `Warning` |
| Estado desconocido | `Warning` |

### 4.8 `src/services/tcc.rs`

| Función | Firma | Notas |
|---|---|---|
| `service_label` | `fn service_label(service: &str) -> &'static str` | Traduce 23 identificadores conocidos; el resto cae en «Otro servicio TCC» |
| `scan` | `fn scan() -> TccOverview` | Lee base de usuario y de sistema |
| `decode_decision` | `fn decode_decision(raw: i64, modern_schema: bool) -> (String, bool)` | `0` denegado · `2` permitido · `3` limitado |
| `is_sensitive` | `fn is_sensitive(service: &str) -> bool` | Nueve servicios |
| `permission_watch_items` | `fn permission_watch_items(overview: &TccOverview) -> Vec<WatchedItem>` | Solo sensibles y concedidos |

#### Servicios TCC considerados sensibles

`kTCCServiceSystemPolicyAllFiles`, `kTCCServiceAccessibility`, `kTCCServiceScreenCapture`,
`kTCCServiceListenEvent`, `kTCCServiceMicrophone`, `kTCCServiceCamera`,
`kTCCServicePostEvent`, `kTCCServiceDeveloperTool`, `kTCCServiceSystemPolicySysAdminFiles`.

### 4.9 `src/services/network.rs`

| Función | Firma | Pura |
|---|---|---|
| `parse_lsof_field_output` | `fn parse_lsof_field_output(output: &str, process_paths: &HashMap<u32, String>) -> Vec<ConnectionInsight>` | Sí |
| `classify_connection` | `fn classify_connection(is_listening: bool, is_public_remote: bool, local_address: &str, remote_address: &str) -> (Severity, String)` | Sí |
| `extract_ip` | `fn extract_ip(endpoint: &str) -> Option<String>` | Sí |
| `is_public_ip` | `fn is_public_ip(value: &str) -> bool` | Sí |
| `unique_public_remotes_by_pid` | `fn unique_public_remotes_by_pid(connections: &[ConnectionInsight]) -> HashMap<u32, Vec<String>>` | Sí |

**Rangos que `is_public_ip` considera no públicos:** privadas RFC 1918, loopback,
link-local, broadcast, multicast, no especificada, documentación, CGNAT `100.64.0.0/10`,
IPv6 `fc00::/7` y `fe80::/10`.

### 4.10 `src/services/netscan.rs`

| Función | Firma | Notas |
|---|---|---|
| `scan` | `fn scan(deep: bool, scanned_at: &str) -> NetworkScan` | Con `deep` hace barrido y DNS inverso |
| `parse_arp_table` | `fn parse_arp_table(raw: &str) -> Vec<NetworkDevice>` | Descarta `(incomplete)` y difusión |
| `normalize_mac` | `fn normalize_mac(mac: &str) -> String` | Rellena octetos a dos dígitos |
| `vendor_from_mac` | `fn vendor_from_mac(mac: &str) -> String` | 26 prefijos OUI |
| `subnet_prefix_of` | `fn subnet_prefix_of(ip: &str) -> String` | Solo IPv4 |
| `device_key` | `fn device_key(device: &NetworkDevice) -> String` | `mac:…` o `ip:…` |
| `classify_device` | `fn classify_device(device: &mut NetworkDevice)` | Puerta de enlace nueva → crítico |
| `device_watch_items` | `fn device_watch_items(devices: &[NetworkDevice]) -> Vec<WatchedItem>` | Excluye el propio equipo |
| `device_from_watch_item` | `fn device_from_watch_item(item: &WatchedItem) -> NetworkDevice` | Reconstruye desaparecidos |
| `new_device_event` | `fn new_device_event(detected_at: DateTime<Utc>, device: &NetworkDevice) -> Option<AnomalyEvent>` | `kind = "unknown-device"` |

### 4.11 `src/services/temp_scan.rs`

| Función | Firma | Efectos |
|---|---|---|
| `scan` | `fn scan(thresholds: &CacheThresholds) -> CacheOverview` | Solo lectura |
| `clean_user_caches` | `fn clean_user_caches(min_age_hours: u64, dry_run: bool) -> CacheCleanResult` | **Borra** si `dry_run = false` |
| `measure_directory` *(privada)* | `fn measure_directory(path: &Path, max_entries: usize) -> (f32, u64, bool)` | No sigue enlaces simbólicos |

### 4.12 `src/services/rules.rs`

| Función | Firma |
|---|---|
| `classify_process` | `fn classify_process(name: &str, exe_path: &str, cpu_percent: f32, memory_mb: f32, write_delta_mb: f32, signature: Option<CodeSignature>, thresholds: &ProcessThresholds) -> (Severity, u8, Vec<String>, String)` |
| `build_alerts` | `fn build_alerts(inputs: AlertBuildInputs<'_>, overview: &mut SystemOverview, max_alerts: usize) -> Vec<Alert>` |
| `derive_incident` | `fn derive_incident(snapshot: &SystemSnapshot) -> Option<IncidentSummary>` |

**Puntaje de `classify_process`**

| Señal | Puntos |
|---|---:|
| CPU ≥ crítico / ≥ aviso | +35 / +18 |
| Memoria ≥ crítico / ≥ aviso | +28 / +14 |
| Escritura ≥ crítico / ≥ aviso | +40 / +20 |
| Ruta temporal o compartida | +24 |
| Binario oculto | +20 |
| Sin firma / firma ad-hoc | +26 / +14 |
| Categoría «Instalador / actualizador» | +10 |

Umbrales: `0–24` `Healthy` · `25–54` `Warning` · `55+` `Critical`.

### 4.13 `src/services/anomaly.rs`

| Función | Firma |
|---|---|
| `AnomalyTracker::analyze` | `fn analyze(&mut self, input: DetectionInput<'_>) -> Vec<AnomalyEvent>` |
| `persistence_change_event` | `fn persistence_change_event(detected_at: DateTime<Utc>, entry: &PersistenceEntry) -> Option<AnomalyEvent>` |

**Catálogo de `kind` de anomalía**

| `kind` | Riesgo | Puntaje | Origen | Condición |
|---|---|---:|---|---|
| `sustained-cpu` | `Medium` | 55 | `anomaly.rs` | `cpu_sustained_samples` muestras seguidas sobre el umbral |
| `memory-growth` | `Medium` | 50 | `anomaly.rs` | Crecimiento sobre la línea base durante N muestras |
| `aggressive-write` | `High` | 70 | `anomaly.rs` | Escritura por encima del umbral en muestras consecutivas |
| `unusual-outbound` | `High` | 72 | `anomaly.rs` | ≥ 4 destinos públicos distintos desde un proceso no habitual |
| `local-scan` | `High` | 75 | `anomaly.rs` | ≥ 8 equipos privados distintos contactados |
| `suspicious-path` | `High` | 68 | `anomaly.rs` | Ruta con una palabra clave sospechosa |
| `unsigned-binary` | `Medium` | 58 | `anomaly.rs` | Binario sin firma fuera de rutas del sistema |
| `fast-respawn` | `High` | 66 | `anomaly.rs` | Cambio de PID N veces en la ventana |
| `persistence-change` | `High`/`Critical`/`Medium` | 78 / 74 / 42 | `anomaly.rs` | Entrada nueva, modificada o eliminada |
| `security-control-change` | `High` / `Medium` | 72 / 68 / 45 | `baseline.rs` | Control nuevo, cambiado o ausente |
| `tcc-permission-change` | `High` / `Medium` | 72 / 68 / 45 | `baseline.rs` | Permiso nuevo, cambiado o revocado |
| `unknown-device` | `Critical` / `Medium` | 92 / 48 | `netscan.rs` | Puerta de enlace con MAC nueva / equipo nuevo |

### 4.14 `src/services/baseline.rs`

| Función | Firma | Retorno |
|---|---|---|
| `diff_surface` | `fn diff_surface(store: &PersistenceStore, surface_id: &str, items: &mut Vec<WatchedItem>) -> bool` | `true` si había baseline previa |
| `surface_change_event` | `fn surface_change_event(detected_at: DateTime<Utc>, spec: &SurfaceSpec, item: &WatchedItem) -> Option<AnomalyEvent>` | `None` si no hay cambio |

### 4.15 `src/services/persistence.rs`

| Método | Firma | SQL |
|---|---|---|
| `PersistenceStore::new` | `fn new(app_name: &str) -> Result<Self>` | `CREATE TABLE IF NOT EXISTS` × 5 |
| `db_path` | `fn db_path(&self) -> &Path` | — |
| `persist_snapshot` | `fn persist_snapshot(&self, snapshot: &SystemSnapshot, history_limit: usize) -> Result<()>` | `INSERT` + `trim_table` |
| `persist_incident` | `fn persist_incident(&self, incident: &IncidentSummary, incident_limit: usize) -> Result<bool>` | Compara huella con el último; `INSERT` si difiere |
| `update_incident_ai` | `fn update_incident_ai(&self, incident_id: &str, advice: &AiIncidentAdvice) -> Result<()>` | `UPDATE … SET payload_json` |
| `load_recent` | `fn load_recent(&self, limit: usize) -> Result<Vec<SnapshotRow>>` | `SELECT … ORDER BY id DESC LIMIT ?` |
| `load_recent_incidents` | `fn load_recent_incidents(&self, limit: usize) -> Result<Vec<IncidentSummary>>` | Igual sobre `incidents` |
| `latest_incident` | `fn latest_incident(&self) -> Result<Option<IncidentSummary>>` | `LIMIT 1` |
| `record_audit` | `fn record_audit(&self, record: &AuditRecord) -> Result<()>` | `INSERT` |
| `load_recent_audits` | `fn load_recent_audits(&self, limit: usize) -> Result<Vec<AuditRecord>>` | `SELECT` |
| `export_history_backup` | `fn export_history_backup(&self, limit: usize) -> Result<PathBuf>` | Lee y escribe JSON |
| `latest_summary_line` | `fn latest_summary_line(&self) -> Result<Option<String>>` | `SELECT … LIMIT 1` |
| `export_path` | `fn export_path(&self) -> PathBuf` | — |
| `load_persistence_baseline` | `fn load_persistence_baseline(&self) -> Result<HashMap<String, PersistenceEntry>>` | `SELECT` |
| `replace_persistence_baseline` | `fn replace_persistence_baseline(&self, entries: &[PersistenceEntry]) -> Result<()>` | `DELETE` + `INSERT` en transacción |
| `load_baseline` | `fn load_baseline(&self, surface: &str) -> Result<HashMap<String, WatchedItem>>` | `SELECT … WHERE surface = ?` |
| `replace_baseline` | `fn replace_baseline(&self, surface: &str, items: &[WatchedItem]) -> Result<()>` | `DELETE` + `INSERT` en transacción |
| `persistence_entry_key` | `fn persistence_entry_key(entry: &PersistenceEntry) -> String` | Clave `kind␟location␟name` |

### 4.16 `src/services/resilience.rs`

| Método | Firma | Efectos |
|---|---|---|
| `ResilienceMonitor::new` | `fn new(app_name: &str, config_path: &Path, config: &ResilienceConfig) -> Result<Self>` | Lee y escribe el JSON de estado |
| `health` | `fn health(&self) -> &AgentHealth` | — |
| `startup_audits` | `fn startup_audits(&self) -> Vec<AuditRecord>` | — |
| `heartbeat` | `fn heartbeat(&mut self) -> Result<Vec<AuditRecord>>` | Escribe si pasó el intervalo |
| `shutdown` | `fn shutdown(&mut self) -> Result<AuditRecord>` | Marca cierre limpio |

### 4.17 `src/services/report.rs` y `src/services/ai.rs`

| Función | Firma | Notas |
|---|---|---|
| `reports_dir` | `fn reports_dir() -> PathBuf` | `~/Documents/RootCause/reports` |
| `build_report` | `fn build_report(snapshot: &SystemSnapshot, hardware: &HardwareInfo) -> String` | 11 secciones |
| `save_report` | `fn save_report(content: &str) -> std::io::Result<PathBuf>` | Nombre con fecha local |
| `AiAdvisor::new` | `fn new(config: AiConfig) -> Self` | No valida nada aún |
| `AiAdvisor::summarize_incident` | `fn summarize_incident(&self, incident: &IncidentSummary) -> Result<AiIncidentAdvice>` | **Única salida de red del producto** |
| `parse_response` | `fn parse_response(raw: &str, config: &AiConfig) -> Result<AiIncidentAdvice>` | Espera `choices[0].message.content` |

### 4.18 `src/services/inspector.rs`

Métodos públicos, agrupados por naturaleza:

| Grupo | Métodos |
|---|---|
| Construcción | `new` |
| Accesores | `config`, `config_path`, `db_path`, `environment`, `get_hardware_info`, `latest_incident`, `latest_history_line` |
| Lectura de historial | `load_history`, `load_incidents`, `load_audits` |
| Captura | `collect_snapshot`, `persistence_with_changes`, `scan_network`, `security_events`, `login_items` |
| Baselines | `accept_persistence_baseline`, `accept_security_baseline`, `accept_tcc_baseline`, `accept_network_baseline` |
| Acciones | `clean_caches`, `terminate_process`, `reveal_in_finder`, `suggest_block_ip` |
| Salidas | `generate_report`, `export_snapshot`, `export_history_backup` |
| Configuración | `save_config`, `write_default_config_if_missing` |
| IA | `explain_latest_incident_with_ai` |
| Notificación | `notify_if_critical` |

## 5. Comandos del CLI

| Comando | Banderas | Salida | Código |
|---|---|---|---|
| `--help`, `-h`, `help` | — | Ayuda completa | `0` |
| `--version`, `-V`, `version` | — | Nombre y versión | `0` |
| `status` | `--json` | Veredicto, controles, XProtect, persistencia, TCC, contexto, alertas | `0` / `1` |
| `snapshot` | `--json` implícito, `--output PATH` | Captura completa | `0` / `1` |
| `history` | `[N]`, `--json`, `--backup` | Últimas N capturas o ruta de la copia | `0` / `1` |
| `incidents` | `[N]`, `--json` | Incidentes persistidos | `0` / `1` |
| `audit` | `[N]`, `--json` | Acciones auditadas | `0` / `1` |
| `export` | — | Ruta del JSON exportado | `0` / `1` |
| `report` | — | Ruta del Markdown generado | `0` / `1` |
| `persistence` | `--json`, `--all`, `--login-items`, `--accept` | Entradas con estado vs baseline | `0` / `1` |
| `security` | `--json`, `--accept` | Seis controles con evidencia | `0` / `1` |
| `xprotect` | `--json` | Definiciones y antigüedad | `0` / `1` |
| `tcc` | `--json`, `--sensitive`, `--accept` | Permisos concedidos | `0` / `1` |
| `connections` | `--json` | Hasta 80 conexiones | `0` / `1` |
| `network` | `--json`, `--deep`, `--accept` | Vecinos del segmento | `0` / `1` |
| `events` | `--json`, `--minutes N` | Eventos del log unificado | `0` / `1` |
| `clean-caches` | `--yes` | Simulación o limpieza real | `0` |
| `kill <PID>` | — | Resultado del `SIGTERM` | `0` / `1` / `2` |
| `block-ip <IP>` | — | Regla `pfctl` sugerida | `0` / `1` / `2` |
| `config` | `show`, `init`, `--json` | Rutas y configuración | `0` / `1` / `2` |
| `ai` | `explain-latest`, `--json` | Consejo de la IA opcional | `0` / `1` / `2` |

### Códigos de salida

| Código | Significado |
|---|---|
| `0` | Ejecución correcta |
| `1` | Error de ejecución: motor no inicializable, captura fallida, permiso denegado, IA no disponible |
| `2` | Uso incorrecto: falta un argumento obligatorio o el subcomando no existe |

## 6. Atajos de teclado de la interfaz

| Atajo | Acción |
|---|---|
| `F5` | Actualizar (nueva captura) |
| `⌘E` | Exportar la captura a JSON |
| `⌘R` | Generar el reporte forense |

## 7. Rutas y archivos que el sistema usa

### 7.1 Escritura

| Ruta | Contenido |
|---|---|
| `~/Library/Application Support/RootCauseInspector/rootcause-history.db` | SQLite con las cinco tablas |
| `~/Library/Application Support/RootCauseInspector/rootcause-config.json` | Configuración |
| `~/Library/Application Support/RootCauseInspector/rootcause-agent-state.json` | Estado de resiliencia |
| `~/Library/Application Support/RootCauseInspector/rootcause-history-backup.json` | Copia del historial |
| `~/Downloads/rootcause-snapshot-<fecha>.json` | Exporte de captura (respaldo: Documentos) |
| `~/Documents/RootCause/reports/rootcause-report-<fecha>.md` | Reporte forense |

### 7.2 Lectura

| Ruta | Para qué |
|---|---|
| `~/Library/LaunchAgents`, `/Library/LaunchAgents`, `/Library/LaunchDaemons` | Persistencia (por defecto) |
| `/System/Library/LaunchAgents`, `/System/Library/LaunchDaemons` | Persistencia con `--all` |
| `~/Library/Application Support/com.apple.TCC/TCC.db` | Permisos del usuario |
| `/Library/Application Support/com.apple.TCC/TCC.db` | Permisos del sistema |
| `/Library/Apple/System/Library/CoreServices/XProtect.bundle/Contents/Info.plist` | Versión de XProtect |
| `/Library/Apple/System/Library/CoreServices/XProtect.app/Contents/Info.plist` | XProtect Remediator |
| `/Library/Apple/System/Library/CoreServices/MRT.app/Contents/Info.plist` | MRT |
| `/System/Library/CoreServices/XProtect.bundle/Contents/Info.plist` | XProtect heredado |
| `~/Library/Caches`, `~/Library/Logs`, `~/Library/Developer/Xcode/DerivedData`, caché de Safari, `/private/var/tmp`, `/Library/Caches`, `~/.Trash`, `$TMPDIR` | Almacenamiento |
| `/Library/Preferences/com.apple.alf` | Respaldo del estado del firewall |

### 7.3 Borrado

Solo una ruta, y con tres salvaguardas: `~/Library/Caches`, entradas no modificadas en las
últimas 24 h, saltando lo que esté en uso, y con `dry_run` por defecto en el CLI.

## 8. Variables de entorno

| Variable | Quién la lee | Efecto |
|---|---|---|
| `ROOTCAUSE_AI_API_KEY` *(nombre configurable)* | `ai.rs` vía `config.ai.api_key_env_var` | Clave del proveedor IA. Sin ella, `summarize_incident` falla con mensaje claro |
| `TMPDIR` | `temp_scan.rs` | Añade el directorio temporal de la sesión a las raíces medidas |
| `USER` | `macos.rs` | Nombre de usuario; respaldo `id -un` |
| `CARGO_TERM_COLOR` | GitHub Actions | Color en los logs de CI |

No hay archivo `.env` ni carga de variables desde disco.

## 9. Mensajes de error frecuentes y su origen

| Mensaje | Origen | Causa |
|---|---|---|
| `No se pudo inicializar RootCause: …` | `cli::service` | `InspectorService::new` falló (normalmente, carpeta de datos no accesible) |
| `No se pudo capturar el estado: …` | `cmd_status` | `collect_snapshot` devolvió error |
| `{program} devolvió error: {detail}` | `macos::run_capture` | La utilidad salió con código ≠ 0; `detail` viene de `stderr` |
| `No se pudo ejecutar {program}` | `macos::run_capture` | El binario no existe o no es ejecutable |
| `No se pueden leer los permisos TCC: concede Acceso total al disco a RootCause` | `accept_tcc_baseline` | `TccOverview::readable = false` |
| `Proceso protegido por política local` | `terminate_process` | Nombre en la lista de trece protegidos |
| `La aplicación no se permite finalizar a sí misma` | `terminate_process` | `pid == own_pid` |
| `Las acciones manuales están desactivadas por configuración` | `terminate_process` | `remediation.manual_actions_enabled = false` |
| `No se pudo extraer una IP de '…'` | `suggest_block_ip` | El argumento no contiene una IP reconocible |
| `La integración IA está desactivada en la configuración` | `AiAdvisor::summarize_incident` | `ai.enabled = false` |
| «Falta `ai.endpoint` en la configuración» | `AiAdvisor::summarize_incident` | Endpoint vacío |
| `No existe la variable de entorno … con la API key` | `AiAdvisor::summarize_incident` | Falta la variable declarada |
| «La respuesta IA no trae `choices[0].message.content`» | `parse_response` | El proveedor no es compatible con la forma esperada |
| `Configuración inválida en …. Se usan valores por defecto: …` | `ConfigManager::load_or_default` | JSON malformado; se convierte en alerta de la captura |
| `No se pudo guardar el historial SQLite: …` | `collect_snapshot` | Fallo de escritura; se añade alerta y la captura continúa |

---

**Siguiente lectura recomendada:** [06 · Explicación profunda del código](06-deep-code-explanation.md).
