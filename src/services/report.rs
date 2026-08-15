//! Reporte forense en Markdown.
//!
//! Es la salida pensada para que la lea una persona: un documento con la
//! evidencia de una captura concreta, ordenado como se investiga —veredicto,
//! qué está encendido, qué persiste, quién habla con fuera, qué consume— y con
//! sus limitaciones escritas al final en vez de sugeridas.

use crate::meta;
use crate::models::{HardwareInfo, RiskLevel, Severity, SystemSnapshot};
use chrono::Local;
use std::path::PathBuf;

/// Carpeta donde se guardan los reportes generados.
pub fn reports_dir() -> PathBuf {
    dirs::document_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RootCause")
        .join("reports")
}

/// Construye el reporte completo de una captura.
pub fn build_report(snapshot: &SystemSnapshot, hardware: &HardwareInfo) -> String {
    let mut out = String::new();

    out.push_str(&format!("# Reporte forense · {}\n\n", meta::DISPLAY_NAME));
    out.push_str(&format!(
        "- **Generado:** {}\n- **Versión:** {}\n- **Equipo:** {} ({})\n- **Sistema:** {} {}\n- **CPU:** {} · {} núcleos\n- **RAM:** {:.1} GB\n\n",
        snapshot.collected_at.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S"),
        meta::VERSION,
        nonempty(&hardware.host_name),
        nonempty(&hardware.model),
        nonempty(&hardware.os_name),
        nonempty(&hardware.os_version),
        nonempty(&hardware.cpu_brand),
        hardware.cpu_cores,
        hardware.total_ram_gb,
    ));

    out.push_str("## 1 · Veredicto\n\n");
    out.push_str(&format!(
        "**{}** — {}\n\n",
        severity_word(snapshot.overview.primary_severity),
        nonempty(&snapshot.overview.primary_reason),
    ));

    if let Some(incident) = snapshot.incident.as_ref() {
        out.push_str(&format!(
            "### Incidente dominante\n\n- **Título:** {}\n- **Tipo:** {}\n- **Resumen:** {}\n- **Hipótesis:** {}\n\n",
            incident.title,
            incident.kind,
            incident.summary,
            nonempty(&incident.root_cause_hypothesis),
        ));
        if !incident.probable_causes.is_empty() {
            out.push_str("**Causas probables:**\n\n");
            for cause in &incident.probable_causes {
                out.push_str(&format!("- {cause}\n"));
            }
            out.push('\n');
        }
        if !incident.recommended_actions.is_empty() {
            out.push_str("**Acciones sugeridas:**\n\n");
            for action in &incident.recommended_actions {
                out.push_str(&format!("- {action}\n"));
            }
            out.push('\n');
        }
    }

    out.push_str("## 2 · Alertas\n\n");
    if snapshot.alerts.is_empty() {
        out.push_str("_Sin alertas en esta captura._\n\n");
    } else {
        out.push_str("| Severidad | Alerta | Detalle |\n|---|---|---|\n");
        for alert in &snapshot.alerts {
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                severity_word(alert.severity),
                escape(&alert.title),
                escape(&alert.detail),
            ));
        }
        out.push('\n');
    }

    out.push_str("## 3 · Controles de seguridad de macOS\n\n");
    out.push_str("| Control | Estado | Evidencia |\n|---|---|---|\n");
    for control in &snapshot.security_controls {
        out.push_str(&format!(
            "| {} | {} | `{}` |\n",
            escape(&control.name),
            control.status,
            escape(&control.evidence),
        ));
    }
    out.push('\n');

    out.push_str("## 4 · Definiciones antimalware (XProtect)\n\n");
    out.push_str(&format!("{}\n\n", snapshot.xprotect.headline));
    if !snapshot.xprotect.definitions.is_empty() {
        out.push_str("| Componente | Versión | Antigüedad |\n|---|---|---|\n");
        for definition in &snapshot.xprotect.definitions {
            out.push_str(&format!(
                "| {} | {} | {} días |\n",
                escape(&definition.component),
                escape(&definition.version),
                definition.age_days,
            ));
        }
        out.push('\n');
    }

    out.push_str("## 5 · Persistencia (LaunchAgents / LaunchDaemons)\n\n");
    let relevant: Vec<_> = snapshot
        .persistence_entries
        .iter()
        .filter(|entry| entry.severity >= RiskLevel::Medium || entry.change_status.is_change())
        .take(25)
        .collect();
    if relevant.is_empty() {
        out.push_str(&format!(
            "_{} entradas inventariadas; ninguna con riesgo medio o superior ni cambios vs baseline._\n\n",
            snapshot.persistence_entries.len()
        ));
    } else {
        out.push_str("| Riesgo | Cambio | Label | Comando | Ubicación |\n|---|---|---|---|---|\n");
        for entry in relevant {
            out.push_str(&format!(
                "| {} | {} | {} | `{}` | `{}` |\n",
                entry.severity.label(),
                if entry.change_status.is_change() {
                    entry.change_status.label()
                } else {
                    "—"
                },
                escape(&entry.name),
                escape(&entry.command),
                escape(&entry.location),
            ));
        }
        out.push('\n');
    }

    out.push_str("## 6 · Permisos de privacidad (TCC)\n\n");
    out.push_str(&format!("{}\n\n", snapshot.tcc.headline));
    let sensitive: Vec<_> = snapshot
        .tcc
        .permissions
        .iter()
        .filter(|permission| permission.allowed && permission.severity >= Severity::Warning)
        .take(25)
        .collect();
    if !sensitive.is_empty() {
        out.push_str("| Permiso | Aplicación | Base |\n|---|---|---|\n");
        for permission in sensitive {
            out.push_str(&format!(
                "| {} | `{}` | {} |\n",
                escape(&permission.service_label),
                escape(&permission.client),
                permission.database,
            ));
        }
        out.push('\n');
    }

    out.push_str("## 7 · Procesos dominantes\n\n");
    out.push_str(
        "| Severidad | Proceso | PID | CPU | RAM | Escritura |\n|---|---|---|---|---|---|\n",
    );
    for process in snapshot.processes.iter().take(12) {
        out.push_str(&format!(
            "| {} | {} | {} | {:.1}% | {:.0} MB | {:.1} MB |\n",
            severity_word(process.severity),
            escape(&process.name),
            process.pid,
            process.cpu_percent,
            process.memory_mb,
            process.io_write_mb_delta,
        ));
    }
    out.push('\n');

    out.push_str("## 8 · Conexiones relevantes\n\n");
    let connections: Vec<_> = snapshot
        .connections
        .iter()
        .filter(|connection| connection.is_public_remote || connection.is_listening)
        .take(20)
        .collect();
    if connections.is_empty() {
        out.push_str("_Sin conexiones públicas ni puertos a la escucha en esta captura._\n\n");
    } else {
        out.push_str("| Proceso | PID | Local | Remoto | Estado |\n|---|---|---|---|---|\n");
        for connection in connections {
            out.push_str(&format!(
                "| {} | {} | `{}` | `{}` | {} |\n",
                escape(&connection.process_name),
                connection.pid,
                escape(&connection.local_address),
                escape(&connection.remote_address),
                nonempty(&connection.state),
            ));
        }
        out.push('\n');
    }

    if let Some(network) = snapshot.network.as_ref() {
        out.push_str("## 9 · Red local\n\n");
        out.push_str(&format!(
            "- **Interfaz:** {} · **IP:** {} · **Puerta de enlace:** {}\n- **Equipos vistos:** {} ({} nuevos vs baseline)\n\n",
            nonempty(&network.adapter_name),
            nonempty(&network.local_ip),
            nonempty(&network.gateway_ip),
            network.total_devices,
            network.new_devices,
        ));
    }

    out.push_str("## 10 · Salud del agente\n\n");
    out.push_str(&format!(
        "- **Estado:** {}\n- **Resumen:** {}\n- **Último arranque:** {}\n\n",
        snapshot.agent_health.status.label(),
        nonempty(&snapshot.agent_health.summary),
        nonempty(&snapshot.agent_health.last_start_at),
    ));

    out.push_str("## 11 · Limitaciones de esta captura\n\n");
    for limitation in snapshot
        .caches
        .limitations
        .iter()
        .chain(snapshot.tcc.limitations.iter())
        .chain(snapshot.xprotect.limitations.iter())
    {
        out.push_str(&format!("- {limitation}\n"));
    }
    out.push_str(
        "- RootCause es un sensor forense y de apoyo a la decisión: no elimina malware ni sustituye a un EDR.\n",
    );
    out.push_str("\n---\n\n");
    out.push_str(&format!(
        "_Generado localmente por {} v{}. Ningún dato de este reporte salió del equipo._\n",
        meta::DISPLAY_NAME,
        meta::VERSION
    ));

    out
}

