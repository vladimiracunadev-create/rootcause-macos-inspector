//! Adaptador opcional de IA.
//!
//! RootCause **no depende** de este módulo para detectar, alertar, persistir
//! evidencia ni operar por GUI o CLI. Solo se usa bajo demanda para redactar en
//! lenguaje llano un incidente que el motor local ya resumió.
//!
//! Tres decisiones deliberadas:
//!
//! * **Apagado por defecto.** Hay que activarlo en la configuración y definir un
//!   endpoint; sin eso, ni siquiera se intenta.
//! * **La clave nunca vive en el archivo de configuración**, sino en una
//!   variable de entorno cuyo nombre sí se configura.
//! * **Solo viaja el incidente ya resumido**, no la captura completa: ni
//!   procesos, ni rutas del usuario, ni permisos TCC.
//!
//! El transporte es `curl`, que viene con macOS: añadir un cliente HTTP a las
//! dependencias por una función opcional no compensa.

use crate::config::AiConfig;
use crate::models::{AiIncidentAdvice, IncidentSummary};
use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::process::Command;

pub struct AiAdvisor {
    config: AiConfig,
}

/// Forma esperada de la respuesta del modelo.
#[derive(Debug, Deserialize)]
struct AiOutputShape {
    summary: String,
    #[serde(default)]
    probable_causes: Vec<String>,
    #[serde(default)]
    suggested_actions: Vec<String>,
    #[serde(default)]
    confidence: String,
    #[serde(default)]
    warnings: Vec<String>,
}

impl AiAdvisor {
    pub fn new(config: AiConfig) -> Self {
        Self { config }
    }

    /// Enriquece un incidente. Cualquier fallo se devuelve como error: quien
    /// llama decide, y el incidente local ya persistido no se toca.
    pub fn summarize_incident(&self, incident: &IncidentSummary) -> Result<AiIncidentAdvice> {
        if !self.config.enabled {
            bail!("La integración IA está desactivada en la configuración");
        }
        if self.config.endpoint.trim().is_empty() {
            bail!("Falta `ai.endpoint` en la configuración");
        }

        let api_key = env::var(&self.config.api_key_env_var).with_context(|| {
            format!(
                "No existe la variable de entorno {} con la API key",
                self.config.api_key_env_var
            )
        })?;

        let payload = build_payload(&self.config.model, incident);
        let response = post_json(
            &self.config.endpoint,
            &api_key,
            &payload,
            self.config.timeout_secs,
        )?;
        parse_response(&response, &self.config)
    }
}

/// Construye el cuerpo de la petición, compatible con la API de chat de OpenAI.
fn build_payload(model: &str, incident: &IncidentSummary) -> String {
    let evidence: Vec<String> = incident
        .evidence
        .iter()
        .map(|item| format!("{}: {}", item.label, item.value))
        .collect();

    let user_prompt = format!(
        "Incidente detectado en un Mac.\n\
         Título: {}\n\
         Tipo: {}\n\
         Resumen: {}\n\
         Hipótesis local: {}\n\
         Evidencia: {}\n\n\
         Devuelve SOLO un objeto JSON con las claves: summary (string), \
         probable_causes (array de strings), suggested_actions (array de strings), \
         confidence (string: alta|media|baja), warnings (array de strings).",
        incident.title,
        incident.kind,
        incident.summary,
        incident.root_cause_hypothesis,
        evidence.join(" | "),
    );

    json!({
        "model": model,
        "temperature": 0.1,
        "response_format": { "type": "json_object" },
        "messages": [
            {
                "role": "system",
                "content": "Eres un analista forense de macOS. Explicas hallazgos con precisión y \
                            sin alarmismo. Nunca afirmas que hay una infección si la evidencia solo \
                            muestra un comportamiento anómalo. Respondes en español y solo con JSON."
            },
            { "role": "user", "content": user_prompt }
        ]
    })
    .to_string()
}

