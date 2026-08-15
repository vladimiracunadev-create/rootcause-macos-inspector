//! Interfaz de línea de comandos de RootCause macOS Inspector.
//!
//! Todo lo que se puede hacer en la GUI se puede hacer aquí, y casi todo acepta
//! `--json` para encadenarlo con otras herramientas. Es el modo pensado para
//! automatización, para SSH y para incluir a RootCause en un pipeline.

use crate::meta;
use crate::models::{IncidentSummary, RiskLevel, Severity, SnapshotRow};
use crate::services::inspector::InspectorService;
use serde::Serialize;
use std::fs;

/// Ejecuta el modo CLI y devuelve el código de salida del proceso.
pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        print_help();
        return 0;
    }

    match args[0].as_str() {
        "--help" | "-h" | "help" => {
            print_help();
            0
        }
        "--version" | "-V" | "version" => {
            println!("{} v{}", meta::DISPLAY_NAME, meta::VERSION);
            0
        }
        "status" => cmd_status(&args[1..]),
        "snapshot" => cmd_snapshot(&args[1..]),
        "history" => cmd_history(&args[1..]),
        "incidents" => cmd_incidents(&args[1..]),
        "audit" => cmd_audit(&args[1..]),
        "export" => cmd_export(),
        "report" => cmd_report(),
        "persistence" => cmd_persistence(&args[1..]),
        "security" => cmd_security(&args[1..]),
        "xprotect" => cmd_xprotect(&args[1..]),
        "tcc" => cmd_tcc(&args[1..]),
        "connections" => cmd_connections(&args[1..]),
        "network" => cmd_network(&args[1..]),
        "events" => cmd_events(&args[1..]),
        "clean-caches" => cmd_clean_caches(&args[1..]),
        "kill" => cmd_kill(args.get(1).and_then(|value| value.parse::<u32>().ok())),
        "block-ip" => cmd_block_ip(args.get(1).map(String::as_str)),
        "config" => cmd_config(&args[1..]),
        "ai" => cmd_ai(&args[1..]),
        other => {
            eprintln!(
                "Comando desconocido: '{other}'\nUsa  rootcause --help  para ver todas las opciones."
            );
            1
        }
    }
}

fn print_help() {
    println!(
        r#"
╔══════════════════════════════════════════════════════════════════════╗
║  {name:<62}║
║  v{version:<61}║
║  {author:<62}║
╚══════════════════════════════════════════════════════════════════════╝

MODO GUI (por defecto):
  rootcause                              Abre la interfaz gráfica
  rootcause --gui                        Abre la interfaz gráfica (explícito)

INFORMACIÓN:
  rootcause --help                       Esta ayuda
  rootcause --version                    Versión del software

DIAGNÓSTICO DEL SISTEMA:
  rootcause status [--json]              Estado y veredicto del equipo
  rootcause snapshot [--output PATH]     Captura completa en JSON
  rootcause history [N] [--json]         Últimas N capturas del historial
  rootcause history --backup             Copia el historial a JSON junto al SQLite
  rootcause incidents [N] [--json]       Últimos incidentes persistidos
  rootcause audit [N] [--json]           Últimas acciones auditadas
  rootcause export                       Exporta la última captura a JSON
  rootcause report                       Genera un reporte forense en Markdown

PERSISTENCIA (LaunchAgents / LaunchDaemons / cron):
  rootcause persistence [--json]         Entradas con su estado vs baseline
  rootcause persistence --all            Incluye también las de Apple (SIP)
  rootcause persistence --login-items    Consulta login items (pide permiso de Automatización)
  rootcause persistence --accept         Fija el estado actual como baseline

SEGURIDAD DE macOS:
  rootcause security [--json]            Gatekeeper, SIP, FileVault, firewall, SSH
  rootcause security --accept            Fija el estado actual como baseline
  rootcause xprotect [--json]            Versión y antigüedad de las firmas de Apple

PRIVACIDAD (TCC):
  rootcause tcc [--json]                 Permisos concedidos (requiere Acceso total al disco)
  rootcause tcc --sensitive              Solo los permisos sensibles concedidos
  rootcause tcc --accept                 Fija los permisos actuales como baseline

RED:
  rootcause connections [--json]         Conexiones activas por proceso (lsof)
  rootcause network [--json]             Equipos del segmento local (vecinos ARP)
  rootcause network --deep               Barrido activo + resolución de nombres
  rootcause network --accept             Fija los equipos actuales como red conocida

REGISTRO Y MANTENIMIENTO:
  rootcause events [--minutes N]         Eventos recientes de seguridad del log unificado
  rootcause clean-caches                 Simula la limpieza de ~/Library/Caches (no borra)
  rootcause clean-caches --yes           Limpia de verdad lo no usado en 24 h

CONFIGURACIÓN E IA OPCIONAL:
  rootcause config show [--json]         Ruta y configuración efectiva
  rootcause config init                  Crea el JSON de configuración si no existe
  rootcause ai explain-latest [--json]   Enriquece el último incidente con IA

INTERVENCIÓN CONTROLADA:
  rootcause kill <PID>                   Envía SIGTERM a un proceso no protegido
  rootcause block-ip <IP>                Muestra la regla pf para bloquear una IP

Todo el análisis es local. Ningún dato sale del equipo salvo que actives la IA
opcional, que solo envía el incidente ya resumido.
"#,
        name = meta::DISPLAY_NAME,
        version = meta::VERSION,
        author = meta::AUTHOR,
    );
}

