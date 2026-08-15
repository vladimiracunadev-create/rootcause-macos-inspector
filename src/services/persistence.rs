//! Persistencia histórica en SQLite.
//!
//! La base guarda cuatro capas:
//! 1. snapshots compactos para tendencia,
//! 2. incidentes resumidos para correlación y evidencia,
//! 3. auditoría de acciones manuales,
//! 4. baselines de superficies vigiladas (persistencia, seguridad, TCC, red).
//!
//! Vive en `~/Library/Application Support/RootCauseInspector/` y nunca sale del
//! equipo: no hay telemetría ni envío remoto en ninguna capa del producto.

use crate::models::{
    AiIncidentAdvice, AuditRecord, IncidentSummary, PersistenceChange, PersistenceEntry,
    SnapshotRow, SystemSnapshot, WatchedItem,
};
use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Clave estable que identifica una entrada de persistencia a lo largo del
/// tiempo. No incluye el comando: cambiar el comando de un mismo plist se
/// interpreta como *modificación*, no como una entrada eliminada más una nueva.
pub fn persistence_entry_key(entry: &PersistenceEntry) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        entry.entry_kind, entry.location, entry.name
    )
}

/// Adaptador pequeño sobre SQLite.
pub struct PersistenceStore {
    db_path: PathBuf,
}

impl PersistenceStore {
    /// Crea el almacenamiento en la carpeta de datos del usuario.
    pub fn new(app_name: &str) -> Result<Self> {
        let base_dir = dirs::data_local_dir()
            .or_else(dirs::data_dir)
            .context("No fue posible obtener la carpeta de datos del usuario")?
            .join(app_name);
        fs::create_dir_all(&base_dir)?;

        let db_path = base_dir.join("rootcause-history.db");
        let store = Self { db_path };
        store.ensure_schema()?;
        Ok(store)
    }

    /// Ruta física del archivo SQLite.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Guarda un resumen compacto de la instantánea actual.
    pub fn persist_snapshot(&self, snapshot: &SystemSnapshot, history_limit: usize) -> Result<()> {
        let connection = Connection::open(&self.db_path)?;
        let dominant_process = snapshot
            .processes
            .first()
            .map(|process| format!("{} ({})", process.name, process.pid))
            .unwrap_or_else(|| "Sin datos".to_owned());
        let alerts_json = serde_json::to_string(&snapshot.alerts)?;

        connection.execute(
            r#"
            INSERT INTO snapshots (
                collected_at, cpu_usage, memory_used_gb, memory_total_gb, cache_total_mb,
                network_rx_mb_delta, network_tx_mb_delta, io_read_mb_delta, io_write_mb_delta,
                dominant_process, alerts_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                snapshot.collected_at.to_rfc3339(),
                snapshot.overview.cpu_usage_percent,
                snapshot.overview.memory_used_gb,
                snapshot.overview.memory_total_gb,
                snapshot.overview.cache_total_mb,
                snapshot.overview.network_rx_mb_delta,
                snapshot.overview.network_tx_mb_delta,
                snapshot.overview.io_read_mb_delta,
                snapshot.overview.io_write_mb_delta,
                dominant_process,
                alerts_json,
            ],
        )?;

        self.trim_table("snapshots", history_limit)?;
        Ok(())
    }

