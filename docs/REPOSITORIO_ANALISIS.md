# Análisis del repositorio

Qué hay dentro, cuánto pesa cada parte y qué revela sobre cómo está construido. Pensado para quien
evalúa el proyecto sin tiempo de leerlo entero.

## Composición

| Área | Contenido |
|---|---|
| `src/` | 18 archivos Rust: motor, superficies, análisis, persistencia, GUI y CLI |
| `docs/` | 25+ documentos de arquitectura, uso, operación, producto y requisitos |
| `scripts/` | 6 scripts ejecutables: entorno, CI local, empaquetado, release |
| `.github/workflows/` | 3 workflows: CI, release y publicación de la landing |
| `packaging/` | `Info.plist`, entitlements y plantilla de cask |
| `landing/` | Página del producto, sin dependencias externas |

## Distribución del código

| Módulo | Papel |
|---|---|
| `app.rs` | Interfaz completa: 12 secciones, tema, hilo de trabajo |
| `services/inspector.rs` | Orquestador de la captura |
| `cli.rs` | CLI completa con `--json` |
| `models.rs` | Modelos de dominio serializables |
| `services/anomaly.rs` | Heurísticas con memoria entre capturas |
| `services/rules.rs` | Clasificación, alertas e incidentes |
| `services/*.rs` | Una superficie por archivo |

La proporción dice algo: la interfaz es la parte más grande, y el análisis está repartido en
módulos pequeños y probados. Es lo esperable en una herramienta cuyo valor está en **presentar bien
lo que descubre**.

## Cobertura de pruebas

**112 tests**, todos en el mismo archivo que el código que prueban. Distribución por módulo:

| Módulo | Qué se prueba |
|---|---|
| `rules` | Clasificación, alertas, derivación de incidentes, huella |
| `anomaly` | Rachas, listas de confianza y los tres falsos positivos corregidos |
| `network` / `netscan` | Parseo de `lsof` y `arp`, IPs públicas, normalización de MAC |
| `tcc` | Ambos esquemas de `TCC.db`, filtro de permisos sensibles |
| `security` | Umbrales de antigüedad, severidad de estados desconocidos |
| `launchd` | Clasificación de riesgo con entradas sintéticas |
| `config` / `persistence` / `report` / `ai` / `cli` / `app` | Serialización, claves estables, formato, banderas |

Los tests **no requieren un macOS concreto**: la lógica de decisión trabaja sobre estructuras, así
que se puede probar sin depender del estado de una máquina. Es el resultado directo de separar
recolección de análisis.

## Señales de calidad

| Señal | Estado |
|---|---|
| `cargo fmt` en CI | ✅ Obligatorio |
| `clippy -D warnings` | ✅ Sin excepciones |
| Dos ediciones compiladas en CI | ✅ Completa y CLI-only |
| Humo de la CLI en CI | ✅ Ejecuta comandos reales en el runner |
| Documentación de límites | ✅ Documento propio |
| Falsos positivos con test de regresión | ✅ Tres casos |
| Dependencias | 9 directas, todas permisivas |

## Lo que revela el historial

El primer commit trae el producto completo y validado. El segundo añade el orquestador de release y
una corrección que solo aparece al ejecutar en un volumen exFAT. El tercero corrige dos lints que
solo existen en la versión de Rust del runner de CI, no en la local.

Es un patrón reconocible: **los problemas se descubren ejecutando, no razonando**, y cada
corrección deja rastro documentado de por qué existía el problema.

## Deuda reconocida

Escrita aquí para que no haya que descubrirla leyendo:

| Deuda | Dónde | Impacto |
|---|---|---|
| Sin correlación entre superficies | `rules.rs` | Un binario con tres señales puntúa como tres señales sueltas |
| Umbrales fijos | `config.rs` | No se adaptan al perfil del equipo |
| Tabla OUI corta | `netscan.rs` | Muchos fabricantes salen sin identificar |
| Sin diff visual de capturas | `app.rs` | El historial se lee como lista, no como comparación |
| Sin firma ni notarización | Distribución | Gatekeeper avisa al abrir el `.app` |

Todo está en el [`ROADMAP.md`](ROADMAP.md) con su prioridad.

## Cómo evaluarlo rápido

```bash
./scripts/ci-local.sh          # 5 minutos: formato, lint, 112 tests, dos ediciones
cargo run --release -- status  # 10 segundos: veredicto real del equipo
```

Y tres archivos que resumen el criterio del proyecto:
[`ARCHITECTURE.md`](ARCHITECTURE.md), [`HEURISTICAS.md`](HEURISTICAS.md) y
[`LIMITACIONES.md`](LIMITACIONES.md).
