//! Persistencia de macOS: LaunchAgents, LaunchDaemons, login items y `cron`.
//!
//! En Windows la persistencia vive sobre todo en el registro; en macOS vive en
//! archivos `.plist` repartidos por cinco carpetas conocidas. Ese es el terreno
//! que este módulo cubre:
//!
//! | Carpeta | Quién la ejecuta | Cuándo |
//! |---|---|---|
//! | `~/Library/LaunchAgents` | el usuario | al iniciar sesión |
//! | `/Library/LaunchAgents` | cualquier usuario | al iniciar sesión |
//! | `/Library/LaunchDaemons` | **root** | al arrancar el equipo |
//! | `/System/Library/Launch*` | Apple | protegido por SIP |
//!
//! Un implante que quiere sobrevivir a un reinicio casi siempre deja rastro en
//! una de las tres primeras. Por eso el escaneo por defecto las cubre y omite
//! las de Apple: son cientos de entradas inmutables que solo añadirían ruido a
//! la baseline (`include_apple` permite incluirlas cuando se quiere el inventario
//! completo).

use crate::models::{CodeSignature, PersistenceEntry, PersistenceScope, RiskLevel};
use crate::services::macos;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Carpeta vigilada y el ámbito que representa.
struct LaunchDir {
    path: &'static str,
    scope: PersistenceScope,
    kind: &'static str,
    /// `true` si la carpeta pertenece a Apple y se omite salvo petición expresa.
    apple: bool,
}

const LAUNCH_DIRS: &[LaunchDir] = &[
    LaunchDir {
        path: "~/Library/LaunchAgents",
        scope: PersistenceScope::UserAgent,
        kind: "LaunchAgent",
        apple: false,
    },
    LaunchDir {
        path: "/Library/LaunchAgents",
        scope: PersistenceScope::GlobalAgent,
        kind: "LaunchAgent",
        apple: false,
    },
    LaunchDir {
        path: "/Library/LaunchDaemons",
        scope: PersistenceScope::GlobalDaemon,
        kind: "LaunchDaemon",
        apple: false,
    },
    LaunchDir {
        path: "/System/Library/LaunchAgents",
        scope: PersistenceScope::SystemApple,
        kind: "LaunchAgent",
        apple: true,
    },
    LaunchDir {
        path: "/System/Library/LaunchDaemons",
        scope: PersistenceScope::SystemApple,
        kind: "LaunchDaemon",
        apple: true,
    },
];

/// Expande `~` usando el directorio del usuario actual.
fn expand_home(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => dirs::home_dir().unwrap_or_default().join(rest),
        None => PathBuf::from(path),
    }
}

/// Recorre las carpetas de launchd y devuelve todas las entradas de persistencia
/// encontradas, ya clasificadas por riesgo.
///
/// `verify_signatures` controla si se invoca `codesign` sobre cada binario
/// destino: da la señal más valiosa del conjunto, pero cuesta un proceso por
/// entrada, así que quien llama decide.
pub fn scan_persistence(include_apple: bool, verify_signatures: bool) -> Vec<PersistenceEntry> {
    let mut entries = Vec::new();

    for dir in LAUNCH_DIRS {
        if dir.apple && !include_apple {
            continue;
        }
        let base = expand_home(dir.path);
        let Ok(read_dir) = std::fs::read_dir(&base) else {
            continue;
        };

        for item in read_dir.flatten() {
            let path = item.path();
            if path.extension().and_then(|value| value.to_str()) != Some("plist") {
                continue;
            }
            if let Some(entry) = parse_launch_plist(&path, dir.scope, dir.kind, verify_signatures) {
                entries.push(entry);
            }
        }
    }

    entries.extend(scan_cron());
    entries.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| left.name.cmp(&right.name))
    });
    entries
}

