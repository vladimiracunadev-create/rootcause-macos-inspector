//! Reglas, priorización y armado de incidentes.
//!
//! Esta capa se mantiene separada de `inspector.rs` a propósito: recolectar
//! datos y decidir qué significan son dos trabajos distintos, y mezclarlos hace
//! imposible probar el segundo sin un macOS real detrás. Todo lo que hay aquí
//! es función pura sobre estructuras de datos, y por eso todo está cubierto por
//! tests.

use crate::config::ProcessThresholds;
use crate::models::{
    Alert, AnomalyEvent, CacheEntry, CodeSignature, ConnectionInsight, IncidentEvidence,
    IncidentSummary, PersistenceEntry, ProcessInsight, RiskLevel, SecurityControl, Severity,
    SystemOverview, SystemSnapshot, TccOverview, XProtectStatus,
};

/// Entradas para construir la lista de alertas de una captura.
pub struct AlertBuildInputs<'a> {
    pub processes: &'a [ProcessInsight],
    pub connections: &'a [ConnectionInsight],
    pub cache_entries: &'a [CacheEntry],
    pub security_controls: &'a [SecurityControl],
    pub persistence_entries: &'a [PersistenceEntry],
    pub xprotect: &'a XProtectStatus,
    pub tcc: &'a TccOverview,
    pub anomalies: &'a [AnomalyEvent],
}

/// Clasifica un proceso y devuelve `(severidad, puntaje, motivos, categoría)`.
///
/// El puntaje es acumulativo: ninguna señal por sí sola declara un problema.
/// Un proceso al 70 % de CPU es un compilador; el mismo proceso al 70 % de CPU,
/// sin firmar y ejecutándose desde `/tmp` es otra conversación.
pub fn classify_process(
    name: &str,
    exe_path: &str,
    cpu_percent: f32,
    memory_mb: f32,
    write_delta_mb: f32,
    signature: Option<CodeSignature>,
    thresholds: &ProcessThresholds,
) -> (Severity, u8, Vec<String>, String) {
    let mut score = 0_u8;
    let mut reasons = Vec::new();
    let lower_name = name.to_ascii_lowercase();
    let lower_path = exe_path.to_ascii_lowercase();

    if cpu_percent >= thresholds.cpu_critical_percent {
        score = score.saturating_add(35);
        reasons.push(format!("CPU alto ({cpu_percent:.1}%)"));
    } else if cpu_percent >= thresholds.cpu_warning_percent {
        score = score.saturating_add(18);
        reasons.push(format!("CPU sostenido ({cpu_percent:.1}%)"));
    }

    if memory_mb >= thresholds.memory_critical_mb {
        score = score.saturating_add(28);
        reasons.push(format!("Memoria elevada ({memory_mb:.0} MB)"));
    } else if memory_mb >= thresholds.memory_warning_mb {
        score = score.saturating_add(14);
        reasons.push(format!("Memoria moderada-alta ({memory_mb:.0} MB)"));
    }

    if write_delta_mb >= thresholds.io_write_critical_mb {
        score = score.saturating_add(40);
        reasons.push(format!(
            "Escritura intensa ({write_delta_mb:.1} MB en el intervalo)"
        ));
    } else if write_delta_mb >= thresholds.io_write_warning_mb {
        score = score.saturating_add(20);
        reasons.push(format!("Escritura perceptible ({write_delta_mb:.1} MB)"));
    }

    // Ruta de ejecución: en macOS el software se instala en /Applications o
    // /usr; ejecutar desde /tmp o desde una carpeta compartida no es normal.
    const SUSPICIOUS_PATHS: &[&str] = &[
        "/tmp/",
        "/private/tmp/",
        "/var/tmp/",
        "/users/shared/",
        "/downloads/",
    ];
    if SUSPICIOUS_PATHS
        .iter()
        .any(|needle| lower_path.contains(needle))
    {
        score = score.saturating_add(24);
        reasons.push("Ejecutable lanzado desde una ruta temporal o compartida".to_owned());
    }

    // Binario oculto: nombre que empieza por punto.
    if lower_path
        .rsplit('/')
        .next()
        .map(|file| file.starts_with('.'))
        .unwrap_or(false)
    {
        score = score.saturating_add(20);
        reasons.push("El binario está oculto (nombre con punto inicial)".to_owned());
    }

    match signature {
        Some(CodeSignature::Unsigned) => {
            score = score.saturating_add(26);
            reasons.push("Binario sin firma de código".to_owned());
        }
        Some(CodeSignature::AdHoc) => {
            score = score.saturating_add(14);
            reasons.push("Firma ad-hoc, sin autoridad verificable".to_owned());
        }
        _ => {}
    }

    let category = categorize(&lower_name, &lower_path);
    if category == "Instalador / actualizador" {
        score = score.saturating_add(10);
        reasons.push("Patrón de actualización o instalación".to_owned());
    }

    let severity = match score {
        0..=24 => Severity::Healthy,
        25..=54 => Severity::Warning,
        _ => Severity::Critical,
    };

    if reasons.is_empty() {
        reasons.push("Sin presión relevante en esta muestra".to_owned());
    }

    (severity, score, reasons, category)
}