/// Envía la petición con `curl`. El cuerpo va por `stdin` (`--data @-`) para que
/// no aparezca en la lista de procesos, y la clave viaja en una cabecera.
fn post_json(endpoint: &str, api_key: &str, payload: &str, timeout_secs: u64) -> Result<String> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new("/usr/bin/curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--max-time",
            &timeout_secs.to_string(),
            "-X",
            "POST",
            endpoint,
            "-H",
            "Content-Type: application/json",
            "-H",
            &format!("Authorization: Bearer {api_key}"),
            "--data",
            "@-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("No se pudo invocar curl para la petición IA")?;

    child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow!("No se pudo escribir el cuerpo de la petición"))?
        .write_all(payload.as_bytes())?;

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!(
            "El proveedor IA respondió con error: {}",
            if stderr.is_empty() {
                "sin detalle".to_owned()
            } else {
                stderr
            }
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Extrae el consejo del envoltorio de la API.
pub fn parse_response(raw: &str, config: &AiConfig) -> Result<AiIncidentAdvice> {
    let envelope: Value =
        serde_json::from_str(raw).context("La respuesta del proveedor IA no es JSON válido")?;

    let content = envelope
        .get("choices")
        .and_then(|value| value.get(0))
        .and_then(|value| value.get("message"))
        .and_then(|value| value.get("content"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("La respuesta IA no trae `choices[0].message.content`"))?;

    let shape: AiOutputShape = serde_json::from_str(content)
        .context("El contenido devuelto por la IA no tiene la forma esperada")?;

    Ok(AiIncidentAdvice {
        provider: provider_from_endpoint(&config.endpoint),
        model: config.model.clone(),
        summary: shape.summary,
        probable_causes: shape.probable_causes,
        suggested_actions: shape.suggested_actions,
        confidence: if shape.confidence.is_empty() {
            "desconocida".to_owned()
        } else {
            shape.confidence
        },
        warnings: shape.warnings,
        generated_at: Utc::now().to_rfc3339(),
    })
}

/// Nombre del proveedor a partir del host del endpoint, para dejar registrado
/// a dónde se envió sin guardar la URL completa.
fn provider_from_endpoint(endpoint: &str) -> String {
    endpoint
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or("desconocido")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_ia_desactivada_falla_sin_tocar_la_red() {
        let advisor = AiAdvisor::new(AiConfig::default());
        let error = advisor
            .summarize_incident(&IncidentSummary::default())
            .expect_err("debe fallar");
        assert!(error.to_string().contains("desactivada"));
    }

    #[test]
    fn sin_endpoint_no_se_intenta_la_llamada() {
        let advisor = AiAdvisor::new(AiConfig {
            enabled: true,
            ..Default::default()
        });
        let error = advisor
            .summarize_incident(&IncidentSummary::default())
            .expect_err("debe fallar");
        assert!(error.to_string().contains("ai.endpoint"));
    }

    #[test]
    fn parsea_una_respuesta_bien_formada() {
        let raw = r#"{"choices":[{"message":{"content":"{\"summary\":\"Resumen\",\"probable_causes\":[\"c1\"],\"suggested_actions\":[\"a1\"],\"confidence\":\"media\",\"warnings\":[]}"}}]}"#;
        let advice = parse_response(raw, &AiConfig::default()).expect("debe parsear");
        assert_eq!(advice.summary, "Resumen");
        assert_eq!(advice.probable_causes, vec!["c1".to_owned()]);
        assert_eq!(advice.confidence, "media");
    }

    #[test]
    fn una_respuesta_sin_choices_es_error_claro() {
        let error = parse_response("{}", &AiConfig::default()).expect_err("debe fallar");
        assert!(error.to_string().contains("choices"));
    }

    #[test]
    fn el_proveedor_se_deduce_del_host() {
        assert_eq!(
            provider_from_endpoint("https://api.example.com/v1/chat/completions"),
            "api.example.com"
        );
        assert_eq!(provider_from_endpoint(""), "desconocido");
    }

    #[test]
    fn el_payload_solo_lleva_el_incidente_resumido() {
        let incident = IncidentSummary {
            title: "Persistencia nueva".to_owned(),
            kind: "persistence-change".to_owned(),
            summary: "Apareció un LaunchDaemon".to_owned(),
            ..Default::default()
        };
        let payload = build_payload("modelo", &incident);
        assert!(payload.contains("Persistencia nueva"));
        assert!(payload.contains("persistence-change"));
        assert!(!payload.contains("TCC"));
    }
}
