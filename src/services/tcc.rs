//! Permisos de privacidad (TCC — Transparency, Consent and Control).
//!
//! TCC es la base de datos donde macOS anota qué aplicación obtuvo qué permiso:
//! micrófono, cámara, grabación de pantalla, accesibilidad, control de otras
//! apps y —el más delicado— **acceso total al disco**. Para un atacante es un
//! objetivo directo: con Accesibilidad puede simular pulsaciones de teclado, y
//! con Acceso total al disco puede leer correo, mensajes y las propias bases TCC.
//!
//! Hay una ironía inevitable aquí: **leer TCC.db exige Acceso total al disco**.
//! Si RootCause no lo tiene, no inventa datos: devuelve `readable = false` y
//! explica el permiso que falta y dónde concederlo.

use crate::models::{Severity, TccOverview, TccPermission, WatchedItem};
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;

/// Servicios TCC que dan control real sobre el equipo o sobre datos personales.
/// Un permiso concedido aquí merece revisión explícita; el resto es contexto.
const SENSITIVE_SERVICES: &[&str] = &[
    "kTCCServiceSystemPolicyAllFiles",
    "kTCCServiceAccessibility",
    "kTCCServiceScreenCapture",
    "kTCCServiceListenEvent",
    "kTCCServiceMicrophone",
    "kTCCServiceCamera",
    "kTCCServicePostEvent",
    "kTCCServiceDeveloperTool",
    "kTCCServiceSystemPolicySysAdminFiles",
];

/// Traduce el identificador interno de TCC al nombre que muestra Ajustes.
pub fn service_label(service: &str) -> &'static str {
    match service {
        "kTCCServiceSystemPolicyAllFiles" => "Acceso total al disco",
        "kTCCServiceSystemPolicyDesktopFolder" => "Carpeta Escritorio",
        "kTCCServiceSystemPolicyDocumentsFolder" => "Carpeta Documentos",
        "kTCCServiceSystemPolicyDownloadsFolder" => "Carpeta Descargas",
        "kTCCServiceSystemPolicySysAdminFiles" => "Administración del sistema",
        "kTCCServiceSystemPolicyRemovableVolumes" => "Volúmenes extraíbles",
        "kTCCServiceSystemPolicyNetworkVolumes" => "Volúmenes de red",
        "kTCCServiceAccessibility" => "Accesibilidad (control del equipo)",
        "kTCCServiceScreenCapture" => "Grabación de pantalla",
        "kTCCServiceListenEvent" => "Monitorización de entrada (teclado)",
        "kTCCServicePostEvent" => "Envío de eventos de teclado/ratón",
        "kTCCServiceMicrophone" => "Micrófono",
        "kTCCServiceCamera" => "Cámara",
        "kTCCServiceAppleEvents" => "Control de otras apps (Automatización)",
        "kTCCServiceDeveloperTool" => "Herramientas de desarrollo",
        "kTCCServiceContactsFull" | "kTCCServiceAddressBook" => "Contactos",
        "kTCCServiceCalendar" => "Calendario",
        "kTCCServiceReminders" => "Recordatorios",
        "kTCCServicePhotos" => "Fotos",
        "kTCCServiceLocation" => "Ubicación",
        "kTCCServiceBluetoothAlways" => "Bluetooth",
        "kTCCServiceUbiquity" => "iCloud Drive",
        other => {
            // Sin traducción conocida: se devuelve el identificador crudo para no
            // perder información. Es preferible un nombre técnico a uno inventado.
            debug_assert!(other.starts_with("kTCCService"));
            "Otro servicio TCC"
        }
    }
}

/// Ruta de la base TCC del usuario actual.
fn user_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/Application Support/com.apple.TCC/TCC.db")
}

/// Ruta de la base TCC del sistema (permisos concedidos para todos los usuarios).
fn system_db_path() -> PathBuf {
    PathBuf::from("/Library/Application Support/com.apple.TCC/TCC.db")
}