/// Etiqueta legible del rol del proceso, para que la tabla no sea solo números.
fn categorize(lower_name: &str, lower_path: &str) -> String {
    const INSTALLERS: &[&str] = &["install", "update", "upgrade", "softwareupdate", "pkgutil"];
    const BROWSERS: &[&str] = &["chrome", "safari", "firefox", "edge", "brave", "arc"];
    const DEV: &[&str] = &[
        "cargo", "rustc", "clang", "node", "python", "java", "docker", "xcode",
    ];

    if INSTALLERS.iter().any(|item| lower_name.contains(item)) {
        "Instalador / actualizador".to_owned()
    } else if BROWSERS.iter().any(|item| lower_name.contains(item)) {
        "Navegador".to_owned()
    } else if DEV.iter().any(|item| lower_name.contains(item)) {
        "Herramienta de desarrollo".to_owned()
    } else if lower_path.starts_with("/system/") || lower_path.starts_with("/usr/libexec/") {
        "Componente del sistema".to_owned()
    } else if lower_path.starts_with("/applications/") {
        "Aplicación".to_owned()
    } else if lower_path.is_empty() {
        "Sin ruta visible".to_owned()
    } else {
        "Proceso de usuario".to_owned()
    }
}

/// Construye la lista de alertas priorizada y fija el veredicto del semáforo.
pub fn build_alerts(
    inputs: AlertBuildInputs<'_>,
    overview: &mut SystemOverview,
    max_alerts: usize,
) -> Vec<Alert> {
    let mut alerts: Vec<Alert> = Vec::new();

    // 1 — Anomalías correlacionadas: lo más específico va primero.
    for anomaly in inputs.anomalies.iter().take(max_alerts) {
        alerts.push(Alert {
            severity: anomaly.severity.to_severity(),
            title: anomaly.title.clone(),
            detail: anomaly.summary.clone(),
            pid: anomaly.pid,
            path: anomaly.exe_path.clone(),
            hint: anomaly.recommended_action.clone(),
        });
    }

    // 2 — Controles de seguridad apagados.
    for control in inputs
        .security_controls
        .iter()
        .filter(|control| control.severity >= Severity::Warning)
    {
        alerts.push(Alert {
            severity: control.severity,
            title: format!("{}: {}", control.name, control.status),
            detail: control.explanation.clone(),
            pid: None,
            path: None,
            hint: "Revisa Ajustes del Sistema → Privacidad y seguridad.".to_owned(),
        });
    }

    // 3 — Definiciones antimalware desactualizadas.
    if inputs.xprotect.severity >= Severity::Warning {
        alerts.push(Alert {
            severity: inputs.xprotect.severity,
            title: "Definiciones de XProtect".to_owned(),
            detail: inputs.xprotect.headline.clone(),
            pid: None,
            path: None,
            hint: "Comprueba las actualizaciones automáticas de macOS.".to_owned(),
        });
    }

    // 4 — Persistencia de alto riesgo.
    for entry in inputs
        .persistence_entries
        .iter()
        .filter(|entry| entry.severity >= RiskLevel::High)
        .take(3)
    {
        alerts.push(Alert {
            severity: entry.severity.to_severity(),
            title: format!("Persistencia de riesgo: {}", entry.name),
            detail: entry.note.clone(),
            pid: None,
            path: Some(entry.location.clone()),
            hint: "Revisa el plist y su binario antes de decidir nada.".to_owned(),
        });
    }

    // 5 — Procesos dominantes.
    for process in inputs
        .processes
        .iter()
        .filter(|process| process.severity >= Severity::Warning)
        .take(3)
    {
        alerts.push(Alert {
            severity: process.severity,
            title: format!("Proceso dominante: {} ({})", process.name, process.pid),
            detail: process.reasons.join(" · "),
            pid: Some(process.pid),
            path: Some(process.exe_path.clone()),
            hint: "Comprueba si el consumo corresponde a algo que iniciaste.".to_owned(),
        });
    }

    // 6 — Puertos expuestos a toda la red.
    if let Some(exposed) = inputs
        .connections
        .iter()
        .find(|connection| connection.is_listening && connection.severity >= Severity::Warning)
    {
        alerts.push(Alert {
            severity: Severity::Warning,
            title: format!("Puerto expuesto: {}", exposed.local_address),
            detail: format!(
                "{} ({}) escucha en todas las interfaces",
                exposed.process_name, exposed.pid
            ),
            pid: Some(exposed.pid),
            path: Some(exposed.exe_path.clone()),
            hint:
                "Si el servicio no debería ser accesible desde la red, ciérralo o filtra el puerto."
                    .to_owned(),
        });
    }

    // 7 — Permisos TCC no legibles: la ausencia de dato es un dato.
    if !inputs.tcc.readable {
        alerts.push(Alert {
            severity: Severity::Warning,
            title: "Permisos de privacidad no legibles".to_owned(),
            detail: inputs.tcc.headline.clone(),
            pid: None,
            path: None,
            hint: "Concede Acceso total al disco a RootCause para auditar los permisos TCC."
                .to_owned(),
        });
    }

    // 8 — Cachés grandes: informativo, nunca crítico por sí solo.
    if let Some(cache) = inputs
        .cache_entries
        .iter()
        .find(|entry| entry.severity >= Severity::Warning)
    {
        alerts.push(Alert {
            severity: Severity::Warning,
            title: "Cachés voluminosas".to_owned(),
            detail: format!("{} ocupa {:.0} MB", cache.path, cache.size_mb),
            pid: None,
            path: Some(cache.path.clone()),
            hint: "Puedes vaciarlas desde el tab Almacenamiento si necesitas espacio.".to_owned(),
        });
    }

    alerts.sort_by(|left, right| right.severity.cmp(&left.severity));
    alerts.truncate(max_alerts);

    // El veredicto global es la peor señal presente, con su motivo textual.
    if let Some(worst) = alerts.first() {
        if worst.severity > overview.primary_severity {
            overview.primary_severity = worst.severity;
            overview.primary_reason = worst.title.clone();
        }
    }

    alerts
}

