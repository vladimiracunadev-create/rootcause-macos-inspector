//! Motor genérico de detección de cambios contra baseline.
//!
//! Es el corazón del producto y la idea más simple que contiene: **compara el
//! estado actual con una foto del "estado bueno conocido" y reporta cualquier
//! diferencia**. No necesita saber qué es una amenaza para notar que algo
//! cambió mientras nadie miraba.
//!
//! Cada "superficie vigilada" (persistencia, controles de seguridad, permisos
//! TCC, equipos de la red) aporta una lista de [`WatchedItem`], y este motor la
//! clasifica como NUEVA / MODIFICADA / ELIMINADA. Dos decisiones deliberadas:
//!
//! * **La primera foto se siembra en silencio.** Estrenar la herramienta no
//!   debe generar cien alertas por software que ya estaba instalado.
//! * **Los cambios son pegajosos.** Se siguen reportando hasta que alguien
//!   acepta explícitamente la nueva baseline. Una alerta que se auto-silencia
//!   tras un reinicio es peor que no tenerla.

use crate::models::{AnomalyEvent, IncidentEvidence, PersistenceChange, RiskLevel, WatchedItem};
use crate::services::persistence::PersistenceStore;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

/// Descripción de una superficie vigilada para construir los textos del evento.
pub struct SurfaceSpec {
    /// Id corto: se usa como `kind` de anomalía (`<id>-change`) y clave de tabla.
    pub id: &'static str,
    /// Título del evento cuando aparece un ítem nuevo.
    pub title_added: &'static str,
    /// Título del evento cuando un ítem cambia de valor.
    pub title_modified: &'static str,
    /// Título del evento cuando un ítem desaparece.
    pub title_removed: &'static str,
    /// Sustantivo con artículo para el resumen: "El control", "El permiso".
    pub summary_noun: &'static str,
    /// Severidad base cuando aparece o cambia un ítem de esta superficie.
    pub risk_on_change: RiskLevel,
}

/// Superficie: controles de seguridad nativos (Gatekeeper, SIP, FileVault…).
pub const SECURITY_SURFACE: SurfaceSpec = SurfaceSpec {
    id: "security-control",
    title_added: "Control de seguridad nuevo",
    title_modified: "Un control de seguridad cambió de estado",
    title_removed: "Control de seguridad ya no reportado",
    summary_noun: "El control",
    risk_on_change: RiskLevel::High,
};

/// Superficie: permisos TCC sensibles concedidos.
pub const TCC_SURFACE: SurfaceSpec = SurfaceSpec {
    id: "tcc-permission",
    title_added: "Permiso de privacidad nuevo",
    title_modified: "Un permiso de privacidad cambió",
    title_removed: "Permiso de privacidad revocado",
    summary_noun: "El permiso",
    risk_on_change: RiskLevel::High,
};

/// Superficie: equipos de la red local conocida.
pub const NETWORK_SURFACE_ID: &str = "network-device";

/// Compara `items` contra la baseline de la superficie y los anota con su estado
/// de cambio. Si la baseline está vacía (primera vez), la siembra con el estado
/// actual y no marca nada. Añade ítems sintéticos para los eliminados.
///
/// Devuelve `true` si había una baseline previa contra la que comparar.
pub fn diff_surface(
    store: &PersistenceStore,
    surface_id: &str,
    items: &mut Vec<WatchedItem>,
) -> bool {
    let Ok(baseline) = store.load_baseline(surface_id) else {
        return false;
    };

    if baseline.is_empty() {
        // Primera foto: aceptar todo como baseline "buena conocida".
        let _ = store.replace_baseline(surface_id, items);
        return false;
    }

    let mut current_keys = HashSet::new();
    for item in items.iter_mut() {
        current_keys.insert(item.key.clone());
        item.change_status = match baseline.get(&item.key) {
            None => PersistenceChange::Added,
            Some(base) if base.value != item.value => PersistenceChange::Modified,
            Some(_) => PersistenceChange::Unchanged,
        };
    }

    // Ítems que estaban en la baseline y ya no aparecen: sintéticos eliminados.
    for (key, base) in &baseline {
        if !current_keys.contains(key) {
            let mut removed = base.clone();
            removed.change_status = PersistenceChange::Removed;
            items.push(removed);
        }
    }

    true
}

