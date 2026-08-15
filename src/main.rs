//! Punto de entrada de la aplicación.
//!
//! Soporta dos modos de operación y dos ediciones de compilación:
//!
//! ## Modos de operación
//! * **GUI** (por defecto): `rootcause` o `rootcause --gui`
//! * **CLI**: `rootcause <comando>` — para scripts y automatización.
//!
//! ## Ediciones de compilación
//! * **Completa** (feature `gui`, por defecto): incluye egui + interfaz gráfica.
//! * **CLI-only** (`--no-default-features`): solo consola, sin egui.
//!
//! El modo CLI se despacha sin inicializar ningún contexto gráfico, así que
//! funciona por SSH y en sesiones sin pantalla.

// La edición CLI-only no compila `app.rs`, así que parte del código compartido
// —constantes del producto, traducción de la interfaz, helpers de
// presentación— queda sin usar en esa edición. No es código muerto: lo usa la
// edición completa, y la CI valida ambas.
#![cfg_attr(not(feature = "gui"), allow(dead_code))]

#[cfg(feature = "gui")]
mod app;
mod cli;
mod config;
mod i18n;
mod meta;
mod models;
mod services;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Con argumentos (y salvo `--gui`), se despacha al modo CLI.
    if args.len() > 1 && args[1] != "--gui" {
        std::process::exit(cli::run(&args[1..]));
    }

    #[cfg(feature = "gui")]
    {
        if let Err(error) = launch_gui() {
            eprintln!("Error al iniciar la interfaz gráfica: {error}");
            std::process::exit(1);
        }
    }

    // Edición CLI-only: sin argumentos, mostrar la ayuda.
    #[cfg(not(feature = "gui"))]
    {
        std::process::exit(cli::run(&["--help".to_owned()]));
    }
}

#[cfg(feature = "gui")]
fn launch_gui() -> eframe::Result<()> {
    use app::RootCauseApp;
    use eframe::egui;

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("RootCause — macOS Inspector v{}", meta::VERSION))
            .with_icon(rootcause_icon())
            .with_inner_size([1280.0, 800.0])
            // Mínimo bajo para que quepa en un MacBook de 13" sin recortes.
            .with_min_inner_size([880.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RootCause — macOS Inspector",
        native_options,
        Box::new(|cc| Ok(Box::new(RootCauseApp::new(cc)))),
    )
}

/// Icono de la aplicación: un radar de círculos concéntricos en el azul de
/// marca (#1f6feb) sobre fondo oscuro (#0d1117). Se dibuja a mano para no
/// depender de decodificadores externos ni de un recurso en disco.
#[cfg(feature = "gui")]
fn rootcause_icon() -> eframe::egui::IconData {
    let size: u32 = 64;
    let mut rgba = vec![0_u8; (size * size * 4) as usize];

    let center = (size as f32 - 1.0) / 2.0;
    let ring_width = 3.0_f32;
    let outer_radius = 26.0_f32;
    let inner_radius = 13.0_f32;
    let dot_radius = 4.0_f32;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();

            let on_ring = (distance - outer_radius).abs() < ring_width
                || (distance - inner_radius).abs() < ring_width;
            let on_dot = distance < dot_radius;

            let (r, g, b) = if on_ring || on_dot {
                (31, 111, 235)
            } else {
                (13, 17, 23)
            };

            let index = ((y * size + x) * 4) as usize;
            rgba[index] = r;
            rgba[index + 1] = g;
            rgba[index + 2] = b;
            rgba[index + 3] = 255;
        }
    }

    eframe::egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}