// ── Utilidades compartidas ──────────────────────────────────────────────────

fn wants(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn first_number(args: &[String], default: usize) -> usize {
    args.iter()
        .find_map(|arg| arg.parse::<usize>().ok())
        .unwrap_or(default)
}

/// Crea el motor o informa del fallo de forma legible.
fn service() -> Result<InspectorService, i32> {
    match InspectorService::new() {
        Ok(service) => Ok(service),
        Err(error) => {
            eprintln!("No se pudo inicializar RootCause: {error}");
            Err(1)
        }
    }
}

fn print_json<T: Serialize>(value: &T) -> i32 {
    match serde_json::to_string_pretty(value) {
        Ok(json) => {
            println!("{json}");
            0
        }
        Err(error) => {
            eprintln!("No se pudo serializar la salida: {error}");
            1
        }
    }
}

fn severity_mark(severity: Severity) -> &'static str {
    match severity {
        Severity::Healthy => "[ OK ]",
        Severity::Warning => "[WARN]",
        Severity::Critical => "[CRIT]",
    }
}

fn risk_mark(risk: RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "[ OK ]",
        RiskLevel::Medium => "[WARN]",
        RiskLevel::High | RiskLevel::Critical => "[CRIT]",
    }
}

// ── Comandos ────────────────────────────────────────────────────────────────

