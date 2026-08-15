//! Cachés y temporales de macOS: qué ocupa espacio y qué es seguro vaciar.
//!
//! macOS reparte lo desechable en varios sitios (`~/Library/Caches`,
//! `~/Library/Logs`, el `TMPDIR` por sesión en `/private/var/folders`, la
//! DerivedData de Xcode…). El escaneo es **deliberadamente acotado**: mide las
//! raíces conocidas con un tope de entradas por raíz en vez de indexar el disco.
//! Un monitor que tarda un minuto en refrescar deja de usarse.

use crate::config::CacheThresholds;
use crate::models::{CacheCleanResult, CacheEntry, CacheOverview, Severity};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

/// Tope de entradas recorridas por raíz. Con esto una raíz enorme se mide
/// aproximadamente en vez de bloquear la captura.
const MAX_ENTRIES_PER_ROOT: usize = 40_000;

/// Raíz vigilada: ruta, descripción y si RootCause la considera segura de vaciar.
struct CacheRoot {
    path: &'static str,
    note: &'static str,
    safe_to_clean: bool,
}

const CACHE_ROOTS: &[CacheRoot] = &[
    CacheRoot {
        path: "~/Library/Caches",
        note: "Cachés de aplicaciones del usuario; se regeneran solas",
        safe_to_clean: true,
    },
    CacheRoot {
        path: "~/Library/Logs",
        note: "Registros de aplicaciones del usuario",
        safe_to_clean: true,
    },
    CacheRoot {
        path: "~/Library/Developer/Xcode/DerivedData",
        note: "Productos intermedios de Xcode; se reconstruyen al compilar",
        safe_to_clean: true,
    },
    CacheRoot {
        path: "~/Library/Containers/com.apple.Safari/Data/Library/Caches",
        note: "Caché de Safari",
        safe_to_clean: true,
    },
    CacheRoot {
        path: "/private/var/tmp",
        note: "Temporales del sistema que sobreviven al reinicio",
        safe_to_clean: false,
    },
    CacheRoot {
        path: "/Library/Caches",
        note: "Cachés para todos los usuarios; requiere administrador",
        safe_to_clean: false,
    },
    CacheRoot {
        path: "~/.Trash",
        note: "Papelera del usuario; vaciarla es una decisión tuya, no de RootCause",
        safe_to_clean: false,
    },
];

fn expand_home(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => dirs::home_dir().unwrap_or_default().join(rest),
        None => PathBuf::from(path),
    }
}

/// Mide las raíces conocidas y devuelve el resumen ordenado por tamaño.
pub fn scan(thresholds: &CacheThresholds) -> CacheOverview {
    let mut entries = Vec::new();
    let mut roots_scanned = Vec::new();
    let mut truncated_roots = Vec::new();
    let mut total_mb = 0.0_f32;

    // El TMPDIR por sesión vive bajo /private/var/folders y cambia en cada
    // arranque, así que se resuelve en tiempo de ejecución.
    let mut roots: Vec<(PathBuf, &str, bool)> = CACHE_ROOTS
        .iter()
        .map(|root| (expand_home(root.path), root.note, root.safe_to_clean))
        .collect();
    if let Ok(tmpdir) = std::env::var("TMPDIR") {
        roots.push((
            PathBuf::from(tmpdir),
            "Temporales de esta sesión (TMPDIR)",
            true,
        ));
    }

    for (path, note, safe_to_clean) in roots {
        if !path.is_dir() {
            continue;
        }
        let display = path.display().to_string();
        roots_scanned.push(display.clone());

        let (size_mb, file_count, truncated) = measure_directory(&path, MAX_ENTRIES_PER_ROOT);
        if truncated {
            truncated_roots.push(display.clone());
        }
        total_mb += size_mb;

        entries.push(CacheEntry {
            path: display,
            size_mb,
            file_count,
            severity: severity_for_size(size_mb, thresholds),
            note: note.to_owned(),
            safe_to_clean,
        });
    }

    entries.sort_by(|left, right| right.size_mb.total_cmp(&left.size_mb));

    let mut limitations = vec![
        "El escaneo mide raíces conocidas, no indexa el disco completo.".to_owned(),
        "Vaciar cachés libera espacio pero puede hacer que la próxima apertura de una app sea más lenta."
            .to_owned(),
    ];
    if !truncated_roots.is_empty() {
        limitations.push(format!(
            "Medición aproximada (tope de {MAX_ENTRIES_PER_ROOT} entradas) en: {}",
            truncated_roots.join(", ")
        ));
    }

    CacheOverview {
        total_mb,
        roots_scanned,
        top_entries: entries,
        limitations,
    }
}

