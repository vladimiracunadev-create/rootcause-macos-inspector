# Arquitectura

## Principio rector

Recolectar datos y decidir qué significan son dos trabajos distintos. Mezclarlos hace imposible
probar el segundo sin un macOS real detrás. Toda la arquitectura sale de ahí.

```text
┌──────────────┐   comandos    ┌──────────────────┐
│    app.rs    │──────────────▶│                  │
│  (egui/GUI)  │◀──────────────│ InspectorService │
└──────────────┘   resultados  │  (hilo propio)   │
┌──────────────┐               │                  │
│    cli.rs    │──────────────▶│                  │
└──────────────┘               └────────┬─────────┘
                                        │
        ┌───────────────────────────────┼───────────────────────────────┐
        │                               │                               │
  ┌─────▼─────┐                   ┌─────▼─────┐                   ┌─────▼─────┐
  │ Superficies│                   │  Análisis │                   │Persistencia│
  │  macos.rs  │                   │  rules.rs │                   │persistence │
  │ launchd.rs │                   │ anomaly.rs│                   │ (SQLite)   │
  │ security.rs│                   │baseline.rs│                   │ resilience │
  │   tcc.rs   │                   └───────────┘                   └───────────┘
  │ network.rs │
  │ netscan.rs │
  │temp_scan.rs│
  └───────────┘
```

## Las cuatro capas

### 1 · Adaptador del sistema (`services/macos.rs`)

Todo lo que implica hablar con macOS pasa por un único módulo: ejecutar utilidades nativas, leer
`sysctl`, verificar firmas y lanzar notificaciones. Tres reglas de la casa:

- **Solo lectura por defecto.** Las únicas funciones que modifican estado son `terminate_process`
  y `reveal_in_finder`, y ambas se auditan en la capa superior.
- **Nada de `sudo` implícito.** Si un dato necesita privilegios y no los hay, se devuelve el error
  tal cual para que la interfaz lo explique, en vez de simular que el dato no existe.
- **Fallo suave.** Una utilidad ausente o sin permiso nunca tumba una captura completa.

### 2 · Superficies

Cada superficie es un módulo independiente que traduce una parte de macOS a los modelos de dominio:

| Módulo | Superficie | Fuente |
|---|---|---|
| `launchd.rs` | Persistencia | `.plist` de las 5 carpetas de launchd, `crontab`, `launchctl list` |
| `security.rs` | Controles nativos y XProtect | `spctl`, `csrutil`, `fdesetup`, `socketfilterfw`, plists de definiciones |
| `tcc.rs` | Permisos de privacidad | `TCC.db` (usuario y sistema) por SQLite en solo lectura |
| `network.rs` | Conexiones por proceso | `lsof -i` en modo campo (`-F`) |
| `netscan.rs` | Vecinos de red | `arp -a -n`, `route`, `ifconfig` |
| `temp_scan.rs` | Cachés y temporales | recorrido acotado del sistema de archivos |

### 3 · Análisis

- **`rules.rs`** — clasifica procesos, construye la lista de alertas priorizada, fija el veredicto
  del semáforo y deriva el incidente. Es función pura sobre estructuras de datos: por eso está
  cubierto por tests sin necesidad de un macOS real.
- **`anomaly.rs`** — heurísticas con memoria entre capturas (CPU sostenido, crecimiento de
  memoria, escritura agresiva, tráfico saliente, barrido local, reapariciones rápidas).
- **`baseline.rs`** — motor genérico de detección de cambios contra el estado bueno conocido.

### 4 · Persistencia

`persistence.rs` guarda cuatro capas en un único SQLite:

1. snapshots compactos para tendencia,
2. incidentes resumidos para correlación y evidencia,
3. auditoría de acciones,
4. baselines de cada superficie vigilada.

`resilience.rs` vigila la salud del propio agente: heartbeat, cierres abruptos, integridad de
configuración y reinicios repetidos.

## Por qué el motor vive en un hilo aparte

Una captura invoca `lsof`, `spctl`, `csrutil`, `codesign` y recorre carpetas de cachés: entre
décimas de segundo y varios segundos. Hacerlo en el hilo de la interfaz congelaría la ventana en
cada refresco.

El motor vive en un hilo propio que recibe `Command` y devuelve `EngineEvent` por canales
`std::sync::mpsc`. La interfaz nunca bloquea: pinta el último estado conocido, muestra que hay
trabajo en curso y recoge el resultado cuando llega.

## Decisiones que merecen explicación

### `lsof` en modo campo

El formato tabular de `lsof` se rompe con nombres de proceso que llevan espacios
(`Google Chrome Helper`). Se pide la salida por campos (`-F`): una línea por dato, prefijada por
su letra. Es fea de leer y trivial de parsear sin ambigüedad.

### Presupuesto de firmas

`codesign` cuesta un proceso por binario. Verificar los ~600 procesos de un Mac en cada captura
sería inaceptable. Se verifica un presupuesto configurable (12 por defecto), priorizando lo que ya
destaca por severidad y lo que vive fuera de las rutas del sistema — que es donde una firma
ausente significa algo. El resultado se cachea por ruta.

### Primera muestra de E/S sembrada

`sysinfo` devuelve el total de E/S acumulado desde que el proceso arrancó. Restar contra cero haría
pasar por "escritura del intervalo" toda la vida del proceso. La primera muestra de cada PID
siembra el contador y reporta delta 0.

### Reapariciones solo en procesos de instancia única

Un nombre con varias instancias vivas a la vez (ayudantes de navegador, workers de Spotlight) va y
viene constantemente por diseño. Vigilarlos convertiría la heurística en una fábrica de ruido, así
que solo se siguen los nombres con una única instancia.

### Las carpetas de Apple se omiten por defecto

`/System/Library/Launch*` tiene cientos de entradas inmutables protegidas por SIP. Incluirlas en la
baseline solo añadiría ruido. `--all` las incluye cuando se quiere el inventario completo.

## Flujo de una captura

1. Latido de resiliencia y refresco de `sysinfo`.
2. Procesos + detalles de `ps` (usuario y línea de comandos) en una sola llamada.
3. Firmas de código dentro del presupuesto, y reclasificación de los procesos afectados.
4. Conexiones, cachés, XProtect, controles de seguridad, TCC, persistencia y red.
5. Diff contra baseline de cada superficie → eventos de cambio.
6. Heurísticas de comportamiento → eventos de anomalía.
7. Fusión, ordenación por severidad y puntaje, y construcción de alertas.
8. Derivación del incidente dominante y persistencia en SQLite.
