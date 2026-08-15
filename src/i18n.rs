//! Internacionalización ligera (ES / EN).
//!
//! Filosofía de bajo riesgo: en vez de un diccionario centralizado con claves
//! (propenso a desincronizarse), la traducción es LOCAL al punto de uso mediante
//! [`tr`]: cada llamada pasa las dos variantes literales `(es, en)` y el helper
//! elige según el idioma activo. Ventajas:
//!
//! * Imposible que una clave quede huérfana o mal escrita.
//! * Las cadenas que aún no se traducen simplemente quedan en español; no hay
//!   estado intermedio roto.
//! * Cero asignaciones: se devuelven referencias `'static`.
//!
//! El idioma activo se guarda en un entero atómico global. La GUI de egui corre
//! en un único hilo, así que leerlo cada frame es trivialmente barato y no hace
//! falta cablear `Lang` por cada función de dibujo.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

/// Idioma de la interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    /// Español (por defecto).
    #[default]
    Es,
    /// Inglés.
    En,
}

impl Lang {
    /// Código corto ISO (`"es"` / `"en"`).
    pub fn code(self) -> &'static str {
        match self {
            Lang::Es => "es",
            Lang::En => "en",
        }
    }

    /// Nombre nativo del idioma, para el selector.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Es => "Español",
            Lang::En => "English",
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            Lang::Es => 0,
            Lang::En => 1,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            1 => Lang::En,
            _ => Lang::Es,
        }
    }
}

/// Idioma activo del proceso. Se inicializa en español y se ajusta al cargar la
/// configuración y cuando el usuario cambia el selector.
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Fija el idioma activo del proceso.
pub fn set_lang(lang: Lang) {
    CURRENT.store(lang.as_u8(), Ordering::Relaxed);
}

/// Devuelve el idioma activo del proceso.
pub fn current_lang() -> Lang {
    Lang::from_u8(CURRENT.load(Ordering::Relaxed))
}

/// Elige entre la variante en español y en inglés según el idioma activo.
///
/// Ejemplo: `tr("Resumen", "Overview")`.
pub fn tr(es: &'static str, en: &'static str) -> &'static str {
    match current_lang() {
        Lang::Es => es,
        Lang::En => en,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tr_respeta_el_idioma_activo() {
        set_lang(Lang::Es);
        assert_eq!(tr("Resumen", "Overview"), "Resumen");
        set_lang(Lang::En);
        assert_eq!(tr("Resumen", "Overview"), "Overview");
        // Restaurar para no afectar a otros tests del mismo binario.
        set_lang(Lang::Es);
    }

    #[test]
    fn lang_roundtrip_u8() {
        assert_eq!(Lang::from_u8(Lang::En.as_u8()), Lang::En);
        assert_eq!(Lang::from_u8(Lang::Es.as_u8()), Lang::Es);
        assert_eq!(Lang::from_u8(99), Lang::Es);
    }
}