fn cmd_status(args: &[String]) -> i32 {
    let Ok(mut service) = service() else {
        return 1;
    };
    let snapshot = match service.collect_snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("No se pudo capturar el estado: {error}");
            return 1;
        }
    };

    if wants(args, "--json") {
        return print_json(&snapshot);
    }

    let hardware = service.get_hardware_info();
    println!("{} v{}", meta::DISPLAY_NAME, meta::VERSION);
    println!(
        "{} · {} {} · {} ({} núcleos) · {:.1} GB RAM\n",
        hardware.host_name,
        hardware.os_name,
        hardware.os_version,
        hardware.cpu_brand,
        hardware.cpu_cores,
        hardware.total_ram_gb,
    );

    println!(
        "VEREDICTO {} {}",
        severity_mark(snapshot.overview.primary_severity),
        snapshot.overview.primary_reason
    );
    println!(
        "CPU {:.1}% · RAM {:.1}/{:.1} GB · Escritura {:.1} MB · Cachés {:.0} MB\n",
        snapshot.overview.cpu_usage_percent,
        snapshot.overview.memory_used_gb,
        snapshot.overview.memory_total_gb,
        snapshot.overview.io_write_mb_delta,
        snapshot.overview.cache_total_mb,
    );

    println!("CONTROLES DE SEGURIDAD");
    for control in &snapshot.security_controls {
        println!(
            "  {} {:<34} {}",
            severity_mark(control.severity),
            control.name,
            control.status
        );
    }

    println!("\nXPROTECT");
    println!(
        "  {} {}",
        severity_mark(snapshot.xprotect.severity),
        snapshot.xprotect.headline
    );

    println!("\nPERSISTENCIA");
    let changed = snapshot
        .persistence_entries
        .iter()
        .filter(|entry| entry.change_status.is_change())
        .count();
    println!(
        "  {} entradas vigiladas · {changed} con cambios vs baseline",
        snapshot.persistence_entries.len()
    );

    println!("\nPRIVACIDAD (TCC)");
    println!("  {}", snapshot.tcc.headline);

    let environment = service.environment();
    if !environment.is_root {
        println!(
            "\nCONTEXTO  usuario {} (uid {}) · sin privilegios de root: lsof solo ve tus sockets",
            environment.user, environment.uid
        );
    }
    let missing = environment.missing_tools();
    if !missing.is_empty() {
        println!(
            "          faltan utilidades del sistema: {}",
            missing.join(", ")
        );
    }

    if snapshot.alerts.is_empty() {
        println!("\nSin alertas en esta captura.");
    } else {
        println!("\nALERTAS");
        for alert in &snapshot.alerts {
            println!("  {} {}", severity_mark(alert.severity), alert.title);
            if !alert.detail.is_empty() {
                println!("        {}", alert.detail);
            }
        }
    }

    if let Some(incident) = snapshot.incident.as_ref() {
        println!("\nINCIDENTE DOMINANTE");
        println!("  {} ({})", incident.title, incident.kind);
        println!("  {}", incident.summary);
    }

    0
}

fn cmd_snapshot(args: &[String]) -> i32 {
    let Ok(mut service) = service() else {
        return 1;
    };
    let snapshot = match service.collect_snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("No se pudo capturar: {error}");
            return 1;
        }
    };

    match flag_value(args, "--output") {
        Some(path) => match serde_json::to_string_pretty(&snapshot)
            .map_err(|error| error.to_string())
            .and_then(|json| fs::write(path, json).map_err(|error| error.to_string()))
        {
            Ok(()) => {
                println!("Captura escrita en {path}");
                0
            }
            Err(error) => {
                eprintln!("No se pudo escribir {path}: {error}");
                1
            }
        },
        None => print_json(&snapshot),
    }
}

fn cmd_history(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };

    if wants(args, "--backup") {
        return match service.export_history_backup() {
            Ok(path) => {
                println!("Copia del historial escrita en {path}");
                0
            }
            Err(error) => {
                eprintln!("No se pudo escribir la copia: {error}");
                1
            }
        };
    }

    let rows: Vec<SnapshotRow> = service.load_history(first_number(args, 20));

    if wants(args, "--json") {
        return print_json(&rows);
    }
    if rows.is_empty() {
        println!(
            "El historial está vacío. Ejecuta `rootcause status` para generar la primera captura."
        );
        return 0;
    }

    println!(
        "{:<26} {:>7} {:>12} {:>10}  PROCESO DOMINANTE",
        "FECHA", "CPU", "RAM", "ALERTAS"
    );
    for row in &rows {
        println!(
            "{:<26} {:>6.1}% {:>7.1}/{:<4.1} {:>10}  {}",
            row.collected_at,
            row.cpu_usage,
            row.memory_used_gb,
            row.memory_total_gb,
            row.alerts_count,
            row.dominant_process,
        );
    }
    0
}

fn cmd_incidents(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };
    let incidents: Vec<IncidentSummary> = service.load_incidents(first_number(args, 10));

    if wants(args, "--json") {
        return print_json(&incidents);
    }
    if incidents.is_empty() {
        println!("No hay incidentes persistidos.");
        return 0;
    }

    for incident in &incidents {
        println!(
            "{} {} · {}",
            severity_mark(incident.severity),
            incident.collected_at.to_rfc3339(),
            incident.title
        );
        println!("      {}", incident.summary);
        if !incident.recommended_actions.is_empty() {
            println!("      → {}", incident.recommended_actions[0]);
        }
    }
    0
}

