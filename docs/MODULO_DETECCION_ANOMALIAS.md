# Módulo de detección de anomalías (V1)

Descripción de la implementación real que hay hoy en el repositorio, no de la que nos gustaría
tener. Corresponde a [`REQ-SEC-001`](requirements/REQ-SEC-001-deteccion-comportamiento-anomalo.md).

Código: [`src/services/anomaly.rs`](../src/services/anomaly.rs).

## Qué es

Un detector local que observa el comportamiento de los procesos entre capturas y emite eventos
cuando una señal se **sostiene**. No usa firmas, no consulta servicios externos y no identifica
familias de malware.

## Arquitectura interna

```text
DetectionInput ──▶ AnomalyTracker.analyze() ──▶ Vec<AnomalyEvent>
   │                      │
   │                      ├── history: HashMap<pid, ProcessHistory>
   │                      │      rachas de CPU, escritura y memoria
   │                      └── respawns: HashMap<nombre, RespawnTrace>
   │
   ├── processes: &[ProcessInsight]
   ├── connections: &[ConnectionInsight]
   └── config: &AnomalyConfig
```

El tracker vive dentro del `InspectorService` y sobrevive entre capturas: **ahí está el valor**.
Sin memoria, un detector solo puede ver instantes, y un instante nunca distingue un pico de una
tendencia.

## Las ocho heurísticas

| Evento (`kind`) | Condición | Riesgo | Requiere racha |
|---|---|---|---|
| `sustained-cpu` | CPU ≥ 55 % durante 3 muestras | Medio | Sí |
| `memory-growth` | +250 MB sobre la línea base durante 2 muestras | Medio | Sí |
| `aggressive-write` | ≥ 120 MB por intervalo durante 2 muestras | Alto | Sí |
| `unusual-outbound` | ≥ 4 destinos públicos distintos | Alto | No |
| `local-scan` | ≥ 8 equipos del segmento contactados | Alto | No |
| `suspicious-path` | Ejecutable en ruta temporal o compartida | Alto | No |
| `unsigned-binary` | Sin firma y fuera de rutas del sistema | Medio | No |
| `fast-respawn` | 2 cambios de PID en < 180 s | Alto | Sí |

## Las tres decisiones que evitan el ruido

### 1 · Lista de confianza

Un proceso queda exento si su nombre está en `trusted_process_names` o si está **firmado por Apple
y vive en una ruta del sistema**. Si eso está comprometido, el problema es de otro orden y ninguna
heurística de espacio de usuario lo va a resolver.

### 2 · Excepción de red para software instalado con normalidad

`unusual-outbound` y `local-scan` **no aplican** a binarios que viven en `/Applications`, `/usr` o
`/System` y tienen firma válida. Un navegador hablando con veinte destinos es lo que hace un
navegador. La señal está en que lo haga algo que vive fuera de esas rutas.

Sin esta excepción, la primera captura de cualquier Mac marcaba Chrome como crítico. Lo comprobamos
ejecutándolo, no razonándolo.

### 3 · Reapariciones solo en procesos de instancia única

Un nombre con varias instancias vivas a la vez (`Google Chrome Helper`, `mdworker_shared`) va y
viene constantemente por diseño. Si se vigilan, la primera captura los marca a todos como
reapariciones. Solo se siguen los nombres con **una única instancia**.

Los tres casos están cubiertos por tests con nombre explícito, para que no vuelvan.

## Salida: `AnomalyEvent`

Cada evento lleva lo necesario para actuar sin volver a la herramienta:

| Campo | Contenido |
|---|---|
| `severity` / `score` | Riesgo y puntaje |
| `kind` / `title` | Tipo estable y título legible |
| `process_name`, `pid`, `exe_path`, `user` | Quién |
| `cpu_percent`, `memory_mb`, `io_write_mb_delta` | Cuánto |
| `summary` | Qué pasó, en una frase |
| `root_cause_hypothesis` | Qué podría explicarlo |
| `recommended_action` | Qué hacer a continuación |
| `evidence` | Pares etiqueta/valor verificables |

## Cómo se integra

1. `inspector.rs` llama a `analyze()` con la captura actual.
2. Los eventos de cambio contra baseline (persistencia, seguridad, TCC, red) se **fusionan** con
   estos y todo se reordena junto por severidad y puntaje, para que un cambio grave detectado en
   otra fase no quede fuera del recorte de alertas.
3. `rules::build_alerts()` los convierte en alertas priorizadas.
4. `rules::derive_incident()` deriva el incidente dominante, que se persiste en SQLite.

## Configuración

Todos los umbrales viven en `rootcause-config.json` bajo `anomaly`, y se editan también desde la
sección Configuración de la interfaz. Los interruptores permiten apagar la vigilancia de cada
superficie por separado.

> Si una heurística genera ruido en tu equipo, **súbele el umbral** en vez de apagar la detección:
> una detección apagada no avisa de nada.

## Límites reconocidos

- Muestreo, no intercepción: lo que ocurre entre dos capturas no se ve.
- Sin correlación entre superficies todavía: un mismo binario con persistencia nueva y tráfico
  inusual puntúa como dos señales sueltas, no como una sola más grave. Es la Fase 3 del
  [`PLAN_MAESTRO.md`](PLAN_MAESTRO.md).
- Los umbrales son fijos por configuración, no adaptativos al perfil del equipo.