/// Deriva un incidente resumido de la captura, si hay algo que merezca serlo.
///
/// Un incidente es la unidad que se persiste, se compara y se explica: no toda
/// captura genera uno. Si el equipo está sano, devuelve `None`.
pub fn derive_incident(snapshot: &SystemSnapshot) -> Option<IncidentSummary> {
    let worst_anomaly = snapshot
        .anomalies
        .iter()
        .max_by_key(|anomaly| (anomaly.severity, anomaly.score));

    let has_critical_alert = snapshot
        .alerts
        .iter()
        .any(|alert| alert.severity == Severity::Critical);

    let relevant_anomaly = worst_anomaly.filter(|anomaly| anomaly.severity >= RiskLevel::Medium);
    if relevant_anomaly.is_none() && !has_critical_alert {
        return None;
    }

    let severity = relevant_anomaly
        .map(|anomaly| anomaly.severity.to_severity())
        .unwrap_or(Severity::Critical)
        .max(if has_critical_alert {
            Severity::Critical
        } else {
            Severity::Warning
        });

    let kind = relevant_anomaly
        .map(|anomaly| anomaly.kind.clone())
        .unwrap_or_else(|| "resource-degradation".to_owned());

    let title = relevant_anomaly
        .map(|anomaly| anomaly.title.clone())
        .or_else(|| snapshot.alerts.first().map(|alert| alert.title.clone()))
        .unwrap_or_else(|| "Degradación detectada".to_owned());

    let summary = relevant_anomaly
        .map(|anomaly| anomaly.summary.clone())
        .or_else(|| snapshot.alerts.first().map(|alert| alert.detail.clone()))
        .unwrap_or_default();

    let risk_score = relevant_anomaly.map(|anomaly| anomaly.score).unwrap_or(60);
    let mut anomaly_types: Vec<String> = snapshot
        .anomalies
        .iter()
        .map(|anomaly| anomaly.kind.clone())
        .collect();
    anomaly_types.sort();
    anomaly_types.dedup();

    // La huella agrupa incidentes equivalentes para no persistir el mismo
    // hallazgo una vez por segundo mientras la condición sigue presente.
    let fingerprint = format!(
        "{kind}|{}|{}",
        relevant_anomaly
            .and_then(|anomaly| anomaly.process_name.clone())
            .unwrap_or_default(),
        title
    );

    Some(IncidentSummary {
        incident_id: format!("inc-{}", snapshot.collected_at.timestamp_millis()),
        fingerprint,
        collected_at: snapshot.collected_at,
        severity,
        kind,
        title,
        summary,
        root_cause_hypothesis: relevant_anomaly
            .map(|anomaly| anomaly.root_cause_hypothesis.clone())
            .unwrap_or_default(),
        probable_causes: probable_causes(snapshot),
        recommended_actions: recommended_actions(snapshot),
        evidence: incident_evidence(snapshot),
        risk_level: relevant_anomaly.map(|anomaly| anomaly.severity),
        risk_score,
        anomaly_count: snapshot.anomalies.len(),
        anomaly_types,
        anomaly_events: snapshot.anomalies.iter().take(5).cloned().collect(),
        ai_advice: None,
    })
}

