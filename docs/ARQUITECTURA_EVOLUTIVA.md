# Arquitectura evolutiva

Cómo está preparado el código para crecer sin reescribirse, y qué extensiones ya tienen su sitio
reservado.

## Los tres puntos de extensión

### 1 · Superficies

Una superficie es un módulo en `src/services/` que traduce una parte de macOS a los modelos de
dominio. Añadir una es:

1. Escribir el módulo con su función de escaneo y sus tests.
2. Llamarlo desde `inspector.rs::collect_snapshot()`.
3. Añadir su campo a `SystemSnapshot`.
4. Dibujar su sección en `app.rs` y su comando en `cli.rs`.

Nada de eso toca el motor de baseline, las reglas ni la persistencia. Las siete superficies
actuales se añadieron así.

### 2 · Superficies vigiladas

Para que una superficie nueva se compare contra el estado bueno conocido, basta con:

```rust
pub const MI_SUPERFICIE: SurfaceSpec = SurfaceSpec {
    id: "mi-superficie",
    title_added: "Elemento nuevo detectado",
    title_modified: "Elemento modificado",
    title_removed: "Elemento eliminado",
    summary_noun: "El elemento",
    risk_on_change: RiskLevel::High,
};
```

…y convertir sus elementos a `WatchedItem`. El motor genérico se encarga del diff, de sembrar la
primera foto, de los ítems sintéticos eliminados y de generar los eventos de cambio.

Cuatro superficies ya lo usan: persistencia, controles de seguridad, permisos TCC y red.

### 3 · Heurísticas

Una heurística nueva es un bloque dentro de `AnomalyTracker::analyze()` que emite un
`AnomalyEvent` mediante el constructor común `process_event()`. Si necesita memoria entre capturas,
se le añade un campo a `ProcessHistory`.

El resto del sistema —alertas, incidentes, persistencia, interfaz— la consume sin cambios, porque
todos trabajan sobre `AnomalyEvent`, no sobre heurísticas concretas.

## Qué está preparado y aún no se usa

| Preparado | Dónde | Para qué |
|---|---|---|
| Eventos del log unificado | `EventRecord` en el modelo, `security_events()` en el motor | Ya se consultan bajo demanda; falta la sección propia |
| Adaptador de IA | `services/ai.rs` | Funciona; apagado por defecto |
| Notificaciones | `macos::notify()` + política en configuración | Ya avisa de las señales críticas |
| Auditoría consultable | `load_recent_audits()` | Ya se muestra; falta filtrado |
| Múltiples bases TCC | `read_database()` acepta ruta y etiqueta | Añadir otra base es una línea |

## Decisiones que mantienen la puerta abierta

### Modelos serializables desde el principio

Cada estructura del dominio deriva `Serialize`/`Deserialize`. Eso permitió, sin rediseñar nada:
exportar JSON, persistir incidentes, guardar configuración y ofrecer `--json` en la CLI. Cualquier
salida futura (CSV, syslog, webhook) es un consumidor más del mismo modelo.

### Campos con `#[serde(default)]`

Un `SystemSnapshot` guardado por una versión anterior se sigue leyendo tras añadir campos nuevos.
Sin esto, cada versión rompería el historial del usuario.

### El adaptador del sistema aislado

Todo lo que ejecuta comandos vive en `services/macos.rs`. Si una versión futura de macOS cambia la
salida de `spctl`, hay **un** sitio donde arreglarlo. Y si algún día se sustituye un comando por
una API nativa, el resto del código no se entera.

### Reglas puras y probadas

`rules.rs` no toca el sistema: recibe estructuras y devuelve estructuras. Por eso se puede cambiar
la política de clasificación con la seguridad de una suite de tests que no depende del estado de
ninguna máquina.

## Lo que sí exigiría rediseño

Con honestidad, no todo es una extensión:

| Cambio | Por qué rompe el diseño |
|---|---|
| Intercepción en tiempo real | El modelo es de muestreo periódico; Endpoint Security invierte el flujo de control |
| Agente de flota | Implica servidor, transporte y autenticación: rompe el análisis local |
| Reglas declarativas externas | Hoy la lógica es código compilado y probado; un motor de reglas necesita su propio modelo de confianza |
| Cifrado del historial | Habría que resolver dónde vive la clave, y eso cambia el modelo de amenaza |

Cada uno está en el [`ROADMAP.md`](ROADMAP.md) marcado como decisión abierta, no como tarea
pendiente. La diferencia importa.
