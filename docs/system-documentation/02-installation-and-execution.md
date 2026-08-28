# 02 · Instalación y ejecución

> Cómo pasar de un repositorio recién clonado a un binario funcionando, y qué hacer cuando
> algo no compila o no devuelve datos. Todos los comandos de este documento se ejecutan
> desde la raíz del repositorio salvo que se indique otra cosa.

---

## 1. Requisitos previos

### 1.1 Sistema operativo

| Requisito | Valor | Dónde está declarado |
|---|---|---|
| Sistema | macOS 13 (Ventura) o superior | `packaging/macos/Info.plist` → `LSMinimumSystemVersion` |
| Arquitectura | Apple Silicon (`aarch64`) o Intel (`x86_64`) | `.github/workflows/release-macos.yml` |
| Espacio en disco | ~2 GB para `target/` en compilación de release | `INFERENCIA`, medido en una compilación limpia |

El código compila y los tests pasan en versiones anteriores a Ventura, pero el `.app`
declara 13.0 como mínimo y no hay verificación en CI sobre versiones antiguas:
`REQUIERE VALIDACIÓN`.

### 1.2 Cadena de compilación

| Herramienta | Versión mínima | Cómo obtenerla |
|---|---|---|
| Rust (`cargo`, `rustc`) | 1.82 (`Cargo.toml` → `rust-version`) | <https://rustup.rs> |
| Componentes `rustfmt` y `clippy` | los del canal estable | `rust-toolchain.toml` los declara |
| Herramientas de línea de comandos de Xcode | las que instale `xcode-select` | `xcode-select --install` |

`rust-toolchain.toml` fija `channel = "stable"`, así que `rustup` selecciona la toolchain
correcta automáticamente al entrar en el directorio.

> **Rust por Homebrew.** Funciona para compilar y probar, pero **no** para el binario
> universal: `brew install rust` trae solo el target del host. `scripts/package-app.sh`
> lo detecta y se degrada a la arquitectura nativa con un aviso, en vez de fallar.

### 1.3 Utilidades de macOS

Todas vienen con el sistema. `scripts/verify-environment.sh` comprueba estas once:
`/usr/sbin/lsof`, `/usr/sbin/spctl`, `/usr/bin/csrutil`, `/usr/bin/fdesetup`,
`/usr/bin/codesign`, `/usr/sbin/arp`, `/sbin/route`, `/sbin/ifconfig`, `/bin/launchctl`,
`/bin/ps` y `/usr/bin/log`. Si falta alguna, la superficie correspondiente queda vacía y el
producto lo declara; no aborta.

### 1.4 Herramientas opcionales

| Herramienta | Para qué | Sin ella |
|---|---|---|
| `iconutil` | Generar `AppIcon.icns` en el `.app` | `package-app.sh` falla |
| `hdiutil` | Crear el `.dmg` | `package-dmg.sh` falla |
| `python3` | Generar el icono y los PDF de documentación | Icono y PDF no se generan |
| `gh` (GitHub CLI) | Publicar la release con `release-product.sh --publish` | Solo se construye `dist/` |

## 2. Verificación del entorno

```bash
./scripts/verify-environment.sh
```

Informa, no instala. Devuelve `1` si falta algún requisito imprescindible y `0` si el
entorno está listo (aunque haya avisos). Comprueba sistema, cadena de compilación,
utilidades, herramientas de empaquetado, si el terminal tiene Acceso total al disco y si se
está ejecutando como root.

## 3. Obtener el código

```bash
git clone https://github.com/vladimiracunadev-create/rootcause-macos-inspector.git
cd rootcause-macos-inspector
```

## 4. Instalación de dependencias

No hay paso de instalación separado: `cargo` descarga y compila las dependencias declaradas
en `Cargo.toml`, con las versiones exactas fijadas en `Cargo.lock` (326 paquetes
transitivos). `rusqlite` usa la característica `bundled`, así que **compila SQLite desde el
código fuente** y no depende de la librería del sistema; eso alarga la primera compilación.

## 5. Compilación

### 5.1 Edición completa (GUI + CLI)

```bash
cargo build --release
```

Produce `target/release/rootcause`. Es la edición por defecto: la feature `gui` está
activa e incluye `eframe` y `egui`.

### 5.2 Edición CLI-only

```bash
cargo build --release --no-default-features
```