fn incident_evidence(snapshot: &SystemSnapshot) -> Vec<IncidentEvidence> {
    let mut evidence = vec![
        IncidentEvidence {
            kind: "cpu".to_owned(),
            label: "CPU global".to_owned(),
            value: format!("{:.1}%", snapshot.overview.cpu_usage_percent),
        },
        IncidentEvidence {
            kind: "memory".to_owned(),
            label: "Memoria".to_owned(),
            value: format!(
                "{:.1} / {:.1} GB",
                snapshot.overview.memory_used_gb, snapshot.overview.memory_total_gb
            ),
        },
    ];

    if let Some(process) = snapshot.processes.first() {
        evidence.push(IncidentEvidence {
            kind: "process".to_owned(),
            label: "Proceso dominante".to_owned(),
            value: format!("{} ({})", process.name, process.pid),
        });
        if !process.exe_path.is_empty() {
            evidence.push(IncidentEvidence {
                kind: "path".to_owned(),
                label: "Ruta".to_owned(),
                value: process.exe_path.clone(),
            });
        }
    }

    if let Some(control) = snapshot
        .security_controls
        .iter()
        .find(|control| control.severity >= Severity::Warning)
    {
        evidence.push(IncidentEvidence {
            kind: "security".to_owned(),
            label: control.name.clone(),
            value: control.status.clone(),
        });
    }

    let public_connections = snapshot
        .connections
        .iter()
        .filter(|connection| connection.is_public_remote)
        .count();
    if public_connections > 0 {
        evidence.push(IncidentEvidence {
            kind: "network".to_owned(),
            label: "Conexiones a IP pública".to_owned(),
            value: public_connections.to_string(),
        });
    }

    evidence
}

