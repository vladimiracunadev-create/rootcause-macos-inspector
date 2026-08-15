# 👔 Documento para reclutadores

> Guía rápida para reclutadores y líderes técnicos: qué es RootCause macOS Inspector, qué problema
> resuelve y qué capacidades profesionales demuestra.

## 1 · Resumen ejecutivo

**RootCause macOS Inspector** es un sensor forense de ciberseguridad para macOS, escrito en
**Rust**, con interfaz gráfica nativa y CLI completa.

Observa siete superficies del sistema —persistencia de launchd, controles de seguridad nativos,
definiciones antimalware de Apple, permisos de privacidad, procesos con verificación de firma, red
y almacenamiento—, las compara contra un estado bueno conocido y explica lo que cambió con la
evidencia al lado.

No es un limpiador ni un antivirus: es una herramienta de **diagnóstico, observabilidad y apoyo a
la decisión**.

## 2 · Qué problema resuelve

Un usuario o un equipo de soporte observa síntomas vagos: «el Mac va lento», «se abre algo solo al
arrancar», «no sé qué app tiene permiso para grabar la pantalla». macOS tiene la información, pero
repartida entre siete sitios distintos y sin memoria de lo que había ayer.

RootCause responde tres preguntas concretas:

1. **¿Qué cambió en este equipo desde la última vez que lo miré?**
2. **¿Qué se ejecuta solo al arrancar, y quién firmó ese binario?**
3. **¿Siguen encendidas las defensas que creo que están encendidas?**

## 3 · Qué demuestra técnicamente

### Rust de producción

- Arquitectura en cuatro capas con separación estricta entre recolección y análisis, lo que permite
  probar la lógica de decisión sin depender del estado de una máquina concreta.
- **112 tests** que cubren clasificación, parseo, heurísticas y detección de cambios.
- `clippy -D warnings` en CI: ninguna advertencia llega a `main`.
- Dos ediciones desde el mismo código mediante *feature flags*: completa con GUI y CLI-only.

### Concurrencia con criterio

El motor vive en un hilo propio y se comunica con la interfaz por canales `mpsc`. Es la diferencia
entre una herramienta que se usa y una que se abandona a la tercera vez que congela la ventana.

### Conocimiento real de la plataforma

No es una aplicación multiplataforma con un `#[cfg(macos)]`. Cada superficie está traducida a lo
que macOS usa de verdad: `launchd` y sus cinco carpetas de `.plist`, ambos esquemas históricos de
`TCC.db`, `lsof` en modo campo para tolerar nombres de proceso con espacios, `codesign` con
presupuesto por captura para no pagar 600 procesos por refresco.

### Diseño de producto, no solo de código

- Falsos positivos **detectados ejecutándolo contra un sistema real** y corregidos con un test que
  documenta el caso: ayudantes multi-instancia contados como reapariciones, primer delta de E/S
  tomado como escritura del intervalo, navegador legítimo marcado por hablar con muchos destinos.
- Decisiones difíciles documentadas con su razonamiento, incluidas las que limitan el producto.

### Ingeniería de entrega

- CI en `macos-latest`: formato, lint, tests, ambas ediciones y humo de la CLI.
- Release automatizada: binario universal (arm64 + x86_64), `.app`, `.dmg` y hashes.
- Orquestador de release en un comando, con seis comprobaciones antes de tocar el remoto.
- **19+ documentos** de arquitectura, operación, límites y requisitos.

## 4 · Decisiones que conviene mirar

Estas son las que mejor muestran criterio de ingeniería:

| Decisión | Por qué importa |
|---|---|
| Presupuesto de firmas con caché por ruta | Reconocer un coste real y acotarlo, en vez de ignorarlo o eliminar la función |
| Las carpetas de Apple se omiten por defecto | Distinguir señal de ruido: cientos de entradas inmutables no aportan |
| La primera captura se siembra en silencio | Estrenar la herramienta no debe generar cien alertas falsas |
| Los cambios son pegajosos | Una alerta que se auto-silencia tras un reinicio es peor que no tenerla |
| `block-ip` no bloquea | Modificar el firewall del equipo no debe ser efecto secundario de un botón |

## 5 · Familia de producto

Es la cuarta edición de la misma idea, cada una con sus superficies nativas:

| Edición | Plataforma | Tecnología |
|---|---|---|
| **Windows Inspector** | Windows 10/11 | Rust + egui |
| **macOS Inspector** | macOS 13+ | Rust + egui |
| **Web Inspector** | Navegador | Extensión MV3 + Node |
| **Mobile Inspector** | Android / iOS | Flutter |

Demuestra capacidad de **sostener una arquitectura común entre plataformas** sin forzar simetrías
artificiales: la idea y el motor de baseline se comparten; las superficies son idiomáticas de cada
sistema.

## 6 · Cómo evaluarlo en 10 minutos

```bash
git clone https://github.com/vladimiracunadev-create/rootcause-macos-inspector
cd rootcause-macos-inspector
./scripts/ci-local.sh        # formato, lint, 112 tests, ambas ediciones
cargo run --release -- status
```

Y para leer, en este orden:

1. [`ARCHITECTURE.md`](ARCHITECTURE.md) — cómo está construido y por qué.
2. [`HEURISTICAS.md`](HEURISTICAS.md) — cada umbral con su justificación.
3. [`LIMITACIONES.md`](LIMITACIONES.md) — qué no hace, escrito sin adornos.
4. `src/services/anomaly.rs` — la lógica con memoria y sus tests.