fn cmd_audit(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };
    let records = service.load_audits(first_number(args, 25));

    if wants(args, "--json") {
        return print_json(&records);
    }
    if records.is_empty() {
        println!("No hay acciones auditadas todavía.");
        return 0;
    }

    for record in &records {
        println!(
            "{} {:<28} {:<28} {}",
            if record.success { "[ OK ]" } else { "[FAIL]" },
            record.action,
            record.target,
            record.detail
        );
    }
    0
}

fn cmd_export() -> i32 {
    let Ok(mut service) = service() else {
        return 1;
    };
    let snapshot = match service.collect_snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("No se pudo capturar: {error}");
            return 1;
        }
    };
    match service.export_snapshot(&snapshot) {
        Ok(path) => {
            println!("Captura exportada en {path}");
            0
        }
        Err(error) => {
            eprintln!("No se pudo exportar: {error}");
            1
        }
    }
}

fn cmd_report() -> i32 {
    let Ok(mut service) = service() else {
        return 1;
    };
    let snapshot = match service.collect_snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("No se pudo capturar: {error}");
            return 1;
        }
    };
    match service.generate_report(&snapshot) {
        Ok(path) => {
            println!("Reporte forense generado en {path}");
            0
        }
        Err(error) => {
            eprintln!("No se pudo generar el reporte: {error}");
            1
        }
    }
}

fn cmd_persistence(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };

    if wants(args, "--accept") {
        return match service.accept_persistence_baseline() {
            Ok(count) => {
                println!("Baseline de persistencia actualizada con {count} entradas.");
                0
            }
            Err(error) => {
                eprintln!("No se pudo actualizar la baseline: {error}");
                1
            }
        };
    }

    let mut entries = service.persistence_with_changes(wants(args, "--all"));
    if wants(args, "--login-items") {
        entries.extend(service.login_items());
    }

    if wants(args, "--json") {
        return print_json(&entries);
    }
    if entries.is_empty() {
        println!("No se encontraron entradas de persistencia.");
        return 0;
    }

    println!("{:<7} {:<12} {:<38} COMANDO", "RIESGO", "CAMBIO", "LABEL");
    for entry in &entries {
        println!(
            "{:<7} {:<12} {:<38} {}",
            risk_mark(entry.severity),
            if entry.change_status.is_change() {
                entry.change_status.label()
            } else {
                "-"
            },
            truncate(&entry.name, 38),
            truncate(&entry.command, 60),
        );
    }
    println!(
        "\n{} entradas. Usa --json para el detalle completo.",
        entries.len()
    );
    0
}

fn cmd_security(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };

    if wants(args, "--accept") {
        return match service.accept_security_baseline() {
            Ok(count) => {
                println!("Baseline de seguridad actualizada con {count} controles.");
                0
            }
            Err(error) => {
                eprintln!("No se pudo actualizar la baseline: {error}");
                1
            }
        };
    }

    let controls = crate::services::security::scan_controls();
    if wants(args, "--json") {
        return print_json(&controls);
    }

    for control in &controls {
        println!(
            "{} {:<34} {:<12} {}",
            severity_mark(control.severity),
            control.name,
            control.status,
            control.evidence
        );
    }
    println!();
    for control in controls.iter().filter(|c| c.severity >= Severity::Warning) {
        println!("→ {}: {}", control.name, control.explanation);
    }
    0
}

fn cmd_xprotect(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };
    let status = crate::services::security::scan_xprotect(
        &service.config().thresholds.xprotect,
        chrono::Utc::now(),
    );

    if wants(args, "--json") {
        return print_json(&status);
    }

    println!("{} {}", severity_mark(status.severity), status.headline);
    for definition in &status.definitions {
        println!(
            "  {} {:<24} v{:<16} {}",
            severity_mark(definition.severity),
            definition.component,
            definition.version,
            definition.note
        );
    }
    0
}