/// Guarda el reporte con nombre fechado y devuelve la ruta.
pub fn save_report(content: &str) -> std::io::Result<PathBuf> {
    let dir = reports_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!(
        "rootcause-report-{}.md",
        Local::now().format("%Y%m%d-%H%M%S")
    ));
    std::fs::write(&path, content)?;
    Ok(path)
}

fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Healthy => "🟢 Normal",
        Severity::Warning => "🟡 Atención",
        Severity::Critical => "🔴 Crítico",
    }
}

fn nonempty(value: &str) -> String {
    if value.trim().is_empty() {
        "—".to_owned()
    } else {
        value.to_owned()
    }
}

/// Neutraliza las barras verticales para no romper las tablas Markdown.
fn escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Alert, ProcessInsight, SystemOverview};
    use chrono::Utc;

    fn snapshot() -> SystemSnapshot {
        SystemSnapshot {
            collected_at: Utc::now(),
            overview: SystemOverview {
                cpu_usage_percent: 12.5,
                primary_severity: Severity::Warning,
                primary_reason: "Gatekeeper desactivado".to_owned(),
                ..Default::default()
            },
            alerts: vec![Alert {
                severity: Severity::Warning,
                title: "Gatekeeper: Desactivado".to_owned(),
                detail: "El control | está apagado".to_owned(),
                ..Default::default()
            }],
            processes: vec![ProcessInsight {
                pid: 42,
                name: "helper".to_owned(),
                cpu_percent: 55.0,
                memory_mb: 300.0,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn el_reporte_incluye_veredicto_y_secciones() {
        let report = build_report(&snapshot(), &HardwareInfo::default());
        assert!(report.contains("# Reporte forense"));
        assert!(report.contains("## 1 · Veredicto"));
        assert!(report.contains("Gatekeeper desactivado"));
        assert!(report.contains("## 11 · Limitaciones de esta captura"));
    }

    #[test]
    fn las_barras_no_rompen_las_tablas() {
        let report = build_report(&snapshot(), &HardwareInfo::default());
        assert!(report.contains("El control \\| está apagado"));
    }

    #[test]
    fn los_campos_vacios_se_muestran_como_guion() {
        assert_eq!(nonempty("  "), "—");
        assert_eq!(nonempty("valor"), "valor");
    }

    #[test]
    fn el_reporte_declara_que_nada_sale_del_equipo() {
        let report = build_report(&snapshot(), &HardwareInfo::default());
        assert!(report.contains("Ningún dato de este reporte salió del equipo"));
    }
}