    /// Guarda un incidente resumido si no es un duplicado inmediato del anterior.
    pub fn persist_incident(
        &self,
        incident: &IncidentSummary,
        incident_limit: usize,
    ) -> Result<bool> {
        let connection = Connection::open(&self.db_path)?;
        let last_fingerprint: Option<String> = connection
            .query_row(
                "SELECT fingerprint FROM incidents ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;

        if last_fingerprint.as_deref() == Some(incident.fingerprint.as_str()) {
            return Ok(false);
        }

        connection.execute(
            r#"
            INSERT INTO incidents (
                incident_id, fingerprint, collected_at, severity, kind, title, summary, payload_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                &incident.incident_id,
                &incident.fingerprint,
                incident.collected_at.to_rfc3339(),
                format!("{:?}", incident.severity),
                &incident.kind,
                &incident.title,
                &incident.summary,
                serde_json::to_string(incident)?,
            ],
        )?;

        self.trim_table("incidents", incident_limit)?;
        Ok(true)
    }

    /// Actualiza el enriquecimiento IA de un incidente ya persistido.
    pub fn update_incident_ai(&self, incident_id: &str, advice: &AiIncidentAdvice) -> Result<()> {
        let Some(mut incident) = self.load_incident_by_id(incident_id)? else {
            return Ok(());
        };
        incident.ai_advice = Some(advice.clone());

        let connection = Connection::open(&self.db_path)?;
        connection.execute(
            "UPDATE incidents SET payload_json = ?2 WHERE incident_id = ?1",
            params![incident_id, serde_json::to_string(&incident)?],
        )?;
        Ok(())
    }

    /// Últimas N filas del historial, para el tab Historial.
    pub fn load_recent(&self, limit: usize) -> Result<Vec<SnapshotRow>> {
        let connection = Connection::open(&self.db_path)?;
        let mut statement = connection.prepare(
            r#"
            SELECT id, collected_at, cpu_usage, memory_used_gb, memory_total_gb,
                   io_write_mb_delta, cache_total_mb, dominant_process, alerts_json
            FROM snapshots
            ORDER BY id DESC
            LIMIT ?1
            "#,
        )?;

        let mut rows_out = Vec::new();
        let mut rows = statement.query(params![limit as i64])?;
        while let Some(row) = rows.next()? {
            let alerts_json: String = row.get(8)?;
            let alerts: Vec<serde_json::Value> =
                serde_json::from_str(&alerts_json).unwrap_or_default();

            rows_out.push(SnapshotRow {
                id: row.get(0)?,
                collected_at: row.get(1)?,
                cpu_usage: row.get(2)?,
                memory_used_gb: row.get(3)?,
                memory_total_gb: row.get(4)?,
                io_write_mb_delta: row.get(5)?,
                cache_total_mb: row.get(6)?,
                dominant_process: row.get(7)?,
                alerts_count: alerts.len(),
                has_critical: alerts.iter().any(|alert| {
                    alert.get("severity").and_then(|value| value.as_str()) == Some("Critical")
                }),
            });
        }
        Ok(rows_out)
    }

    pub fn load_recent_incidents(&self, limit: usize) -> Result<Vec<IncidentSummary>> {
        let connection = Connection::open(&self.db_path)?;
        let mut statement =
            connection.prepare("SELECT payload_json FROM incidents ORDER BY id DESC LIMIT ?1")?;
        let rows = statement.query_map(params![limit as i64], |row| row.get::<_, String>(0))?;

        let mut incidents = Vec::new();
        for row in rows {
            if let Ok(incident) = serde_json::from_str::<IncidentSummary>(&row?) {
                incidents.push(incident);
            }
        }
        Ok(incidents)
    }

    pub fn latest_incident(&self) -> Result<Option<IncidentSummary>> {
        let connection = Connection::open(&self.db_path)?;
        let payload = connection
            .query_row(
                "SELECT payload_json FROM incidents ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(payload.and_then(|text| serde_json::from_str::<IncidentSummary>(&text).ok()))
    }

    /// Guarda un evento de auditoría.
    pub fn record_audit(&self, record: &AuditRecord) -> Result<()> {
        let connection = Connection::open(&self.db_path)?;
        connection.execute(
            "INSERT INTO audit_log (occurred_at, action, target, success, detail) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &record.occurred_at,
                &record.action,
                &record.target,
                record.success,
                &record.detail,
            ],
        )?;
        Ok(())
    }

    /// Últimos N registros de auditoría.
    pub fn load_recent_audits(&self, limit: usize) -> Result<Vec<AuditRecord>> {
        let connection = Connection::open(&self.db_path)?;
        let mut statement = connection.prepare(
            "SELECT occurred_at, action, target, success, detail \
             FROM audit_log ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit as i64], |row| {
            Ok(AuditRecord {
                occurred_at: row.get(0)?,
                action: row.get(1)?,
                target: row.get(2)?,
                success: row.get(3)?,
                detail: row.get(4)?,
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    /// Exporta el historial reciente a JSON como copia de seguridad.
    pub fn export_history_backup(&self, limit: usize) -> Result<PathBuf> {
        let rows = self.load_recent(limit)?;
        let json =
            serde_json::to_string_pretty(&rows).context("No se pudo serializar el historial")?;
        let backup_path = self
            .db_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("rootcause-history-backup.json");
        fs::write(&backup_path, json)
            .with_context(|| format!("No se pudo escribir {}", backup_path.display()))?;
        Ok(backup_path)
    }

    /// Línea resumen del último historial, para la barra de estado.
    pub fn latest_summary_line(&self) -> Result<Option<String>> {
        let connection = Connection::open(&self.db_path)?;
        let row = connection
            .query_row(
                "SELECT collected_at, cpu_usage, cache_total_mb, dominant_process \
                 FROM snapshots ORDER BY id DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, f32>(1)?,
                        row.get::<_, f32>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;

        Ok(row.map(|(collected_at, cpu, cache_mb, dominant)| {
            format!(
                "Último historial {collected_at} | CPU {cpu:.1}% | Cachés {cache_mb:.0} MB | Proceso dominante: {dominant}"
            )
        }))
    }

    /// Ruta sugerida para exportar una captura puntual.
    pub fn export_path(&self) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        dirs::download_dir()
            .or_else(dirs::document_dir)
            .unwrap_or_else(|| {
                self.db_path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .to_path_buf()
            })
            .join(format!("rootcause-snapshot-{timestamp}.json"))
    }

    fn load_incident_by_id(&self, incident_id: &str) -> Result<Option<IncidentSummary>> {
        let connection = Connection::open(&self.db_path)?;
        let payload = connection
            .query_row(
                "SELECT payload_json FROM incidents WHERE incident_id = ?1 ORDER BY id DESC LIMIT 1",
                params![incident_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(payload.and_then(|text| serde_json::from_str::<IncidentSummary>(&text).ok()))
    }

    /// Recorta una tabla dejando solo las `keep` filas más recientes.
    fn trim_table(&self, table: &str, keep: usize) -> Result<()> {
        // `table` nunca viene del usuario: son literales de este módulo.
        let connection = Connection::open(&self.db_path)?;
        connection.execute(
            &format!(
                "DELETE FROM {table} WHERE id NOT IN \
                 (SELECT id FROM {table} ORDER BY id DESC LIMIT ?1)"
            ),
            params![keep as i64],
        )?;
        Ok(())
    }

    fn ensure_schema(&self) -> Result<()> {
        let connection = Connection::open(&self.db_path)?;
        connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                collected_at TEXT NOT NULL,
                cpu_usage REAL NOT NULL,
                memory_used_gb REAL NOT NULL,
                memory_total_gb REAL NOT NULL,
                cache_total_mb REAL NOT NULL,
                network_rx_mb_delta REAL NOT NULL,
                network_tx_mb_delta REAL NOT NULL,
                io_read_mb_delta REAL NOT NULL,
                io_write_mb_delta REAL NOT NULL,
                dominant_process TEXT NOT NULL,
                alerts_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS incidents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                incident_id TEXT NOT NULL,
                fingerprint TEXT NOT NULL,
                collected_at TEXT NOT NULL,
                severity TEXT NOT NULL,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_incidents_incident_id ON incidents(incident_id);

            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                occurred_at TEXT NOT NULL,
                action TEXT NOT NULL,
                target TEXT NOT NULL,
                success INTEGER NOT NULL,
                detail TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS persistence_baseline (
                entry_key TEXT PRIMARY KEY,
                entry_kind TEXT NOT NULL,
                location TEXT NOT NULL,
                name TEXT NOT NULL,
                command TEXT NOT NULL,
                target_path TEXT,
                first_seen TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS baseline (
                surface TEXT NOT NULL,
                entry_key TEXT NOT NULL,
                value TEXT NOT NULL,
                label TEXT NOT NULL,
                detail TEXT NOT NULL,
                first_seen TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (surface, entry_key)
            );
            "#,
        )?;
        Ok(())
    }

    /// Baseline de persistencia conocida, indexada por clave estable.
    pub fn load_persistence_baseline(&self) -> Result<HashMap<String, PersistenceEntry>> {
        let connection = Connection::open(&self.db_path)?;
        let mut statement = connection.prepare(
            "SELECT entry_key, entry_kind, location, name, command, target_path \
             FROM persistence_baseline",
        )?;
        let rows = statement.query_map([], |row| {
            let key: String = row.get(0)?;
            Ok((
                key,
                PersistenceEntry {
                    entry_kind: row.get(1)?,
                    location: row.get(2)?,
                    name: row.get(3)?,
                    command: row.get(4)?,
                    target_path: row.get(5)?,
                    ..Default::default()
                },
            ))
        })?;

        let mut baseline = HashMap::new();
        for row in rows {
            let (key, entry) = row?;
            baseline.insert(key, entry);
        }
        Ok(baseline)
    }

    /// Reemplaza la baseline de persistencia con el estado actual. Se usa para
    /// sembrar la primera foto y para "aceptar" cambios como legítimos.
    pub fn replace_persistence_baseline(&self, entries: &[PersistenceEntry]) -> Result<()> {
        let mut connection = Connection::open(&self.db_path)?;
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM persistence_baseline", [])?;
        {
            let mut statement = transaction.prepare(
                "INSERT OR REPLACE INTO persistence_baseline \
                 (entry_key, entry_kind, location, name, command, target_path) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for entry in entries {
                // Nunca sembrar entradas sintéticas de tipo "eliminada".
                if matches!(entry.change_status, PersistenceChange::Removed) {
                    continue;
                }
                statement.execute(params![
                    persistence_entry_key(entry),
                    entry.entry_kind,
                    entry.location,
                    entry.name,
                    entry.command,
                    entry.target_path,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Baseline genérica de una superficie vigilada, indexada por clave estable.
    pub fn load_baseline(&self, surface: &str) -> Result<HashMap<String, WatchedItem>> {
        let connection = Connection::open(&self.db_path)?;
        let mut statement = connection
            .prepare("SELECT entry_key, value, label, detail FROM baseline WHERE surface = ?1")?;
        let rows = statement.query_map([surface], |row| {
            let key: String = row.get(0)?;
            Ok((
                key.clone(),
                WatchedItem {
                    key,
                    value: row.get(1)?,
                    label: row.get(2)?,
                    detail: row.get(3)?,
                    ..Default::default()
                },
            ))
        })?;

        let mut baseline = HashMap::new();
        for row in rows {
            let (key, item) = row?;
            baseline.insert(key, item);
        }
        Ok(baseline)
    }

    /// Reemplaza la baseline de una superficie con el estado actual.
    pub fn replace_baseline(&self, surface: &str, items: &[WatchedItem]) -> Result<()> {
        let mut connection = Connection::open(&self.db_path)?;
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM baseline WHERE surface = ?1", [surface])?;
        {
            let mut statement = transaction.prepare(
                "INSERT OR REPLACE INTO baseline (surface, entry_key, value, label, detail) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for item in items {
                if matches!(item.change_status, PersistenceChange::Removed) {
                    continue;
                }
                statement.execute(params![
                    surface,
                    item.key,
                    item.value,
                    item.label,
                    item.detail
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PersistenceScope;

    #[test]
    fn la_clave_ignora_el_comando() {
        let base = PersistenceEntry {
            entry_kind: "LaunchAgent".to_owned(),
            location: "/Library/LaunchAgents/x.plist".to_owned(),
            name: "com.x".to_owned(),
            command: "/usr/bin/uno".to_owned(),
            scope: PersistenceScope::GlobalAgent,
            ..Default::default()
        };
        let mut changed = base.clone();
        changed.command = "/usr/bin/otro".to_owned();

        assert_eq!(
            persistence_entry_key(&base),
            persistence_entry_key(&changed),
            "cambiar el comando debe leerse como modificación, no como entrada nueva"
        );
    }

    #[test]
    fn claves_distintas_para_ubicaciones_distintas() {
        let user = PersistenceEntry {
            entry_kind: "LaunchAgent".to_owned(),
            location: "/Users/x/Library/LaunchAgents/a.plist".to_owned(),
            name: "com.a".to_owned(),
            ..Default::default()
        };
        let global = PersistenceEntry {
            location: "/Library/LaunchAgents/a.plist".to_owned(),
            ..user.clone()
        };
        assert_ne!(persistence_entry_key(&user), persistence_entry_key(&global));
    }
}
