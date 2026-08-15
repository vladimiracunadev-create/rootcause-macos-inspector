//! Controles de seguridad nativos de macOS y estado antimalware de Apple.
//!
//! macOS trae una pila de defensas propias —Gatekeeper, SIP, FileVault, el
//! firewall de aplicaciones y las firmas de XProtect— que suelen darse por
//! activas sin comprobarlo nunca. Este módulo las consulta una por una y deja
//! la evidencia textual del comando que las respondió.
//!
//! El criterio de severidad es el mismo en todos los casos: **el estado seguro
//! es el que trae macOS de fábrica.** Que un control esté apagado no prueba una
//! intrusión —hay motivos legítimos para desactivar SIP en una máquina de
//! desarrollo—, pero sí es una superficie abierta que merece una explicación
//! consciente. RootCause la señala y explica; no la cambia.

use crate::config::XProtectThresholds;
use crate::models::{MalwareDefinition, SecurityControl, Severity, WatchedItem, XProtectStatus};
use crate::services::macos;
use chrono::{DateTime, Utc};
use std::path::Path;

/// Consulta todos los controles de seguridad y los devuelve ordenados: primero
/// lo que está apagado.
pub fn scan_controls() -> Vec<SecurityControl> {
    let mut controls = vec![
        gatekeeper(),
        system_integrity_protection(),
        filevault(),
        application_firewall(),
        firewall_stealth_mode(),
        remote_login(),
    ];
    controls.sort_by(|left, right| right.severity.cmp(&left.severity));
    controls
}

/// Gatekeeper: decide si macOS ejecuta software descargado sin verificar.
fn gatekeeper() -> SecurityControl {
    let output = macos::run_combined("/usr/sbin/spctl", &["--status"]).unwrap_or_default();
    let lower = output.to_ascii_lowercase();
    let enabled = lower.contains("assessments enabled");
    let known = enabled || lower.contains("assessments disabled");

    SecurityControl {
        id: "gatekeeper".to_owned(),
        name: "Gatekeeper".to_owned(),
        status: status_text(enabled, known),
        enabled,
        severity: severity_for(enabled, known, Severity::Critical),
        evidence: first_line(&output, "spctl --status"),
        explanation: if enabled {
            "Gatekeeper verifica la firma y la notarización del software antes de ejecutarlo por \
             primera vez."
                .to_owned()
        } else {
            "Gatekeeper está desactivado: macOS ejecutará binarios descargados sin comprobar firma \
             ni notarización. Se reactiva con `sudo spctl --master-enable`."
                .to_owned()
        },
        change_status: Default::default(),
    }
}

/// System Integrity Protection: protege `/System`, `/usr` y los procesos de Apple.
fn system_integrity_protection() -> SecurityControl {
    let output = macos::run_combined("/usr/bin/csrutil", &["status"]).unwrap_or_default();
    let lower = output.to_ascii_lowercase();
    let enabled = lower.contains("status: enabled");
    let known = enabled || lower.contains("status: disabled") || lower.contains("custom");

    SecurityControl {
        id: "sip".to_owned(),
        name: "System Integrity Protection (SIP)".to_owned(),
        status: status_text(enabled, known),
        enabled,
        severity: severity_for(enabled, known, Severity::Critical),
        evidence: first_line(&output, "csrutil status"),
        explanation: if enabled {
            "SIP impide modificar archivos y procesos del sistema incluso con root.".to_owned()
        } else {
            "SIP está desactivado: un proceso con root puede modificar el sistema y los binarios de \
             Apple. Solo se desactiva desde recoveryOS, así que fue una acción deliberada."
                .to_owned()
        },
        change_status: Default::default(),
    }
}

/// FileVault: cifrado del disco de arranque.
fn filevault() -> SecurityControl {
    let output = macos::run_combined("/usr/bin/fdesetup", &["status"]).unwrap_or_default();
    let lower = output.to_ascii_lowercase();
    let enabled = lower.contains("filevault is on");
    let known = enabled || lower.contains("filevault is off");

    SecurityControl {
        id: "filevault".to_owned(),
        name: "FileVault".to_owned(),
        status: status_text(enabled, known),
        enabled,
        severity: severity_for(enabled, known, Severity::Warning),
        evidence: first_line(&output, "fdesetup status"),
        explanation: if enabled {
            "El disco de arranque está cifrado: sin la contraseña, los datos no se pueden leer."
                .to_owned()
        } else {
            "FileVault está desactivado: cualquiera con acceso físico al equipo puede leer el disco \
             arrancando desde otro sistema."
                .to_owned()
        },
        change_status: Default::default(),
    }
}

