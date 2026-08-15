```text
╔═══════════════════════════════════════════════════════════════════════════════════╗
║                                                                                   ║
║  ██████╗  ██████╗  ██████╗ ████████╗ ██████╗  █████╗ ██╗   ██╗███████╗███████╗    ║
║  ██╔══██╗██╔═══██╗██╔═══██╗╚══██╔══╝██╔════╝ ██╔══██╗██║   ██║██╔════╝██╔════╝    ║
║  ██████╔╝██║   ██║██║   ██║   ██║   ██║      ███████║██║   ██║███████╗█████╗      ║
║  ██╔══██╗██║   ██║██║   ██║   ██║   ██║      ██╔══██║██║   ██║╚════██║██╔══╝      ║
║  ██║  ██║╚██████╔╝╚██████╔╝   ██║   ╚██████╗ ██║  ██║╚██████╔╝███████║███████╗    ║
║  ╚═╝  ╚═╝ ╚═════╝  ╚═════╝    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚══════╝    ║
║                                                                                   ║
║                        M A C O S   I N S P E C T O R                              ║
║               Forensic diagnostics · Built in Rust · v0.1.0                       ║
╚═══════════════════════════════════════════════════════════════════════════════════╝
```

[![CI macOS](https://github.com/vladimiracunadev-create/rootcause-macos-inspector/actions/workflows/ci.yml/badge.svg)](https://github.com/vladimiracunadev-create/rootcause-macos-inspector/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-macOS%2013%2B-lightgrey.svg)](docs/REQUIREMENTS.md)
[![Version](https://img.shields.io/badge/version-0.1.0-green.svg)](docs/ROADMAP.md)

📘 **[Manual de usuario →](docs/MANUAL_USUARIO.md)** ·
🏗️ **[Arquitectura →](docs/ARCHITECTURE.md)** ·
🛡️ **[Qué detecta →](docs/DETECCION_AMENAZAS.md)** ·
📑 **[Índice →](docs/INDEX.md)**

---

**RootCause es un software forense de ciberseguridad para macOS**, escrito en **Rust**.

Nace de una idea que es su razón de existir: **cualquier distorsión anómala de los recursos o de la
configuración de un equipo puede ser el primer indicio de que algo está ocurriendo.** No solo
lentitud: también un LaunchDaemon que apareció anoche, Gatekeeper apagado, una app con Acceso total
al disco que nadie recuerda haber autorizado, un binario sin firmar hablando con cuatro destinos
públicos o un equipo desconocido en tu segmento de red.

RootCause **vigila esas distorsiones de forma agnóstica** —no necesita saber *qué* amenaza es para
notar que algo se comporta distinto—, **correlaciona las señales** en incidentes y **explica la
causa raíz con evidencia**.

> **Diagnóstico primero. Intervención después.**

Es un **sensor forense y de apoyo a la decisión**, no un antivirus ni un EDR: no elimina malware ni
bloquea por firma. Detecta indicios de comportamiento, deja registro y **complementa** a las
defensas nativas de macOS y a tu solución de seguridad.

Es la edición macOS del mismo producto que
[**RootCause Windows Inspector**](https://github.com/vladimiracunadev-create/rootcause-windows-inspector):
misma filosofía, misma arquitectura, superficies nativas distintas.

---

## ⚡ Inicio rápido

```bash
# 1. Verificar el entorno (Rust y utilidades de macOS)
./scripts/verify-environment.sh

# 2. Compilar
cargo build --release

# 3. Ejecutar la interfaz gráfica
./target/release/rootcause

# …o el modo consola
./target/release/rootcause status
```

Para empaquetar como `.app` y `.dmg` → [`docs/PACKAGING_MACOS.md`](docs/PACKAGING_MACOS.md).

---

## 🔍 Qué problema resuelve

Preguntas concretas que macOS no responde bien en una sola vista:

| Pregunta | Cómo RootCause ayuda |
|---|---|
| ¿Qué se ejecuta al arrancar mi Mac y no puse yo? | Inventario de LaunchAgents/LaunchDaemons con detección de cambios vs baseline |
| ¿Gatekeeper y SIP siguen activos? | Tab Seguridad con la evidencia del comando que lo consultó |
| ¿Qué app tiene Acceso total al disco o puede leer mi teclado? | Lectura directa de las bases TCC, con severidad por servicio |
| ¿Están al día las firmas de XProtect? | Versión y antigüedad de cada bundle de definiciones de Apple |
| ¿Qué proceso habla con Internet y desde qué ruta? | `lsof` por proceso + verificación de firma con `codesign` |
| ¿Hay algún equipo nuevo en mi red? | Vecinos ARP contra una baseline de "red conocida" |
| ¿Qué carpeta se comió el disco? | Medición acotada de cachés con limpieza segura de dos pasos |

---

## 🛡️ Las siete superficies que observa

### 1 · Persistencia (`LaunchAgents`, `LaunchDaemons`, login items, `cron`)

Es donde vive un implante que quiere sobrevivir a un reinicio. RootCause inventaría las cinco
carpetas de launchd, lee cada `.plist` y clasifica el riesgo con señales acumulativas: binario en
ruta temporal, nombre oculto, `Label` que imita a `com.apple.*` fuera de las carpetas del sistema,
firma ausente, `KeepAlive`, intervalos muy cortos o un intérprete en vez de un binario propio.

### 2 · Procesos

Consumo, ruta, usuario, línea de comandos y **firma de código** (`codesign`). Un binario sin firma
fuera de las rutas del sistema es una de las señales más fuertes que existen en macOS.

### 3 · Controles de seguridad nativos

Gatekeeper (`spctl`), SIP (`csrutil`), FileVault (`fdesetup`), firewall de aplicaciones y modo
encubierto (`socketfilterfw`) y acceso remoto SSH. Cada control muestra **la salida cruda del
comando** que lo respondió: nada de estados inventados.

### 4 · XProtect y familia antimalware

Versión y antigüedad de `XProtect`, `XProtect Remediator` y `MRT`. Una definición de hace meses
casi siempre significa que las actualizaciones automáticas están rotas — un hallazgo en sí mismo.

### 5 · Permisos de privacidad (TCC)

Quién puede grabar la pantalla, leer el teclado, usar el micrófono o acceder a todo el disco,
leído directamente de `TCC.db`. Requiere que RootCause tenga Acceso total al disco; si no lo tiene,
**lo dice** en vez de mostrar una lista vacía.

### 6 · Red

Conexiones activas por proceso (`lsof -i` en modo campo), con marcado de destinos públicos y de
puertos a la escucha expuestos a toda la red. Y los equipos vecinos del segmento contra una
baseline de red conocida: un cambio de MAC en la puerta de enlace se reporta como **crítico**.

### 7 · Almacenamiento

Cachés y temporales medidos por raíz con tope de entradas, y limpieza segura acotada a
`~/Library/Caches`, solo lo no usado en 24 h, saltando lo que esté en uso.

---

## 🧠 El motor de baseline

La idea central del producto cabe en un párrafo:

> **La primera captura se guarda en silencio como "estado bueno conocido".** A partir de ahí, cada
> superficie vigilada se compara contra esa foto y todo cambio se clasifica como NUEVA, MODIFICADA
> o ELIMINADA. Los cambios son **pegajosos**: se siguen reportando hasta que alguien los acepta
> explícitamente. Una alerta que se auto-silencia tras un reinicio es peor que no tenerla.

Superficies con baseline: persistencia, controles de seguridad, permisos TCC sensibles y equipos de
la red local.

---

## 🖥️ La interfaz

Doce secciones en una barra lateral, agrupadas por la pregunta que responden:

| Sección | Descripción |
|---|---|
| **Resumen** | Veredicto, semáforo, tendencias de CPU/memoria/escritura y alertas priorizadas |
| **Procesos** | Tabla con filtro de severidad, firma de código y finalización controlada |
| **Conexiones** | Sockets por proceso, destinos públicos y puertos expuestos |
| **Red** | Vecinos del segmento, escaneo profundo bajo demanda y baseline de red conocida |
| **Persistencia** | LaunchAgents/Daemons, login items y `cron` con estado vs baseline |
| **Seguridad** | Gatekeeper, SIP, FileVault, firewall, SSH y XProtect, con evidencia |
| **Privacidad** | Permisos TCC concedidos, con explicación de qué permite cada uno |
| **Almacenamiento** | Cachés medidas y limpieza segura de dos pasos |
| **Historial** | Capturas SQLite, incidentes persistidos y auditoría de acciones |
| **Configuración** | Tema, idioma ES/EN, umbrales y qué superficies se vigilan |
| **Manual** | Guía integrada: qué hace cada sección y qué permisos puede pedir |
| **Acerca** | Versión, licencia, equipo y contexto de ejecución |

La interfaz **nunca se bloquea**: el motor vive en un hilo propio y se comunica por canales, así
que una captura lenta no congela la ventana.

Atajos: `F5` actualizar · `⌘E` exportar JSON · `⌘R` generar reporte forense.

---

## 💻 CLI completa

Todo lo que hace la GUI se puede hacer desde consola, y casi todo acepta `--json`:

```bash
rootcause status              # Veredicto completo del equipo
rootcause persistence         # LaunchAgents/Daemons con estado vs baseline
rootcause security            # Gatekeeper, SIP, FileVault, firewall, SSH
rootcause tcc --sensitive     # Permisos de privacidad sensibles concedidos
rootcause xprotect            # Antigüedad de las definiciones de Apple
rootcause connections         # Conexiones activas por proceso
rootcause network --deep      # Barrido activo del segmento local
rootcause report              # Reporte forense en Markdown
rootcause --help              # Todos los comandos
```

Referencia completa → [`docs/COMMANDS.md`](docs/COMMANDS.md).

---

## 🗂️ Ediciones del producto

| Modalidad | Tipo | Estado | Cómo se compila |
|---|---|---|---|
| **GUI Desktop** | Núcleo principal | Producción | `cargo build --release` |
| **CLI-only** | Núcleo alternativo | Producción | `cargo build --release --no-default-features` |
| **App bundle `.app`** | Distribución | Producción | `./scripts/package-app.sh` |
| **Imagen `.dmg`** | Distribución | Producción | `./scripts/package-dmg.sh` |
| **Cask de Homebrew** | Adaptador | Plantilla | [`packaging/homebrew/`](packaging/homebrew/) |

---

## 🔐 Permisos que puede pedir

RootCause **no pide permisos en silencio**. Solo estos dos, y siempre por una acción tuya:

| Permiso | Para qué | Cuándo |
|---|---|---|
| **Acceso total al disco** | Leer `TCC.db` y auditar los permisos de privacidad | Lo concedes tú en Ajustes; sin él, la sección lo declara |
| **Automatización** | Consultar los login items vía System Events | Solo al pulsar «Consultar login items» |

Detalle completo → [`docs/PERMISOS_MACOS.md`](docs/PERMISOS_MACOS.md).

---

## 🔏 Privacidad

**Todo el análisis es local.** No hay telemetría, ni servidor, ni envío de datos en ninguna capa
del producto. La única salida a la red posible es el adaptador de IA opcional, que está **apagado
por defecto**, requiere configurar un endpoint y una clave en una variable de entorno, y envía
únicamente el incidente ya resumido — nunca la captura completa, ni rutas de usuario, ni permisos
TCC.

Política completa → [`docs/POLITICA_DE_PRIVACIDAD_LOCAL.md`](docs/POLITICA_DE_PRIVACIDAD_LOCAL.md).

---

## 📐 Estructura del repositorio

```text
rootcause-macos-inspector/
├── Cargo.toml              ← versión, features (gui / cli-only), dependencias
├── README.md · LICENSE · SECURITY.md
├── docs/                   ← documentación de arquitectura, uso, operación y producto
├── landing/                ← página del producto (GitHub Pages)
├── packaging/
│   ├── macos/              ← Info.plist y entitlements del .app
│   └── homebrew/           ← plantilla de cask
├── scripts/                ← verify, build, package-app, package-dmg, ci-local
└── src/
    ├── main.rs             ← entrada: despacha CLI o GUI según argumentos
    ├── cli.rs              ← CLI completa
    ├── app.rs              ← interfaz egui + hilo de trabajo
    ├── config.rs           ← configuración operativa y umbrales
    ├── i18n.rs             ← traducción local ES/EN
    ├── meta.rs             ← constantes del producto
    ├── models.rs           ← modelos de dominio serializables
    └── services/
        ├── inspector.rs    ← orquestador de la captura
        ├── macos.rs        ← adaptador del sistema (comandos nativos)
        ├── launchd.rs      ← LaunchAgents, LaunchDaemons, login items, cron
        ├── security.rs     ← Gatekeeper, SIP, FileVault, firewall, XProtect
        ├── tcc.rs          ← permisos de privacidad (TCC.db)
        ├── network.rs      ← conexiones por proceso (lsof)
        ├── netscan.rs      ← vecinos de red (ARP)
        ├── temp_scan.rs    ← cachés y limpieza segura
        ├── rules.rs        ← clasificación, alertas e incidentes
        ├── anomaly.rs      ← heurísticas de comportamiento
        ├── baseline.rs     ← motor genérico de cambios vs estado bueno conocido
        ├── persistence.rs  ← SQLite: historial, incidentes, auditoría, baselines
        ├── resilience.rs   ← salud del propio agente
        ├── report.rs       ← reporte forense en Markdown
        └── ai.rs           ← adaptador IA opcional (apagado por defecto)
```

---

## 🚀 Validación automática

- **`ci.yml`** — formato, `clippy -D warnings`, tests y build release en `macos-latest`
- **`release-macos.yml`** — binario universal (arm64 + x86_64), `.app`, `.dmg` y `SHA256SUMS.txt`
- **`deploy-landing.yml`** — publica la landing en GitHub Pages
- Réplica local de la CI con `./scripts/ci-local.sh`

> La CI aumenta la confianza, pero no reemplaza la prueba manual en un Mac real con los permisos
> concedidos.

---

## ⚠️ Limitaciones honestas

- No se entrega binario precompilado firmado ni notarizado por Apple.
- Sin **Acceso total al disco**, la sección de privacidad no puede leer `TCC.db`.
- Sin privilegios de root, `lsof` solo ve los sockets del propio usuario.
- El escaneo de cachés es deliberadamente acotado; no indexa el disco completo.
- Los vecinos ARP no equivalen a un IDS ni a un análisis forense de red.
- RootCause **no elimina malware**: señala dónde mirar y deja evidencia.
- `block-ip` **no aplica** reglas de firewall: entrega el comando `pfctl` exacto para que lo
  ejecutes conscientemente.

---

## 📚 Rutas de lectura recomendadas

| Perfil | Documento |
|---|---|
| 👤 Usuario final | [`MANUAL_USUARIO.md`](docs/MANUAL_USUARIO.md) · [`MANUAL_PARA_NOVATOS.md`](docs/MANUAL_PARA_NOVATOS.md) |
| 🧑‍💻 Desarrollador | [`ARCHITECTURE.md`](docs/ARCHITECTURE.md) · [`BUILD_MACOS.md`](docs/BUILD_MACOS.md) |
| 🛡️ Seguridad | [`DETECCION_AMENAZAS.md`](docs/DETECCION_AMENAZAS.md) · [`HEURISTICAS.md`](docs/HEURISTICAS.md) · [`PERSISTENCIA_MACOS.md`](docs/PERSISTENCIA_MACOS.md) |
| 🔐 Permisos | [`PERMISOS_MACOS.md`](docs/PERMISOS_MACOS.md) · [`POLITICA_DE_PRIVACIDAD_LOCAL.md`](docs/POLITICA_DE_PRIVACIDAD_LOCAL.md) |
| 📦 Distribución | [`PACKAGING_MACOS.md`](docs/PACKAGING_MACOS.md) · [`RELEASE_CHECKLIST.md`](docs/RELEASE_CHECKLIST.md) |
| 📑 Todo | [`INDEX.md`](docs/INDEX.md) |

---

## 📄 Licencia

Apache 2.0 — ver [`LICENSE`](LICENSE).

## ✍️ Autor

Vladimir Acuña · [@vladimiracunadev-create](https://github.com/vladimiracunadev-create)