/// Lee un `.plist` de launchd y lo convierte en una entrada de persistencia.
///
/// Devuelve `None` si el archivo no es un plist legible: un plist corrupto no
/// debe abortar el resto del escaneo.
fn parse_launch_plist(
    path: &Path,
    scope: PersistenceScope,
    kind: &str,
    verify_signatures: bool,
) -> Option<PersistenceEntry> {
    let value = plist::Value::from_file(path).ok()?;
    let dict = value.as_dictionary()?;

    let label = dict
        .get("Label")
        .and_then(|value| value.as_string())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("sin-label")
                .to_owned()
        });

    let mut argv: Vec<String> = Vec::new();
    if let Some(program) = dict.get("Program").and_then(|value| value.as_string()) {
        argv.push(program.to_owned());
    }
    if let Some(array) = dict
        .get("ProgramArguments")
        .and_then(|value| value.as_array())
    {
        let args: Vec<String> = array
            .iter()
            .filter_map(|item| item.as_string().map(str::to_owned))
            .collect();
        if argv.is_empty() {
            argv = args;
        } else if args.len() > 1 {
            // `Program` manda sobre argv[0]; el resto son argumentos reales.
            argv.extend(args.into_iter().skip(1));
        }
    }

    let command = argv.join(" ");
    let target_path = argv.first().cloned().filter(|value| !value.is_empty());
    let exists_on_disk = target_path
        .as_deref()
        .map(|value| Path::new(value).exists())
        .unwrap_or(false);

    let run_at_load = dict
        .get("RunAtLoad")
        .and_then(|value| value.as_boolean())
        .unwrap_or(false);
    // `KeepAlive` puede ser booleano o un diccionario de condiciones; ambas
    // formas significan "relánzame", que es lo que interesa marcar.
    let keep_alive = dict
        .get("KeepAlive")
        .map(|value| value.as_boolean().unwrap_or(true))
        .unwrap_or(false);
    let start_interval = dict
        .get("StartInterval")
        .and_then(|value| value.as_signed_integer())
        .filter(|value| *value > 0)
        .map(|value| value as u64);

    let signature = match (verify_signatures, target_path.as_deref()) {
        (true, Some(target)) if exists_on_disk => Some(macos::code_signature(target)),
        _ => None,
    };

    let mut entry = PersistenceEntry {
        entry_kind: kind.to_owned(),
        location: path.display().to_string(),
        name: label,
        command,
        scope,
        target_path,
        exists_on_disk,
        run_at_load,
        keep_alive,
        start_interval_secs: start_interval,
        signature,
        severity: RiskLevel::Low,
        note: String::new(),
        change_status: Default::default(),
    };
    classify_entry(&mut entry);
    Some(entry)
}

/// Tareas `cron` del usuario y scripts `periodic` de terceros.
///
/// `cron` sigue vivo en macOS aunque Apple empuje launchd, y sigue siendo un
/// sitio cómodo para esconder una tarea recurrente.
fn scan_cron() -> Vec<PersistenceEntry> {
    let mut entries = Vec::new();

    if let Ok(output) = macos::run_capture("/usr/bin/crontab", &["-l"]) {
        for (index, line) in output.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut entry = PersistenceEntry {
                entry_kind: "cron".to_owned(),
                location: format!("crontab -l (línea {})", index + 1),
                name: format!("cron-{}", index + 1),
                command: line.to_owned(),
                scope: PersistenceScope::Cron,
                target_path: None,
                exists_on_disk: true,
                run_at_load: false,
                keep_alive: false,
                start_interval_secs: None,
                signature: None,
                severity: RiskLevel::Low,
                note: "Tarea programada en el crontab del usuario".to_owned(),
                change_status: Default::default(),
            };
            classify_entry(&mut entry);
            entries.push(entry);
        }
    }

    entries
}

