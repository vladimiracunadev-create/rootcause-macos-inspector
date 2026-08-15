//! Metadatos del producto — versión, autor, contacto.

/// Versión del software (se sincroniza automáticamente con Cargo.toml).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Nombre visible en la UI, el CLI y los instaladores.
pub const DISPLAY_NAME: &str = "RootCause macOS Inspector";

/// Descripción breve para el tab Acerca y el `--help` del CLI.
pub const DESCRIPTION: &str =
    "Monitor forense ligero para macOS. Observa LaunchAgents/Daemons, procesos, Gatekeeper, \
     XProtect, permisos TCC, red y persistencia para explicar la causa raíz con evidencia.";

/// Autor principal.
pub const AUTHOR: &str = "Vladimir Acuña";

/// URL del repositorio en GitHub.
pub const GITHUB: &str = "https://github.com/vladimiracunadev-create/rootcause-macos-inspector";

/// Repositorio hermano para Windows (mismo producto, otra plataforma).
pub const GITHUB_WINDOWS: &str =
    "https://github.com/vladimiracunadev-create/rootcause-windows-inspector";

/// Licencia del software.
pub const LICENSE: &str = "Apache License 2.0";

/// Identificador de bundle usado por el `.app` empaquetado.
pub const BUNDLE_ID: &str = "dev.vladimiracuna.rootcause";

/// Carpeta de datos del usuario (config, historial SQLite, reportes).
pub const APP_DIR: &str = "RootCauseInspector";