/// Firewall de aplicaciones (`socketfilterfw`), con respaldo por `defaults`.
fn application_firewall() -> SecurityControl {
    let mut evidence = String::new();
    let mut enabled = false;
    let mut known = false;

    if let Ok(output) = macos::run_combined(
        "/usr/libexec/ApplicationFirewall/socketfilterfw",
        &["--getglobalstate"],
    ) {
        let lower = output.to_ascii_lowercase();
        if lower.contains("state = 1") || lower.contains("state = 2") {
            enabled = true;
            known = true;
        } else if lower.contains("state = 0") {
            known = true;
        }
        evidence = first_line(&output, "socketfilterfw --getglobalstate");
    }

    // Respaldo: la preferencia cruda sigue siendo legible sin privilegios.
    if !known {
        if let Ok(output) = macos::run_capture(
            "/usr/bin/defaults",
            &["read", "/Library/Preferences/com.apple.alf", "globalstate"],
        ) {
            let value = output.trim();
            enabled = value == "1" || value == "2";
            known = !value.is_empty();
            evidence = format!("com.apple.alf globalstate = {value}");
        }
    }

    SecurityControl {
        id: "firewall".to_owned(),
        name: "Firewall de aplicaciones".to_owned(),
        status: status_text(enabled, known),
        enabled,
        severity: severity_for(enabled, known, Severity::Warning),
        evidence,
        explanation: if enabled {
            "El firewall filtra las conexiones entrantes por aplicación.".to_owned()
        } else {
            "El firewall de aplicaciones está apagado: cualquier servicio a la escucha en este \
             equipo es alcanzable desde la red local."
                .to_owned()
        },
        change_status: Default::default(),
    }
}

/// Modo encubierto: no responder a pings ni a sondeos de puertos cerrados.
fn firewall_stealth_mode() -> SecurityControl {
    let output = macos::run_combined(
        "/usr/libexec/ApplicationFirewall/socketfilterfw",
        &["--getstealthmode"],
    )
    .unwrap_or_default();
    let lower = output.to_ascii_lowercase();
    // La redacción cambia entre versiones de macOS: unas dicen "stealth mode
    // enabled" y otras "stealth mode is on". Se aceptan ambas.
    let enabled = lower.contains("stealth mode enabled") || lower.contains("stealth mode is on");
    let known =
        enabled || lower.contains("stealth mode disabled") || lower.contains("stealth mode is off");

    SecurityControl {
        id: "stealth-mode".to_owned(),
        name: "Modo encubierto del firewall".to_owned(),
        status: status_text(enabled, known),
        enabled,
        // Apagado es el valor de fábrica de macOS: se informa, no se alarma.
        // Es el único control de la lista que nunca sube de verde.
        severity: Severity::Healthy,
        evidence: first_line(&output, "socketfilterfw --getstealthmode"),
        explanation:
            "Con el modo encubierto activo el equipo no responde a pings ni a sondeos de puertos \
             cerrados, así que es más difícil de descubrir en un barrido de red."
                .to_owned(),
        change_status: Default::default(),
    }
}

/// Acceso remoto por SSH: se deduce de si `com.openssh.sshd` está cargado en
/// launchd, porque `systemsetup -getremotelogin` exige privilegios de root.
fn remote_login() -> SecurityControl {
    let labels = crate::services::launchd::loaded_labels();
    let enabled = labels
        .iter()
        .any(|label| label.contains("com.openssh.sshd"));

    SecurityControl {
        id: "remote-login".to_owned(),
        name: "Acceso remoto (SSH)".to_owned(),
        // Aquí "enabled" significa "el servicio está activo", que es lo INSEGURO.
        status: if enabled {
            "Activado".to_owned()
        } else {
            "Desactivado".to_owned()
        },
        enabled: !enabled,
        severity: if enabled {
            Severity::Warning
        } else {
            Severity::Healthy
        },
        evidence: if enabled {
            "launchctl list contiene com.openssh.sshd".to_owned()
        } else {
            "com.openssh.sshd no está cargado".to_owned()
        },
        explanation: if enabled {
            "El acceso remoto por SSH está habilitado: este equipo acepta sesiones desde la red. \
             Confirma que es intencional en Ajustes → General → Compartir."
                .to_owned()
        } else {
            "El acceso remoto por SSH está deshabilitado.".to_owned()
        },
        change_status: Default::default(),
    }
}