/// Consulta los login items vía `osascript`.
///
/// Se mantiene fuera del escaneo automático a propósito: la primera llamada
/// dispara el diálogo de permiso de Automatización de macOS, y una herramienta
/// de seguridad no debería provocar diálogos de permisos que el usuario no pidió.
/// La GUI y el CLI la invocan solo cuando alguien lo solicita explícitamente.
pub fn login_items() -> Vec<PersistenceEntry> {
    let script = "tell application \"System Events\" to get the name of every login item";
    let Ok(output) = macos::run_capture("/usr/bin/osascript", &["-e", script]) else {
        return Vec::new();
    };

    output
        .trim()
        .split(", ")
        .filter(|name| !name.trim().is_empty())
        .map(|name| {
            let mut entry = PersistenceEntry {
                entry_kind: "LoginItem".to_owned(),
                location: "System Events · Login Items".to_owned(),
                name: name.trim().to_owned(),
                command: name.trim().to_owned(),
                scope: PersistenceScope::LoginItem,
                target_path: None,
                exists_on_disk: true,
                run_at_load: true,
                keep_alive: false,
                start_interval_secs: None,
                signature: None,
                severity: RiskLevel::Low,
                note: "Elemento de inicio de sesión del usuario".to_owned(),
                change_status: Default::default(),
            };
            classify_entry(&mut entry);
            entry
        })
        .collect()
}

/// Asigna riesgo y nota explicativa a una entrada de persistencia.
///
/// El puntaje parte del ámbito (un daemon de root pesa más que un agente de
/// usuario) y suma señales acumulativas. Se expone públicamente para poder
/// probarla con entradas sintéticas sin tocar el disco.
pub fn classify_entry(entry: &mut PersistenceEntry) {
    let mut score = u16::from(entry.scope.base_risk());
    let mut notes: Vec<String> = Vec::new();

    let lower_command = entry.command.to_ascii_lowercase();
    let lower_target = entry
        .target_path
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    // Señal 1 — el binario vive en una ruta desde la que nadie instala software.
    const SUSPICIOUS: &[&str] = &[
        "/tmp/",
        "/private/tmp/",
        "/var/tmp/",
        "/users/shared/",
        "/downloads/",
    ];
    if SUSPICIOUS
        .iter()
        .any(|needle| lower_target.contains(needle) || lower_command.contains(needle))
    {
        score += 30;
        notes.push("Ejecuta un binario desde una ruta temporal o compartida".to_owned());
    }

    // Señal 2 — el binario está oculto (nombre con punto inicial).
    if entry
        .target_path
        .as_deref()
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
    {
        score += 25;
        notes.push("El binario destino está oculto (nombre con punto inicial)".to_owned());
    }

    // Señal 3 — se hace pasar por Apple fuera de las carpetas de Apple.
    if entry.name.to_ascii_lowercase().starts_with("com.apple.")
        && !matches!(entry.scope, PersistenceScope::SystemApple)
    {
        score += 35;
        notes.push(
            "El Label imita a Apple (`com.apple.*`) pero no vive en una carpeta del sistema"
                .to_owned(),
        );
    }

    // Señal 4 — firma de código ausente o ad-hoc.
    match entry.signature {
        Some(CodeSignature::Unsigned) => {
            score += 30;
            notes.push("El binario destino no está firmado".to_owned());
        }
        Some(CodeSignature::AdHoc) => {
            score += 20;
            notes.push(
                "El binario destino tiene firma ad-hoc (sin autoridad verificable)".to_owned(),
            );
        }
        _ => {}
    }

    // Señal 5 — apunta a algo que ya no existe.
    if entry.target_path.is_some() && !entry.exists_on_disk {
        score += 12;
        notes.push("Apunta a un binario que no existe en disco".to_owned());
    }

    // Señal 6 — se relanza solo o se repite muy seguido.
    if entry.keep_alive {
        score += 10;
        notes.push("KeepAlive activo: se relanza automáticamente si se cierra".to_owned());
    }
    if let Some(interval) = entry.start_interval_secs {
        if interval <= 60 {
            score += 12;
            notes.push(format!("Se ejecuta cada {interval} s"));
        }
    }

    // Señal 7 — lanza un intérprete en vez de un binario propio.
    const INTERPRETERS: &[&str] = &[
        "/bin/sh",
        "/bin/bash",
        "/bin/zsh",
        "osascript",
        "python",
        "curl",
    ];
    if INTERPRETERS
        .iter()
        .any(|needle| lower_command.contains(needle))
    {
        score += 18;
        notes.push("Ejecuta un intérprete o descarga en vez de un binario propio".to_owned());
    }

    entry.severity = match score {
        0..=24 => RiskLevel::Low,
        25..=54 => RiskLevel::Medium,
        55..=84 => RiskLevel::High,
        _ => RiskLevel::Critical,
    };

    if notes.is_empty() {
        if entry.note.is_empty() {
            entry.note = "Sin señales anómalas en esta entrada".to_owned();
        }
    } else {
        entry.note = notes.join(" · ");
    }
}

