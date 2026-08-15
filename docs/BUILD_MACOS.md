# Compilar en macOS

## Requisitos

| Requisito | Versión | Cómo obtenerlo |
|---|---|---|
| macOS | 13 (Ventura) o superior | — |
| Rust | estable reciente | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Herramientas de línea de comandos de Xcode | cualquiera | `xcode-select --install` |

No hacen falta dependencias de sistema adicionales: SQLite se compila incrustado (`rusqlite` con
la feature `bundled`) y la interfaz usa `eframe`/`egui` sobre OpenGL.

Verificación automática:

```bash
./scripts/verify-environment.sh
```

## Compilar

```bash
# Edición completa (GUI + CLI) — por defecto
cargo build --release

# Edición CLI-only, sin egui ni dependencias gráficas
cargo build --release --no-default-features
```

El binario queda en `target/release/rootcause`.

## Ejecutar

```bash
./target/release/rootcause          # interfaz gráfica
./target/release/rootcause status   # modo consola
./target/release/rootcause --help   # todos los comandos
```

## Validación local

Réplica exacta de lo que hace la CI:

```bash
./scripts/ci-local.sh
```

O paso a paso:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

## Compilación universal (Apple Silicon + Intel)

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin

cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

lipo -create -output rootcause \
  target/aarch64-apple-darwin/release/rootcause \
  target/x86_64-apple-darwin/release/rootcause

lipo -info rootcause   # debe listar ambas arquitecturas
```

`./scripts/package-app.sh --universal` hace esto y además construye el `.app`.

## Perfil de release

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

La compilación de release tarda notablemente más que la de debug por el LTO y el `codegen-units=1`.
Para iterar durante el desarrollo, usa `cargo build` sin `--release`.

## Problemas frecuentes al compilar

| Síntoma | Causa | Solución |
|---|---|---|
| `linker 'cc' not found` | Faltan las herramientas de Xcode | `xcode-select --install` |
| `failed to run custom build command for libsqlite3-sys` | Compilador de C no disponible | Igual que arriba |
| `error: edition 2021 is unstable` | Rust demasiado antiguo | `rustup update stable` |
| La ventana no abre por SSH | No hay sesión gráfica | Usa el modo CLI, que no inicializa contexto gráfico |