fn status_text(enabled: bool, known: bool) -> String {
    if !known {
        "Desconocido".to_owned()
    } else if enabled {
        "Activado".to_owned()
    } else {
        "Desactivado".to_owned()
    }
}

/// Un control apagado toma la severidad declarada; uno indeterminado avisa en
/// amarillo, porque "no lo sé" nunca debe pintarse de verde.
fn severity_for(enabled: bool, known: bool, when_disabled: Severity) -> Severity {
    match (known, enabled) {
        (true, true) => Severity::Healthy,
        (true, false) => when_disabled,
        (false, _) => Severity::Warning,
    }
}

fn first_line(output: &str, fallback: &str) -> String {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(fallback)
        .to_owned()
}

/// Convierte los controles en ítems vigilables por el motor de baseline: si
/// Gatekeeper pasa de activo a inactivo entre dos capturas, eso es un cambio
/// que RootCause debe reportar aunque el usuario no estuviera mirando.
pub fn control_watch_items(controls: &[SecurityControl]) -> Vec<WatchedItem> {
    controls
        .iter()
        .map(|control| WatchedItem {
            key: control.id.clone(),
            value: control.status.clone(),
            label: control.name.clone(),
            detail: control.evidence.clone(),
            change_status: Default::default(),
        })
        .collect()
}

// ── XProtect y familia antimalware ──────────────────────────────────────────

/// Rutas de las definiciones antimalware de Apple, en orden de preferencia.
const DEFINITION_PATHS: &[(&str, &str)] = &[
    (
        "XProtect",
        "/Library/Apple/System/Library/CoreServices/XProtect.bundle/Contents/Info.plist",
    ),
    (
        "XProtect Remediator",
        "/Library/Apple/System/Library/CoreServices/XProtect.app/Contents/Info.plist",
    ),
    (
        "MRT",
        "/Library/Apple/System/Library/CoreServices/MRT.app/Contents/Info.plist",
    ),
    (
        "XProtect (legacy)",
        "/System/Library/CoreServices/XProtect.bundle/Contents/Info.plist",
    ),
];

/// Estado del subsistema antimalware de Apple: qué versión de firmas hay y qué
/// antigüedad tiene.
///
/// Apple publica firmas de XProtect con frecuencia y las instala en silencio.
/// Una definición de hace meses casi siempre significa que las actualizaciones
/// automáticas están rotas o desactivadas — un hallazgo por sí mismo.
pub fn scan_xprotect(thresholds: &XProtectThresholds, now: DateTime<Utc>) -> XProtectStatus {
    let mut definitions = Vec::new();
    let mut seen_versions = Vec::new();

    for (component, path) in DEFINITION_PATHS {
        let Some(mut definition) = read_definition(component, path, now) else {
            continue;
        };
        // Evita listar dos veces la misma versión cuando conviven la ruta nueva
        // (`/Library/Apple`) y la heredada (`/System`).
        let dedupe_key = format!("{}::{}", definition.component, definition.version);
        if seen_versions.contains(&dedupe_key) {
            continue;
        }
        seen_versions.push(dedupe_key);
        definition.severity = age_severity(definition.age_days, thresholds);
        definition.note = age_note(definition.age_days, thresholds);
        definitions.push(definition);
    }

    let freshest_age_days = definitions
        .iter()
        .map(|definition| definition.age_days)
        .min()
        .unwrap_or(i64::MAX);

    let (severity, headline) = if definitions.is_empty() {
        (
            Severity::Warning,
            "No se pudieron leer las definiciones de XProtect en este equipo".to_owned(),
        )
    } else {
        let severity = age_severity(freshest_age_days, thresholds);
        let headline = match severity {
            Severity::Healthy => format!(
                "Definiciones al día: la más reciente tiene {freshest_age_days} día(s)"
            ),
            Severity::Warning => format!(
                "Definiciones con {freshest_age_days} días de antigüedad; revisa las actualizaciones automáticas"
            ),
            Severity::Critical => format!(
                "Definiciones desactualizadas ({freshest_age_days} días): las actualizaciones automáticas parecen rotas"
            ),
        };
        (severity, headline)
    };

    XProtectStatus {
        definitions,
        freshest_age_days: if freshest_age_days == i64::MAX {
            -1
        } else {
            freshest_age_days
        },
        severity,
        headline,
        limitations: vec![
            "XProtect solo compara contra firmas conocidas de Apple; no detecta amenazas nuevas."
                .to_owned(),
            "La antigüedad se mide por la fecha del bundle en disco, no por la fecha de publicación de Apple."
                .to_owned(),
        ],
    }
}