/// Servicios de launchd cargados en un dominio, según `launchctl list`.
///
/// La salida es `PID  Status  Label`; un `-` en PID significa cargado pero no
/// corriendo, y un status ≠ 0 es el último código de salida.
pub fn launchctl_jobs() -> Vec<(Option<u32>, i32, String)> {
    let Ok(output) = macos::run_capture("/bin/launchctl", &["list"]) else {
        return Vec::new();
    };

    let mut jobs = Vec::new();
    for line in output.lines().skip(1) {
        let mut parts = line.split_whitespace();
        let (Some(pid_text), Some(status_text), Some(label)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let pid = pid_text.parse::<u32>().ok();
        let status = status_text.parse::<i32>().unwrap_or(0);
        jobs.push((pid, status, label.to_owned()));
    }
    jobs
}

/// Etiquetas de launchd que están cargadas ahora mismo, para cruzar con los
/// plists encontrados en disco.
pub fn loaded_labels() -> HashSet<String> {
    launchctl_jobs()
        .into_iter()
        .map(|(_, _, label)| label)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_entry() -> PersistenceEntry {
        PersistenceEntry {
            entry_kind: "LaunchAgent".to_owned(),
            location: "/Library/LaunchAgents/com.example.updater.plist".to_owned(),
            name: "com.example.updater".to_owned(),
            command: "/Applications/Example.app/Contents/MacOS/updater".to_owned(),
            target_path: Some("/Applications/Example.app/Contents/MacOS/updater".to_owned()),
            scope: PersistenceScope::GlobalAgent,
            exists_on_disk: true,
            ..Default::default()
        }
    }

    #[test]
    fn agente_normal_de_aplicacion_es_riesgo_bajo_o_medio() {
        let mut entry = base_entry();
        classify_entry(&mut entry);
        assert!(entry.severity <= RiskLevel::Medium);
    }

    #[test]
    fn daemon_sin_firmar_en_tmp_es_critico() {
        let mut entry = base_entry();
        entry.scope = PersistenceScope::GlobalDaemon;
        entry.entry_kind = "LaunchDaemon".to_owned();
        entry.command = "/tmp/.helper --run".to_owned();
        entry.target_path = Some("/tmp/.helper".to_owned());
        entry.signature = Some(CodeSignature::Unsigned);
        classify_entry(&mut entry);
        assert_eq!(entry.severity, RiskLevel::Critical);
        assert!(entry.note.contains("temporal"));
        assert!(entry.note.contains("no está firmado"));
    }

    #[test]
    fn label_que_imita_a_apple_fuera_del_sistema_sube_el_riesgo() {
        let mut entry = base_entry();
        entry.name = "com.apple.softwareupdated".to_owned();
        classify_entry(&mut entry);
        assert!(entry.severity >= RiskLevel::Medium);
        assert!(entry.note.contains("imita a Apple"));
    }

    #[test]
    fn intervalo_muy_corto_se_reporta() {
        let mut entry = base_entry();
        entry.start_interval_secs = Some(30);
        classify_entry(&mut entry);
        assert!(entry.note.contains("cada 30 s"));
    }

    #[test]
    fn expand_home_resuelve_la_virgulilla() {
        let expanded = expand_home("~/Library/LaunchAgents");
        assert!(expanded.is_absolute());
        assert!(expanded.ends_with("Library/LaunchAgents"));
    }
}