fn probable_causes(snapshot: &SystemSnapshot) -> Vec<String> {
    let mut causes = Vec::new();

    for anomaly in snapshot.anomalies.iter().take(3) {
        if !anomaly.root_cause_hypothesis.is_empty() {
            causes.push(anomaly.root_cause_hypothesis.clone());
        }
    }

    if let Some(process) = snapshot.processes.first() {
        if process.severity >= Severity::Warning {
            causes.push(format!(
                "El proceso {} concentra la presión de recursos de esta muestra",
                process.name
            ));
        }
    }

    if snapshot
        .security_controls
        .iter()
        .any(|control| !control.enabled && control.severity >= Severity::Warning)
    {
        causes.push(
            "Hay controles de seguridad de macOS desactivados que amplían la superficie expuesta"
                .to_owned(),
        );
    }

    dedupe_strings(causes)
}

fn recommended_actions(snapshot: &SystemSnapshot) -> Vec<String> {
    let mut actions = Vec::new();

    for anomaly in snapshot.anomalies.iter().take(3) {
        if !anomaly.recommended_action.is_empty() {
            actions.push(anomaly.recommended_action.clone());
        }
    }

    if snapshot
        .persistence_entries
        .iter()
        .any(|entry| entry.change_status.is_change())
    {
        actions.push(
            "Revisa los cambios de persistencia y acepta la baseline solo si los reconoces"
                .to_owned(),
        );
    }

    if actions.is_empty() {
        actions.push(
            "Exporta la captura en JSON y compárala con el historial antes de intervenir"
                .to_owned(),
        );
    }

    dedupe_strings(actions)
}