/// Lee ambas bases TCC y devuelve el inventario de permisos concedidos.
pub fn scan() -> TccOverview {
    let mut permissions = Vec::new();
    let mut limitations = Vec::new();
    let mut readable = false;

    let user_result = read_database(&user_db_path(), "usuario");
    let full_disk_access = user_result.is_ok();
    match user_result {
        Ok(mut rows) => {
            readable = true;
            permissions.append(&mut rows);
        }
        Err(error) => limitations.push(format!(
            "No se pudo leer la base TCC del usuario ({error}). RootCause necesita Acceso total al \
             disco: Ajustes del Sistema → Privacidad y seguridad → Acceso total al disco."
        )),
    }

    match read_database(&system_db_path(), "sistema") {
        Ok(mut rows) => {
            readable = true;
            permissions.append(&mut rows);
        }
        Err(error) => limitations.push(format!(
            "No se pudo leer la base TCC del sistema ({error}). Suele requerir Acceso total al \
             disco y, en algunas versiones, privilegios de administrador."
        )),
    }

    permissions.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| left.service_label.cmp(&right.service_label))
            .then_with(|| left.client.cmp(&right.client))
    });

    let sensitive_count = permissions
        .iter()
        .filter(|permission| permission.allowed && is_sensitive(&permission.service))
        .count();

    let headline = if !readable {
        "Permisos TCC no legibles: falta Acceso total al disco para RootCause".to_owned()
    } else if sensitive_count == 0 {
        "Ninguna app tiene permisos sensibles concedidos".to_owned()
    } else {
        format!("{sensitive_count} app(s) con permisos sensibles concedidos")
    };

    limitations.push(
        "TCC registra el permiso, no su uso: que una app tenga acceso al micrófono no significa \
         que lo esté usando ahora."
            .to_owned(),
    );

    TccOverview {
        permissions,
        readable,
        full_disk_access,
        headline,
        sensitive_count,
        limitations,
    }
}

/// Abre una base TCC en solo lectura y extrae la tabla `access`.
///
/// El esquema cambió entre versiones de macOS: hasta High Sierra la columna era
/// `allowed`; desde Mojave es `auth_value`. Se detecta con `PRAGMA table_info`
/// en vez de asumir una versión concreta.
fn read_database(path: &PathBuf, database: &str) -> anyhow::Result<Vec<TccPermission>> {
    if !path.exists() {
        anyhow::bail!("no existe {}", path.display());
    }

    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let columns = table_columns(&connection, "access")?;
    let has_auth_value = columns.iter().any(|column| column == "auth_value");
    let has_last_modified = columns.iter().any(|column| column == "last_modified");

    let decision_column = if has_auth_value {
        "auth_value"
    } else {
        "allowed"
    };
    let modified_column = if has_last_modified {
        "last_modified"
    } else {
        "0"
    };
    let query = format!("SELECT service, client, {decision_column}, {modified_column} FROM access");

    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map([], |row| {
        let service: String = row.get(0)?;
        let client: String = row.get(1)?;
        let raw_decision: i64 = row.get(2).unwrap_or(0);
        let modified: i64 = row.get(3).unwrap_or(0);
        Ok((service, client, raw_decision, modified))
    })?;

    let mut permissions = Vec::new();
    for row in rows {
        let (service, client, raw_decision, modified) = row?;
        let (decision, allowed) = decode_decision(raw_decision, has_auth_value);
        let severity = severity_for(&service, allowed);
        permissions.push(TccPermission {
            service_label: service_label(&service).to_owned(),
            note: note_for(&service, allowed),
            service,
            client,
            decision,
            allowed,
            database: database.to_owned(),
            last_modified: format_epoch(modified),
            severity,
            change_status: Default::default(),
        });
    }
    Ok(permissions)
}

/// Nombres de columna de una tabla, para tolerar esquemas de distintas versiones.
fn table_columns(connection: &Connection, table: &str) -> anyhow::Result<Vec<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    let mut columns = Vec::new();
    for row in rows {
        columns.push(row?);
    }
    Ok(columns)
}

/// Traduce el valor crudo de la decisión a `(texto, concedido)`.
///
/// En el esquema moderno: 0 = denegado, 1 = desconocido, 2 = permitido,
/// 3 = limitado. En el heredado, `allowed` es un booleano.
pub fn decode_decision(raw: i64, modern_schema: bool) -> (String, bool) {
    if modern_schema {
        match raw {
            0 => ("denegado".to_owned(), false),
            2 => ("permitido".to_owned(), true),
            3 => ("limitado".to_owned(), true),
            _ => ("desconocido".to_owned(), false),
        }
    } else if raw != 0 {
        ("permitido".to_owned(), true)
    } else {
        ("denegado".to_owned(), false)
    }
}

/// `true` si el servicio da control real sobre el equipo o sobre datos privados.
pub fn is_sensitive(service: &str) -> bool {
    SENSITIVE_SERVICES.contains(&service)
}

/// Un permiso sensible concedido se marca en amarillo (revisar), no en rojo:
/// muchas apps legítimas necesitan estos permisos. La excepción son los dos que
/// permiten controlar el equipo entero.
fn severity_for(service: &str, allowed: bool) -> Severity {
    if !allowed {
        return Severity::Healthy;
    }
    match service {
        "kTCCServiceSystemPolicyAllFiles" | "kTCCServiceAccessibility" => Severity::Warning,
        other if is_sensitive(other) => Severity::Warning,
        _ => Severity::Healthy,
    }
}