fn cmd_tcc(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };

    if wants(args, "--accept") {
        return match service.accept_tcc_baseline() {
            Ok(count) => {
                println!("Baseline de permisos TCC actualizada con {count} permisos.");
                0
            }
            Err(error) => {
                eprintln!("{error}");
                1
            }
        };
    }

    let overview = crate::services::tcc::scan();
    if wants(args, "--json") {
        return print_json(&overview);
    }

    println!("{}", overview.headline);
    if !overview.readable {
        for limitation in &overview.limitations {
            println!("  - {limitation}");
        }
        return 1;
    }

    let only_sensitive = wants(args, "--sensitive");
    for permission in overview.permissions.iter().filter(|permission| {
        permission.allowed
            && (!only_sensitive || crate::services::tcc::is_sensitive(&permission.service))
    }) {
        println!(
            "{} {:<38} {:<10} {}",
            severity_mark(permission.severity),
            truncate(&permission.service_label, 38),
            permission.decision,
            permission.client
        );
    }
    0
}

fn cmd_connections(args: &[String]) -> i32 {
    let Ok(mut service) = service() else {
        return 1;
    };
    let snapshot = match service.collect_snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("No se pudo capturar: {error}");
            return 1;
        }
    };

    if wants(args, "--json") {
        return print_json(&snapshot.connections);
    }
    if snapshot.connections.is_empty() {
        println!("No se observaron conexiones. `lsof` sin privilegios solo ve los sockets del usuario actual.");
        return 0;
    }

    println!(
        "{:<7} {:<22} {:>7} {:<26} {:<26} ESTADO",
        "SEV", "PROCESO", "PID", "LOCAL", "REMOTO"
    );
    for connection in snapshot.connections.iter().take(80) {
        println!(
            "{:<7} {:<22} {:>7} {:<26} {:<26} {}",
            severity_mark(connection.severity),
            truncate(&connection.process_name, 22),
            connection.pid,
            truncate(&connection.local_address, 26),
            truncate(&connection.remote_address, 26),
            connection.state,
        );
    }
    0
}

fn cmd_network(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };

    if wants(args, "--accept") {
        return match service.accept_network_baseline() {
            Ok(count) => {
                println!("Red conocida actualizada con {count} dispositivos.");
                0
            }
            Err(error) => {
                eprintln!("No se pudo actualizar la baseline: {error}");
                1
            }
        };
    }

    let deep = wants(args, "--deep");
    if deep {
        println!("Barrido activo del segmento en curso; puede tardar…");
    }
    let scan = service.scan_network(deep);

    if wants(args, "--json") {
        return print_json(&scan);
    }

    println!(
        "Interfaz {} · IP {} · Puerta de enlace {} · {} equipos ({} nuevos)\n",
        scan.adapter_name, scan.local_ip, scan.gateway_ip, scan.total_devices, scan.new_devices
    );
    println!(
        "{:<7} {:<16} {:<19} {:<16} NOTA",
        "SEV", "IP", "MAC", "FABRICANTE"
    );
    for device in &scan.devices {
        println!(
            "{:<7} {:<16} {:<19} {:<16} {}",
            severity_mark(device.severity),
            device.ip,
            device.mac,
            truncate(&device.vendor, 16),
            device.reason,
        );
    }
    0
}

fn cmd_events(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };
    let minutes = flag_value(args, "--minutes")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(30);

    println!("Consultando el log unificado de los últimos {minutes} minutos…");
    let events = service.security_events(minutes);

    if wants(args, "--json") {
        return print_json(&events);
    }
    if events.is_empty() {
        println!("Sin eventos de seguridad en la ventana consultada.");
        return 0;
    }

    for event in &events {
        println!(
            "{} {:<16} {}",
            event.timestamp,
            truncate(&event.provider, 16),
            truncate(&event.message, 120)
        );
    }
    0
}

fn cmd_clean_caches(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };
    let confirmed = wants(args, "--yes");
    let result = service.clean_caches(!confirmed);

    if confirmed {
        println!(
            "Limpieza completada: {:.1} MB liberados · {} entradas borradas · {} en uso saltadas · {} recientes conservadas",
            result.freed_mb, result.deleted_count, result.skipped_in_use, result.skipped_recent
        );
    } else {
        println!(
            "SIMULACIÓN (no se borró nada): se liberarían {:.1} MB en {} entradas.\nEjecuta `rootcause clean-caches --yes` para hacerlo de verdad.",
            result.freed_mb, result.deleted_count
        );
    }
    0
}