El mismo binario sin interfaz gráfica ni dependencias de ventana. Útil para servidores,
sesiones SSH y contenedores de compilación. `src/main.rs` cambia de comportamiento: sin
argumentos imprime la ayuda en vez de abrir una ventana.

### 5.3 Compilación de desarrollo

```bash
cargo build
cargo run -- status
```

Mucho más rápida de compilar y bastante más lenta de ejecutar. El perfil de release usa
`opt-level = 3`, `lto = true`, `codegen-units = 1` y `strip = true`.

## 6. Ejecución

### 6.1 Interfaz gráfica

```bash
./target/release/rootcause
./target/release/rootcause --gui   # equivalente explícito
```

La ventana abre con 1280 × 800 px y un mínimo de 880 × 600 px para que quepa en un MacBook
de 13 pulgadas. La primera captura se pide antes de dibujar el primer frame.

### 6.2 Línea de comandos

```bash
./target/release/rootcause status            # veredicto completo
./target/release/rootcause status --json     # lo mismo, para encadenar
./target/release/rootcause --help            # los 19 comandos
```

Referencia completa de comandos en [05 · Referencia técnica](05-technical-reference.md) y
en [`docs/COMMANDS.md`](../COMMANDS.md).

### 6.3 Instalación en el `PATH`

```bash
cargo install --path .          # instala en ~/.cargo/bin/rootcause
```

`NO DOCUMENTADO EN EL REPOSITORIO`: el `README.md` no menciona `cargo install`; funciona
porque `Cargo.toml` declara el binario `rootcause`, pero `publish = false` impide publicarlo
en crates.io.

## 7. Configuración inicial

No hace falta ninguna: el producto arranca con valores por defecto razonables y **no crea
el archivo de configuración hasta que se lo pides**.

```bash
./target/release/rootcause config init    # crea el JSON con los valores por defecto
./target/release/rootcause config show    # muestra rutas y configuración efectiva
```

Ubicación: `~/Library/Application Support/RootCauseInspector/rootcause-config.json`.
Detalle campo por campo en [10 · Configuración](10-configuration.md).

## 8. Base de datos

Tampoco requiere pasos: `PersistenceStore::new` crea el directorio de datos y ejecuta
`ensure_schema()` en cada arranque, con sentencias `CREATE TABLE IF NOT EXISTS`. No hay
sistema de migraciones ni scripts SQL externos.

| Elemento | Ruta |
|---|---|
| Historial SQLite | `~/Library/Application Support/RootCauseInspector/rootcause-history.db` |
| Estado del agente | `~/Library/Application Support/RootCauseInspector/rootcause-agent-state.json` |
| Configuración | `~/Library/Application Support/RootCauseInspector/rootcause-config.json` |
| Copia del historial | `~/Library/Application Support/RootCauseInspector/rootcause-history-backup.json` |

Para empezar de cero basta con borrar esos archivos: se recrean solos. Esquema completo en
[07 · Base de datos](07-database.md).

## 9. Permisos de macOS

RootCause funciona sin conceder nada, pero dos superficies quedan limitadas:

| Permiso | Qué habilita | Cómo se concede | Si falta |
|---|---|---|---|
| **Acceso total al disco** | Leer `TCC.db` (sección Privacidad) | Ajustes del Sistema → Privacidad y seguridad → Acceso total al disco → añadir el binario o el `.app` | `TccOverview::readable = false` y una alerta explicativa |
| **Automatización** | Consultar login items vía System Events | Diálogo del sistema al pulsar «Consultar login items» | La lista de login items queda vacía |

El permiso de Acceso total al disco hay que concederlo al **binario concreto** que se
ejecuta: darlo a `Terminal.app` cubre las ejecuciones desde ese terminal, pero no al
`.app` empaquetado, que es otro ejecutable.

Ejecutar como root (`sudo rootcause status`) amplía lo que ve `lsof` a los sockets de todos
los usuarios. No es necesario y el producto no lo pide: `macos::environment` informa del
contexto en vez de escalar.

## 10. Ejecución de pruebas

```bash
cargo test --all-features                  # los 112 tests
cargo test --all-features -- --nocapture   # igual, mostrando stdout (lo que hace la CI)
cargo test services::rules                 # solo un módulo
```

Los tests son unitarios y viven junto al código en módulos `#[cfg(test)]`. Solo uno toca el
sistema real: `services::macos::tests::el_entorno_real_encuentra_las_utilidades_base_de_macos`,
que comprueba que `launchctl` existe. Detalle en
[12 · Pruebas y calidad](12-testing-and-quality.md).