/// Construye un evento de anomalía para un ítem cambiado.
///
/// A diferencia de las heurísticas, no depende de "sospecha": cualquier cambio
/// se reporta, porque la decisión sobre si es legítimo es del usuario y no de la
/// herramienta. Devuelve `None` para ítems sin cambios.
pub fn surface_change_event(
    detected_at: DateTime<Utc>,
    spec: &SurfaceSpec,
    item: &WatchedItem,
) -> Option<AnomalyEvent> {
    let (severity, score, title, verb) = match item.change_status {
        PersistenceChange::Added => (spec.risk_on_change, 72_u16, spec.title_added, "apareció"),
        PersistenceChange::Modified => (spec.risk_on_change, 68, spec.title_modified, "cambió"),
        PersistenceChange::Removed => (RiskLevel::Medium, 45, spec.title_removed, "desapareció"),
        PersistenceChange::Unchanged => return None,
    };

    Some(AnomalyEvent {
        event_id: format!(
            "anom-{}-{}chg-{}",
            detected_at.timestamp_millis(),
            spec.id,
            item.key
        ),
        detected_at,
        severity,
        score,
        status: "open".to_owned(),
        kind: format!("{}-change", spec.id),
        title: title.to_owned(),
        exe_path: Some(item.detail.clone()),
        summary: format!(
            "{} '{}' {} respecto a la baseline conocida.",
            spec.summary_noun, item.label, verb
        ),
        root_cause_hypothesis:
            "cambio en una superficie vigilada respecto al estado bueno conocido".to_owned(),
        recommended_action:
            "Verifica el origen del cambio y acepta la baseline solo si es legítimo.".to_owned(),
        evidence: vec![
            IncidentEvidence {
                kind: "item".to_owned(),
                label: "Elemento".to_owned(),
                value: item.label.clone(),
            },
            IncidentEvidence {
                kind: "detail".to_owned(),
                label: "Detalle".to_owned(),
                value: item.detail.clone(),
            },
            IncidentEvidence {
                kind: "change".to_owned(),
                label: "Cambio".to_owned(),
                value: item.change_status.label().to_owned(),
            },
        ],
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(key: &str, value: &str) -> WatchedItem {
        WatchedItem {
            key: key.to_owned(),
            value: value.to_owned(),
            label: key.to_owned(),
            detail: String::new(),
            change_status: PersistenceChange::Unchanged,
        }
    }

    #[test]
    fn un_item_sin_cambios_no_genera_evento() {
        assert!(surface_change_event(Utc::now(), &SECURITY_SURFACE, &item("a", "b")).is_none());
    }

    #[test]
    fn control_de_seguridad_modificado_genera_evento_de_riesgo_alto() {
        let mut changed = item("gatekeeper", "Desactivado");
        changed.change_status = PersistenceChange::Modified;

        let event = surface_change_event(Utc::now(), &SECURITY_SURFACE, &changed)
            .expect("debe generar evento");
        assert_eq!(event.kind, "security-control-change");
        assert_eq!(event.severity, RiskLevel::High);
        assert!(event.summary.contains("cambió"));
    }

    #[test]
    fn un_item_eliminado_baja_a_riesgo_medio() {
        let mut removed = item("tcc", "permitido");
        removed.change_status = PersistenceChange::Removed;

        let event =
            surface_change_event(Utc::now(), &TCC_SURFACE, &removed).expect("debe generar evento");
        assert_eq!(event.severity, RiskLevel::Medium);
        assert!(event.summary.contains("desapareció"));
    }

    #[test]
    fn la_evidencia_incluye_el_tipo_de_cambio() {
        let mut added = item("filevault", "Desactivado");
        added.change_status = PersistenceChange::Added;

        let event = surface_change_event(Utc::now(), &SECURITY_SURFACE, &added).unwrap();
        assert!(event
            .evidence
            .iter()
            .any(|evidence| evidence.value == "NUEVA"));
    }
}