fn note_for(service: &str, allowed: bool) -> String {
    if !allowed {
        return "Permiso denegado o no concedido".to_owned();
    }
    match service {
        "kTCCServiceSystemPolicyAllFiles" => {
            "Puede leer cualquier archivo del usuario, incluidos correo, mensajes y las bases TCC"
                .to_owned()
        }
        "kTCCServiceAccessibility" => {
            "Puede controlar el equipo: simular teclado y ratón y leer contenido de ventanas"
                .to_owned()
        }
        "kTCCServiceScreenCapture" => "Puede grabar la pantalla en cualquier momento".to_owned(),
        "kTCCServiceListenEvent" => {
            "Puede leer todas las pulsaciones de teclado (monitorización de entrada)".to_owned()
        }
        "kTCCServiceMicrophone" => "Puede grabar audio del micrófono".to_owned(),
        "kTCCServiceCamera" => "Puede capturar vídeo de la cámara".to_owned(),
        "kTCCServiceAppleEvents" => "Puede automatizar y controlar otras aplicaciones".to_owned(),
        _ => "Permiso concedido".to_owned(),
    }
}

/// Formatea una marca de tiempo Unix de TCC como RFC 3339.
fn format_epoch(seconds: i64) -> String {
    if seconds <= 0 {
        return String::new();
    }
    Utc.timestamp_opt(seconds, 0)
        .single()
        .map(|value: DateTime<Utc>| value.to_rfc3339())
        .unwrap_or_default()
}

/// Convierte los permisos concedidos en ítems vigilables por baseline. Solo se
/// vigilan los sensibles y concedidos: un permiso denegado que sigue denegado no
/// aporta nada, y vigilarlo todo llenaría la baseline de ruido.
pub fn permission_watch_items(overview: &TccOverview) -> Vec<WatchedItem> {
    overview
        .permissions
        .iter()
        .filter(|permission| permission.allowed && is_sensitive(&permission.service))
        .map(|permission| WatchedItem {
            key: format!("{}::{}", permission.service, permission.client),
            value: permission.decision.clone(),
            label: format!("{} → {}", permission.service_label, permission.client),
            detail: format!("base {}", permission.database),
            change_status: Default::default(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodifica_el_esquema_moderno() {
        assert_eq!(decode_decision(2, true), ("permitido".to_owned(), true));
        assert_eq!(decode_decision(0, true), ("denegado".to_owned(), false));
        assert_eq!(decode_decision(3, true), ("limitado".to_owned(), true));
        assert_eq!(decode_decision(1, true), ("desconocido".to_owned(), false));
    }

    #[test]
    fn decodifica_el_esquema_heredado() {
        assert_eq!(decode_decision(1, false), ("permitido".to_owned(), true));
        assert_eq!(decode_decision(0, false), ("denegado".to_owned(), false));
    }

    #[test]
    fn traduce_los_servicios_conocidos() {
        assert_eq!(
            service_label("kTCCServiceSystemPolicyAllFiles"),
            "Acceso total al disco"
        );
        assert_eq!(service_label("kTCCServiceCamera"), "Cámara");
    }

    #[test]
    fn permiso_denegado_nunca_es_advertencia() {
        assert_eq!(
            severity_for("kTCCServiceSystemPolicyAllFiles", false),
            Severity::Healthy
        );
        assert_eq!(
            severity_for("kTCCServiceSystemPolicyAllFiles", true),
            Severity::Warning
        );
    }

    #[test]
    fn solo_los_permisos_sensibles_concedidos_entran_en_la_baseline() {
        let overview = TccOverview {
            permissions: vec![
                TccPermission {
                    service: "kTCCServiceAccessibility".to_owned(),
                    service_label: "Accesibilidad".to_owned(),
                    client: "com.example.app".to_owned(),
                    allowed: true,
                    decision: "permitido".to_owned(),
                    database: "usuario".to_owned(),
                    ..Default::default()
                },
                TccPermission {
                    service: "kTCCServiceCalendar".to_owned(),
                    client: "com.example.otra".to_owned(),
                    allowed: true,
                    ..Default::default()
                },
                TccPermission {
                    service: "kTCCServiceCamera".to_owned(),
                    client: "com.example.denegada".to_owned(),
                    allowed: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let items = permission_watch_items(&overview);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key, "kTCCServiceAccessibility::com.example.app");
    }

    #[test]
    fn epoch_cero_no_produce_fecha() {
        assert!(format_epoch(0).is_empty());
        assert!(format_epoch(1_700_000_000).starts_with("2023"));
    }
}