fn cmd_kill(pid: Option<u32>) -> i32 {
    let Some(pid) = pid else {
        eprintln!("Uso: rootcause kill <PID>");
        return 2;
    };
    let Ok(service) = service() else {
        return 1;
    };

    match service.terminate_process(pid) {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn cmd_block_ip(ip: Option<&str>) -> i32 {
    let Some(ip) = ip else {
        eprintln!("Uso: rootcause block-ip <IP>");
        return 2;
    };
    let Ok(service) = service() else {
        return 1;
    };

    match service.suggest_block_ip(ip) {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn cmd_config(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };

    match args.first().map(String::as_str) {
        Some("init") => match service.write_default_config_if_missing() {
            Ok(path) => {
                println!("Configuración disponible en {path}");
                0
            }
            Err(error) => {
                eprintln!("No se pudo crear la configuración: {error}");
                1
            }
        },
        Some("show") | None => {
            if wants(args, "--json") {
                return print_json(service.config());
            }
            println!("Configuración: {}", service.config_path().display());
            println!("Historial:     {}", service.db_path().display());
            println!();
            match serde_json::to_string_pretty(service.config()) {
                Ok(json) => {
                    println!("{json}");
                    0
                }
                Err(error) => {
                    eprintln!("No se pudo mostrar la configuración: {error}");
                    1
                }
            }
        }
        Some(other) => {
            eprintln!("Subcomando de config desconocido: '{other}'. Usa `show` o `init`.");
            2
        }
    }
}

fn cmd_ai(args: &[String]) -> i32 {
    let Ok(service) = service() else {
        return 1;
    };

    match args.first().map(String::as_str) {
        Some("explain-latest") => match service.explain_latest_incident_with_ai() {
            Ok(advice) => {
                if wants(args, "--json") {
                    return print_json(&advice);
                }
                println!("Proveedor: {} · Modelo: {}", advice.provider, advice.model);
                println!("Confianza: {}\n", advice.confidence);
                println!("{}", advice.summary);
                for cause in &advice.probable_causes {
                    println!("  · causa: {cause}");
                }
                for action in &advice.suggested_actions {
                    println!("  → acción: {action}");
                }
                0
            }
            Err(error) => {
                eprintln!("La IA opcional no pudo responder: {error}");
                eprintln!("RootCause sigue funcionando con normalidad sin ella.");
                1
            }
        },
        _ => {
            eprintln!("Uso: rootcause ai explain-latest [--json]");
            2
        }
    }
}

/// Recorta un texto a `max` caracteres para que las tablas no se descuadren.
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
    fn detecta_banderas_presentes() {
        let args = vec!["--json".to_owned(), "--deep".to_owned()];
        assert!(wants(&args, "--json"));
        assert!(!wants(&args, "--accept"));
    }

    #[test]
    fn lee_el_valor_de_una_bandera() {
        let args = vec!["--output".to_owned(), "/tmp/x.json".to_owned()];
        assert_eq!(flag_value(&args, "--output"), Some("/tmp/x.json"));
        assert_eq!(flag_value(&args, "--minutes"), None);
    }

    #[test]
    fn una_bandera_sin_valor_no_entra_en_panico() {
        let args = vec!["--output".to_owned()];
        assert_eq!(flag_value(&args, "--output"), None);
    }

    #[test]
    fn toma_el_primer_numero_o_el_valor_por_defecto() {
        assert_eq!(first_number(&["5".to_owned()], 20), 5);
        assert_eq!(first_number(&["--json".to_owned()], 20), 20);
    }

    #[test]
    fn recorta_respetando_caracteres_multibyte() {
        assert_eq!(truncate("corto", 10), "corto");
        assert_eq!(truncate("configuración", 6), "confi…");
        assert_eq!(truncate("ñññññ", 3), "ññ…");
    }

    #[test]
    fn la_ayuda_y_la_version_siempre_salen_bien() {
        assert_eq!(run(&["--help".to_owned()]), 0);
        assert_eq!(run(&["--version".to_owned()]), 0);
    }

    #[test]
    fn un_comando_desconocido_devuelve_error() {
        assert_eq!(run(&["inventado".to_owned()]), 1);
    }

    #[test]
    fn kill_sin_pid_pide_uso_correcto() {
        assert_eq!(cmd_kill(None), 2);
    }
}
