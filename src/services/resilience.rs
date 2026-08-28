//! Autoprotección y resiliencia del propio agente.
//!
//! Una herramienta de diagnóstico puede convertirse en objetivo: lo primero que
//! hace un atacante consciente es callar al que observa. Este módulo no promete
//! invulnerabilidad —no hay supervisor de nivel sistema ni protección contra un
//! root decidido—; hace lo que sí se puede hacer en el espacio de usuario y lo
//! declara con honestidad:
//!
//! * **Heartbeat local:** deja constancia de que el agente sigue vivo.
//! * **Cierre abrupto:** al arrancar, detecta si la sesión anterior no cerró
//!   limpiamente y lo reporta en vez de empezar de cero como si nada.
//! * **Integridad de configuración:** guarda una huella del archivo de config y
//!   avisa si cambió entre sesiones.
//! * **Reinicios repetidos:** cuenta cierres inesperados dentro de una ventana.

use crate::config::ResilienceConfig;
use crate::models::{AgentHealth, AgentStatus, AuditRecord};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Estado que sobrevive entre ejecuciones, en JSON junto al historial.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AgentStateFile {
    #[serde(default)]
    last_start_at: String,
    #[serde(default)]
    last_heartbeat_at: String,
    #[serde(default)]
    last_clean_shutdown_at: Option<String>,
    #[serde(default)]
    config_fingerprint: String,
    #[serde(default)]
    consecutive_unexpected_stops: u32,
    #[serde(default)]
    window_started_at: Option<String>,
}

/// Vigila la salud del agente y produce registros de auditoría.
pub struct ResilienceMonitor {
    state_path: PathBuf,
    state: AgentStateFile,
    health: AgentHealth,
    config: ResilienceConfig,
    startup_audits: Vec<AuditRecord>,
    last_heartbeat: DateTime<Utc>,
}

impl ResilienceMonitor {
    /// Carga el estado anterior, lo interpreta y prepara la sesión actual.
    pub fn new(app_name: &str, config_path: &Path, config: &ResilienceConfig) -> Result<Self> {
        let base_dir = dirs::data_local_dir()
            .or_else(dirs::data_dir)
            .context("No fue posible obtener la carpeta de datos del usuario")?
            .join(app_name);
        fs::create_dir_all(&base_dir)?;
        let state_path = base_dir.join("rootcause-agent-state.json");

        let previous = fs::read_to_string(&state_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<AgentStateFile>(&raw).ok())
            .unwrap_or_default();

        let now = Utc::now();
        let fingerprint = config_fingerprint(config_path);
        let mut notes = Vec::new();
        let mut audits = Vec::new();

        // ── Cierre abrupto anterior ──────────────────────────────────────────
        // Hubo arranque previo y ningún cierre limpio posterior: la sesión
        // anterior murió sin avisar.
        let unexpected = !previous.last_start_at.is_empty()
            && previous
                .last_clean_shutdown_at
                .as_deref()
                .map(|value| value < previous.last_start_at.as_str())
                .unwrap_or(true);

        let mut consecutive = previous.consecutive_unexpected_stops;
        let mut window_started_at = previous.window_started_at.clone();

        if unexpected {
            // La ventana caduca para que reinicios repartidos en el tiempo no se
            // acumulen indefinidamente hasta declarar un problema que no existe.
            let window_expired = window_started_at
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|start| {
                    (now - start.with_timezone(&Utc)).num_seconds()
                        > config.restart_window_secs as i64
                })
                .unwrap_or(true);

            if window_expired {
                consecutive = 1;
                window_started_at = Some(now.to_rfc3339());
            } else {
                consecutive = consecutive.saturating_add(1);
            }

            notes.push(format!(
                "La sesión anterior terminó sin cierre limpio ({consecutive} en la ventana actual)."
            ));
            audits.push(AuditRecord {
                occurred_at: now.to_rfc3339(),
                action: "agent-unexpected-stop".to_owned(),
                target: previous.last_start_at.clone(),
                success: false,
                detail: "Arranque tras un cierre no limpio".to_owned(),
            });
        } else {
            consecutive = 0;
        }

        // ── Integridad de configuración ──────────────────────────────────────
        let config_changed = config.watch_config_integrity
            && !previous.config_fingerprint.is_empty()
            && previous.config_fingerprint != fingerprint;
        if config_changed {
            notes.push("La configuración cambió desde la última sesión.".to_owned());
            audits.push(AuditRecord {
                occurred_at: now.to_rfc3339(),
                action: "agent-config-changed".to_owned(),
                target: config_path.display().to_string(),
                success: true,
                detail: format!(
                    "Huella anterior {} → actual {}",
                    previous.config_fingerprint, fingerprint
                ),
            });
        }

        let too_many_restarts = consecutive >= u32::from(config.max_restarts_in_window);
        let status = if !config.enabled {
            AgentStatus::Healthy
        } else if too_many_restarts || config_changed {
            AgentStatus::Degraded
        } else if unexpected {
            AgentStatus::Recovered
        } else {
            AgentStatus::Healthy
        };