## 11. Réplica local de la integración continua

```bash
./scripts/ci-local.sh
```

Ejecuta, en el mismo orden que `.github/workflows/ci.yml`: versiones, `cargo fmt --check`,
`cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`,
build CLI-only, build completa y humo de la CLI. Si pasa en local, pasa en la CI.

La validación de Markdown de la CI es un job aparte que no cubre este script:

```bash
npx --yes markdownlint-cli2
```

## 12. Empaquetado

```bash
./scripts/package-app.sh              # dist/RootCause.app para la arquitectura nativa
./scripts/package-app.sh --universal  # binario universal arm64 + x86_64 (necesita rustup)
./scripts/package-dmg.sh              # dist/RootCause-0.1.0.dmg y dist/SHA256SUMS.txt
```

`package-dmg.sh` construye el `.app` antes si no existe, y crea la imagen en un directorio
temporal del disco interno porque `hdiutil` solo trabaja sobre APFS o HFS+: si el
repositorio vive en un volumen exFAT o de red, crear la imagen ahí falla con «Operación no
permitida».

Release completa en un comando:

```bash
./scripts/release-product.sh                      # construye dist/ y para ahí
./scripts/release-product.sh --publish --watch    # además etiqueta, publica y espera al workflow
```

Detalle en [13 · Despliegue y operación](13-deployment-and-operations.md).

## 13. Generación de la documentación en PDF

```bash
python3 -m pip install markdown xhtml2pdf
python3 scripts/build-docs-pdf.py            # todos los documentos
python3 scripts/build-docs-pdf.py 07 11      # solo los que empiecen por 07 y 11
python3 scripts/build-docs-pdf.py --check    # solo comprueba dependencias
```

Si prefieres no tocar el Python del sistema, un entorno virtual funciona igual:

```bash
python3 -m venv .venv && .venv/bin/pip install markdown xhtml2pdf
.venv/bin/python scripts/build-docs-pdf.py
```

## 14. Errores frecuentes durante la instalación

| Síntoma | Causa | Solución |
|---|---|---|
| `error: linker 'cc' not found` | Faltan las herramientas de Xcode | `xcode-select --install` |
| `error: failed to run custom build command for libsqlite3-sys` | Compilador de C no disponible o SDK incompleto | Reinstalar herramientas de Xcode; `rustup update` |
| `error: package requires rustc 1.82` | Toolchain antigua | `rustup update stable` |
| `lipo: can't open input file … x86_64-apple-darwin/release/rootcause` | Rust instalado por Homebrew, sin el segundo target | Instalar Rust con `rustup`, o construir sin `--universal` |
| `hdiutil: create failed - Operation not permitted` | El repositorio está en exFAT o en red | Ya lo evita `package-dmg.sh`; si aparece, ejecutar desde un directorio del disco interno |
| Ventana en negro o error de `glow` al abrir la GUI | Contexto gráfico no disponible (sesión SSH, sin pantalla) | Usar la edición CLI-only o un comando del CLI |
| `iconutil: command not found` | Herramientas de Xcode incompletas | `xcode-select --install` |
| La sección Privacidad aparece vacía y avisa | Falta Acceso total al disco | Concederlo al binario o al `.app` (sección 9) |
| `No se observaron conexiones` | `lsof` sin privilegios solo ve los sockets propios | Es el comportamiento esperado; ejecutar con `sudo` si se necesita la vista completa |
| Archivos `._*` en `dist/` | Volumen exFAT que crea bifurcaciones AppleDouble | `export COPYFILE_DISABLE=1`, que ya hace `release-product.sh` |

## 15. Comandos verificados en este análisis

Ejecutados en el commit analizado, sobre macOS 26.3.1 (build 25D2128), Apple Silicon (arm64):

| Comando | Resultado |
|---|---|
| `cargo fmt --all -- --check` | Sin diferencias (código de salida `0`) |
| `cargo clippy --all-targets --all-features -- -D warnings` | Sin advertencias (código de salida `0`) |
| `cargo test --all-features` | `112 passed; 0 failed; 0 ignored`, 1,47 s |
| `git diff --stat -- src/` | 71 inserciones, 0 eliminaciones (solo comentarios `///`) |

Los mismos cuatro pasos los repite la CI en `macos-latest` en cada push, más las dos
compilaciones de release y el humo de la CLI.

---

**Siguiente lectura recomendada:** [03 · Arquitectura](03-architecture.md).
