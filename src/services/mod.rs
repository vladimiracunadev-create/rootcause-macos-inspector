//! Módulos de servicio.
//!
//! Aquí vive la lógica no visual: recolección, parseo, persistencia y acciones.

/// Adaptador IA opcional, apagado por defecto.
pub mod ai;
/// Heurísticas de comportamiento anómalo.
pub mod anomaly;
/// Motor genérico de cambios contra el estado bueno conocido.
pub mod baseline;
/// Orquestador de la captura y de las acciones.
pub mod inspector;
/// Persistencia de macOS: LaunchAgents, LaunchDaemons, login items y cron.
pub mod launchd;
/// Adaptador del sistema: utilidades nativas de macOS.
pub mod macos;
/// Vecinos de la red local (tabla ARP).
pub mod netscan;
/// Conexiones activas por proceso (`lsof`).
pub mod network;
/// Historial, incidentes, auditoría y baselines en SQLite.
pub mod persistence;
/// Reporte forense en Markdown.
pub mod report;
/// Salud y autoprotección del propio agente.
pub mod resilience;
/// Clasificación, alertas e incidentes.
pub mod rules;
/// Controles de seguridad nativos y estado de XProtect.
pub mod security;
/// Permisos de privacidad (TCC).
pub mod tcc;
/// Cachés y temporales: medición y limpieza segura.
pub mod temp_scan;