/// Lee la versión y la fecha de un bundle de definiciones.
fn read_definition(component: &str, path: &str, now: DateTime<Utc>) -> Option<MalwareDefinition> {
    let plist_path = Path::new(path);
    if !plist_path.exists() {
        return None;
    }

    let value = plist::Value::from_file(plist_path).ok()?;
    let dict = value.as_dictionary()?;
    let version = dict
        .get("CFBundleShortVersionString")
        .and_then(|value| value.as_string())
        .or_else(|| {
            dict.get("CFBundleVersion")
                .and_then(|value| value.as_string())
        })
        .unwrap_or("desconocida")
        .to_owned();

    let modified = plist_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .map(DateTime::<Utc>::from);
    let last_modified = modified.map(|value| value.to_rfc3339()).unwrap_or_default();
    let age_days = modified
        .map(|value| (now - value).num_days().max(0))
        .unwrap_or(-1);

    Some(MalwareDefinition {
        component: component.to_owned(),
        version,
        last_modified,
        age_days,
        severity: Severity::Healthy,
        path: path.to_owned(),
        note: String::new(),
    })
}

fn age_severity(age_days: i64, thresholds: &XProtectThresholds) -> Severity {
    if age_days < 0 {
        Severity::Warning
    } else if age_days >= thresholds.critical_days {
        Severity::Critical
    } else if age_days >= thresholds.warning_days {
        Severity::Warning
    } else {
        Severity::Healthy
    }
}

fn age_note(age_days: i64, thresholds: &XProtectThresholds) -> String {
    if age_days < 0 {
        "No se pudo determinar la fecha de la definición".to_owned()
    } else if age_days >= thresholds.critical_days {
        format!("Sin actualizarse hace {age_days} días")
    } else if age_days >= thresholds.warning_days {
        format!("Última actualización hace {age_days} días")
    } else {
        format!("Actualizada hace {age_days} día(s)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_desconocido_no_se_pinta_de_verde() {
        assert_eq!(
            severity_for(false, false, Severity::Critical),
            Severity::Warning
        );
        assert_eq!(status_text(false, false), "Desconocido");
    }

    #[test]
    fn control_apagado_toma_la_severidad_declarada() {
        assert_eq!(
            severity_for(false, true, Severity::Critical),
            Severity::Critical
        );
        assert_eq!(
            severity_for(true, true, Severity::Critical),
            Severity::Healthy
        );
    }

    #[test]
    fn antiguedad_de_firmas_escala_con_los_umbrales() {
        let thresholds = XProtectThresholds {
            warning_days: 30,
            critical_days: 90,
        };
        assert_eq!(age_severity(3, &thresholds), Severity::Healthy);
        assert_eq!(age_severity(45, &thresholds), Severity::Warning);
        assert_eq!(age_severity(120, &thresholds), Severity::Critical);
        assert_eq!(age_severity(-1, &thresholds), Severity::Warning);
    }

    #[test]
    fn los_controles_se_convierten_en_items_vigilables() {
        let controls = vec![SecurityControl {
            id: "gatekeeper".to_owned(),
            name: "Gatekeeper".to_owned(),
            status: "Activado".to_owned(),
            enabled: true,
            severity: Severity::Healthy,
            evidence: "assessments enabled".to_owned(),
            explanation: String::new(),
            change_status: Default::default(),
        }];
        let items = control_watch_items(&controls);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key, "gatekeeper");
        assert_eq!(items[0].value, "Activado");
    }

    #[test]
    fn primera_linea_util_o_respaldo() {
        assert_eq!(first_line("\n\n  hola \nmundo", "fb"), "hola");
        assert_eq!(first_line("   ", "fb"), "fb");
    }
}