        let summary = match status {
            AgentStatus::Healthy => {
                "Agente estable: heartbeat activo y configuración sin cambios.".to_owned()
            }
            AgentStatus::Recovered => {
                "Agente recuperado tras un cierre abrupto de la sesión anterior.".to_owned()
            }
            AgentStatus::Degraded if too_many_restarts => format!(
                "Se detectaron {consecutive} cierres inesperados en menos de {} s.",
                config.restart_window_secs
            ),
            AgentStatus::Degraded => {
                "La configuración cambió respecto a la sesión anterior.".to_owned()
            }
        };

        let state = AgentStateFile {
            last_start_at: now.to_rfc3339(),
            last_heartbeat_at: now.to_rfc3339(),
            last_clean_shutdown_at: previous.last_clean_shutdown_at.clone(),
            config_fingerprint: fingerprint.clone(),
            consecutive_unexpected_stops: consecutive,
            window_started_at,
        };

        let health = AgentHealth {
            status,
            summary,
            last_start_at: state.last_start_at.clone(),
            last_heartbeat_at: state.last_heartbeat_at.clone(),
            last_clean_shutdown_at: state.last_clean_shutdown_at.clone(),
            config_fingerprint: fingerprint,
            config_changed,
            unexpected_shutdown_detected: unexpected,
            consecutive_unexpected_stops: consecutive,
            notes,
        };

        let monitor = Self {
            state_path,
            state,
            health,
            config: config.clone(),
            startup_audits: audits,
            last_heartbeat: now,
        };
        monitor.persist()?;
        Ok(monitor)
    }

    /// Estado de salud calculado al arrancar esta sesión.
    pub fn health(&self) -> &AgentHealth {
        &self.health
    }

    /// Registros de auditoría generados durante el arranque.
    pub fn startup_audits(&self) -> Vec<AuditRecord> {
        self.startup_audits.clone()
    }

    /// Marca el latido de la sesión actual, respetando el intervalo configurado.
    pub fn heartbeat(&mut self) -> Result<Vec<AuditRecord>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let now = Utc::now();
        if (now - self.last_heartbeat).num_seconds() < self.config.heartbeat_interval_secs as i64 {
            return Ok(Vec::new());
        }

        self.last_heartbeat = now;
        self.state.last_heartbeat_at = now.to_rfc3339();
        self.health.last_heartbeat_at = self.state.last_heartbeat_at.clone();
        self.persist()?;
        Ok(Vec::new())
    }

    /// Cierre limpio: deja constancia para que el próximo arranque no lo lea
    /// como una caída.
    pub fn shutdown(&mut self) -> Result<AuditRecord> {
        let now = Utc::now();
        self.state.last_clean_shutdown_at = Some(now.to_rfc3339());
        self.state.consecutive_unexpected_stops = 0;
        self.persist()?;

        Ok(AuditRecord {
            occurred_at: now.to_rfc3339(),
            action: "agent-clean-shutdown".to_owned(),
            target: "rootcause".to_owned(),
            success: true,
            detail: "Cierre limpio registrado".to_owned(),
        })
    }

    fn persist(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.state)?;
        fs::write(&self.state_path, json)
            .with_context(|| format!("No se pudo escribir {}", self.state_path.display()))?;
        Ok(())
    }
}

/// Huella de la configuración: tamaño y fecha de modificación.
///
/// No es un hash criptográfico a propósito — no defiende contra un atacante que
/// quiera falsificarla, y decir lo contrario sería vender humo. Sirve para lo
/// que sirve: notar que el archivo cambió entre dos sesiones.
fn config_fingerprint(path: &Path) -> String {
    let Ok(metadata) = path.metadata() else {
        return "sin-config".to_owned();
    };
    let modified = metadata
        .modified()
        .ok()
        .map(DateTime::<Utc>::from)
        .map(|value| value.timestamp())
        .unwrap_or_default();
    format!("{}-{}", metadata.len(), modified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_huella_de_una_config_inexistente_es_estable() {
        let fingerprint = config_fingerprint(Path::new("/ruta/que/no/existe.json"));
        assert_eq!(fingerprint, "sin-config");
    }

    #[test]
    fn el_estado_viaja_a_json_y_vuelve() {
        let state = AgentStateFile {
            last_start_at: "2026-01-01T00:00:00Z".to_owned(),
            consecutive_unexpected_stops: 2,
            ..Default::default()
        };
        let json = serde_json::to_string(&state).expect("serializa");
        let back: AgentStateFile = serde_json::from_str(&json).expect("deserializa");
        assert_eq!(back.consecutive_unexpected_stops, 2);
        assert_eq!(back.last_start_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn un_estado_vacio_no_rompe_la_deserializacion() {
        let back: AgentStateFile = serde_json::from_str("{}").expect("deserializa vacío");
        assert!(back.last_start_at.is_empty());
        assert_eq!(back.consecutive_unexpected_stops, 0);
    }
}