/// Suma el tamaño de un directorio con tope de entradas.
///
/// Devuelve `(MB, nº de archivos, se alcanzó el tope)`. No sigue enlaces
/// simbólicos: seguirlos podría salir del árbol o entrar en un bucle.
fn measure_directory(path: &Path, max_entries: usize) -> (f32, u64, bool) {
    let mut bytes = 0_u64;
    let mut files = 0_u64;
    let mut visited = 0_usize;

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        visited += 1;
        if visited > max_entries {
            return (bytes_to_mb(bytes), files, true);
        }
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                bytes = bytes.saturating_add(metadata.len());
                files += 1;
            }
        }
    }

    (bytes_to_mb(bytes), files, false)
}

fn severity_for_size(size_mb: f32, thresholds: &CacheThresholds) -> Severity {
    if size_mb >= thresholds.critical_mb {
        Severity::Critical
    } else if size_mb >= thresholds.warning_mb {
        Severity::Warning
    } else {
        Severity::Healthy
    }
}

/// Vacía `~/Library/Caches` con tres salvaguardas: solo esa carpeta, solo lo que
/// no se ha tocado en `min_age_hours`, y saltando lo que falle por estar en uso.
///
/// `dry_run` calcula exactamente lo mismo sin borrar nada — y es el valor por
/// defecto en el CLI: borrar requiere pedirlo dos veces.
pub fn clean_user_caches(min_age_hours: u64, dry_run: bool) -> CacheCleanResult {
    let mut result = CacheCleanResult {
        dry_run,
        ..Default::default()
    };

    let Some(root) = dirs::home_dir().map(|home| home.join("Library/Caches")) else {
        return result;
    };
    let Ok(read_dir) = std::fs::read_dir(&root) else {
        return result;
    };

    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(min_age_hours * 3_600))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    for item in read_dir.flatten() {
        let path = item.path();
        let Ok(metadata) = item.metadata() else {
            result.error_count += 1;
            continue;
        };

        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if modified > cutoff {
            result.skipped_recent += 1;
            continue;
        }

        let (size_mb, file_count, _) = if metadata.is_dir() {
            measure_directory(&path, MAX_ENTRIES_PER_ROOT)
        } else {
            (bytes_to_mb(metadata.len()), 1, false)
        };

        if dry_run {
            result.freed_mb += size_mb;
            result.deleted_count += file_count.max(1);
            continue;
        }

        let removed = if metadata.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };

        match removed {
            Ok(()) => {
                result.freed_mb += size_mb;
                result.deleted_count += file_count.max(1);
            }
            // `PermissionDenied` y `ResourceBusy` significan casi siempre que una
            // app tiene el archivo abierto: se salta, no se fuerza.
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::Other
                ) =>
            {
                result.skipped_in_use += 1;
            }
            Err(_) => result.error_count += 1,
        }
    }

    result
}

fn bytes_to_mb(bytes: u64) -> f32 {
    bytes as f32 / (1024.0 * 1024.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severidad_por_tamano_respeta_umbrales() {
        let thresholds = CacheThresholds {
            warning_mb: 2_048.0,
            critical_mb: 8_192.0,
        };
        assert_eq!(severity_for_size(100.0, &thresholds), Severity::Healthy);
        assert_eq!(severity_for_size(3_000.0, &thresholds), Severity::Warning);
        assert_eq!(severity_for_size(9_000.0, &thresholds), Severity::Critical);
    }

    #[test]
    fn expand_home_resuelve_rutas_de_usuario_y_absolutas() {
        assert!(expand_home("~/Library/Caches").is_absolute());
        assert_eq!(
            expand_home("/private/var/tmp"),
            PathBuf::from("/private/var/tmp")
        );
    }

    #[test]
    fn medir_directorio_inexistente_devuelve_cero() {
        let (mb, files, truncated) = measure_directory(Path::new("/ruta/que/no/existe"), 100);
        assert_eq!(mb, 0.0);
        assert_eq!(files, 0);
        assert!(!truncated);
    }

    #[test]
    fn el_simulacro_nunca_marca_borrado_real() {
        let result = clean_user_caches(24, true);
        assert!(result.dry_run);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn conversion_de_bytes_a_mb() {
        assert!((bytes_to_mb(1_048_576) - 1.0).abs() < f32::EPSILON);
    }
}