fn dedupe_strings(items: Vec<String>) -> Vec<String> {
    let mut seen = Vec::new();
    for item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn thresholds() -> ProcessThresholds {
        ProcessThresholds::default()
    }

    #[test]
    fn proceso_tranquilo_es_saludable() {
        let (severity, score, reasons, category) = classify_process(
            "Finder",
            "/System/Library/CoreServices/Finder.app/Contents/MacOS/Finder",
            1.2,
            180.0,
            0.4,
            Some(CodeSignature::Apple),
            &thresholds(),
        );
        assert_eq!(severity, Severity::Healthy);
        assert!(score < 25);
        assert_eq!(reasons.len(), 1);
        assert_eq!(category, "Componente del sistema");
    }

    #[test]
    fn binario_sin_firmar_en_tmp_con_escritura_intensa_es_critico() {
        let (severity, score, reasons, _) = classify_process(
            "updater",
            "/tmp/updater",
            72.0,
            1_800.0,
            350.0,
            Some(CodeSignature::Unsigned),
            &thresholds(),
        );
        assert_eq!(severity, Severity::Critical);
        assert!(score > 55);
        assert!(reasons.iter().any(|reason| reason.contains("temporal")));
        assert!(reasons.iter().any(|reason| reason.contains("sin firma")));
    }

    #[test]
    fn binario_oculto_suma_puntaje() {
        let (_, con_punto, _, _) = classify_process(
            ".helper",
            "/Users/x/Library/.helper",
            5.0,
            50.0,
            1.0,
            None,
            &thresholds(),
        );
        let (_, sin_punto, _, _) = classify_process(
            "helper",
            "/Users/x/Library/helper",
            5.0,
            50.0,
            1.0,
            None,
            &thresholds(),
        );
        assert!(con_punto > sin_punto);
    }

    #[test]
    fn una_cpu_alta_por_si_sola_no_es_critica() {
        let (severity, _, _, _) = classify_process(
            "cargo",
            "/opt/homebrew/bin/cargo",
            70.0,
            300.0,
            2.0,
            Some(CodeSignature::DeveloperId),
            &thresholds(),
        );
        assert_eq!(
            severity,
            Severity::Warning,
            "compilar no debería pintarse de rojo"
        );
    }

    fn snapshot_with_anomaly(severity: RiskLevel) -> SystemSnapshot {
        SystemSnapshot {
            collected_at: Utc::now(),
            anomalies: vec![AnomalyEvent {
                severity,
                score: 80,
                kind: "security-control-change".to_owned(),
                title: "Gatekeeper se desactivó".to_owned(),
                summary: "El control cambió respecto a la baseline".to_owned(),
                root_cause_hypothesis: "alguien desactivó Gatekeeper".to_owned(),
                recommended_action: "Reactívalo con spctl".to_owned(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn una_captura_sana_no_genera_incidente() {
        let snapshot = SystemSnapshot {
            collected_at: Utc::now(),
            ..Default::default()
        };
        assert!(derive_incident(&snapshot).is_none());
    }

    #[test]
    fn una_anomalia_baja_no_genera_incidente() {
        let snapshot = snapshot_with_anomaly(RiskLevel::Low);
        assert!(derive_incident(&snapshot).is_none());
    }

    #[test]
    fn una_anomalia_alta_genera_incidente_con_causa_y_accion() {
        let snapshot = snapshot_with_anomaly(RiskLevel::High);
        let incident = derive_incident(&snapshot).expect("debe existir");
        assert_eq!(incident.severity, Severity::Critical);
        assert_eq!(incident.kind, "security-control-change");
        assert!(!incident.probable_causes.is_empty());
        assert!(!incident.recommended_actions.is_empty());
        assert!(!incident.evidence.is_empty());
    }

    #[test]
    fn la_huella_agrupa_incidentes_equivalentes() {
        // Dos capturas de la MISMA condición en momentos distintos: la huella
        // debe coincidir (para no persistir el mismo hallazgo una vez por
        // segundo) y el id debe diferir (cada incidente conserva su instante).
        let mut first_snapshot = snapshot_with_anomaly(RiskLevel::High);
        let mut second_snapshot = snapshot_with_anomaly(RiskLevel::High);
        first_snapshot.collected_at = Utc.timestamp_opt(1_800_000_000, 0).unwrap();
        second_snapshot.collected_at = Utc.timestamp_opt(1_800_000_030, 0).unwrap();

        let first = derive_incident(&first_snapshot).unwrap();
        let second = derive_incident(&second_snapshot).unwrap();
        assert_eq!(first.fingerprint, second.fingerprint);
        assert_ne!(
            first.incident_id, second.incident_id,
            "cada incidente conserva su identidad temporal"
        );
    }

    #[test]
    fn las_alertas_se_ordenan_por_severidad_y_se_recortan() {
        let processes = vec![ProcessInsight {
            pid: 10,
            name: "ruidoso".to_owned(),
            severity: Severity::Warning,
            reasons: vec!["CPU sostenido".to_owned()],
            ..Default::default()
        }];
        let controls = vec![SecurityControl {
            id: "gatekeeper".to_owned(),
            name: "Gatekeeper".to_owned(),
            status: "Desactivado".to_owned(),
            enabled: false,
            severity: Severity::Critical,
            explanation: "está apagado".to_owned(),
            ..Default::default()
        }];
        let mut overview = SystemOverview::default();

        let alerts = build_alerts(
            AlertBuildInputs {
                processes: &processes,
                connections: &[],
                cache_entries: &[],
                security_controls: &controls,
                persistence_entries: &[],
                xprotect: &XProtectStatus::default(),
                tcc: &TccOverview {
                    readable: true,
                    ..Default::default()
                },
                anomalies: &[],
            },
            &mut overview,
            2,
        );

        assert_eq!(alerts.len(), 2);
        assert_eq!(alerts[0].severity, Severity::Critical);
        assert_eq!(overview.primary_severity, Severity::Critical);
        assert!(overview.primary_reason.contains("Gatekeeper"));
    }

    #[test]
    fn tcc_ilegible_produce_alerta_explicativa() {
        let mut overview = SystemOverview::default();
        let alerts = build_alerts(
            AlertBuildInputs {
                processes: &[],
                connections: &[],
                cache_entries: &[],
                security_controls: &[],
                persistence_entries: &[],
                xprotect: &XProtectStatus::default(),
                tcc: &TccOverview {
                    readable: false,
                    headline: "falta acceso".to_owned(),
                    ..Default::default()
                },
                anomalies: &[],
            },
            &mut overview,
            8,
        );
        assert!(alerts
            .iter()
            .any(|alert| alert.title.contains("Permisos de privacidad")));
    }

    #[test]
    fn dedupe_conserva_el_orden_de_aparicion() {
        let input = vec!["b".to_owned(), "a".to_owned(), "b".to_owned()];
        assert_eq!(dedupe_strings(input), vec!["b".to_owned(), "a".to_owned()]);
    }
}
