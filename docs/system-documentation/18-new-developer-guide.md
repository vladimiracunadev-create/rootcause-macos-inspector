# 18 · Guía para un nuevo desarrollador

> Itinerario de incorporación: qué leer, en qué orden, qué ejecutar y qué tocar primero.
> Pensado para llegar a hacer un cambio útil en tu primer día y entender el sistema entero en
> tu primera semana.

---

## 1. Antes de nada

Tres cosas que conviene saber desde el minuto uno, porque explican casi todas las decisiones
que verás en el código:

1. **El producto detecta cambios, no amenazas.** No hay firmas ni catálogos de malware. Todo
   se apoya en comparar el estado actual con una foto guardada.
2. **La evidencia es parte del producto.** Nunca se muestra un veredicto sin el dato crudo
   que lo respalda. Si añades una comprobación, tiene que traer su evidencia.
3. **Lo que no se sabe se declara.** Un control cuyo estado no se pudo determinar se pinta de
   amarillo, no de verde. Una sección sin permisos dice por qué está vacía.

Si un cambio tuyo rompe alguna de las tres, romperá también algún test: hay pruebas
dedicadas precisamente a fijar esos criterios.

## 2. Día 1 — Poner en marcha el proyecto

```bash
git clone https://github.com/vladimiracunadev-create/rootcause-macos-inspector.git
cd rootcause-macos-inspector
./scripts/verify-environment.sh     # ¿tengo todo?
cargo build --release               # 3–8 minutos la primera vez
./target/release/rootcause status   # el producto, en consola
./target/release/rootcause          # el producto, con interfaz
```

Después, dedica media hora a **usarlo**: pulsa por las doce secciones, mira qué muestra cada
una en tu propio equipo, exporta una captura (`⌘E`) y abre el JSON. Entender qué produce el
sistema hace que el código se lea solo.

Ejecuta también la validación completa, para saber cómo se ve el verde:

```bash
./scripts/ci-local.sh
```

## 3. Día 1 — Lectura mínima

| Orden | Documento | Por qué |
|---|---|---|
| 1 | [`README.md`](../../README.md) | La tesis del producto en cinco minutos |
| 2 | [01 · Descripción general](01-system-overview.md) | Qué hace y para quién |
| 3 | [03 · Arquitectura](03-architecture.md) | Las capas y por qué están separadas así |
| 4 | [04 · Mapa del código](04-code-map.md) | Dónde está cada cosa |

Con eso ya puedes moverte. El resto se lee cuando toque.

## 4. Día 2 — Seguir un flujo completo

La mejor forma de entender el sistema es seguir **una** captura de punta a punta. Abre estos
archivos en este orden y lee solo lo que se indica:

| Paso | Archivo | Qué leer |
|---|---|---|
| 1 | `src/main.rs` | Entero: son 119 líneas y define el arranque |
| 2 | `src/services/inspector.rs` | Solo `collect_snapshot`: es la secuencia completa |
| 3 | `src/services/macos.rs` | `run_capture`, `run_combined` y `process_details` |
| 4 | `src/services/launchd.rs` | `scan_persistence`, `parse_launch_plist` y `classify_entry` |
| 5 | `src/services/baseline.rs` | Entero: son 217 líneas y es el corazón del producto |
| 6 | `src/services/rules.rs` | `classify_process`, `build_alerts` y `derive_incident` |
| 7 | `src/services/persistence.rs` | `ensure_schema` y `replace_baseline` |
| 8 | `src/app.rs` | Solo `spawn_worker` y `update`: el resto es dibujado |

Acompaña la lectura con [06 · Explicación profunda](06-deep-code-explanation.md), que explica
las decisiones no evidentes de cada uno de esos puntos.

**Ejercicio útil:** pon un `dbg!` en `collect_snapshot` después del bloque de anomalías,
ejecuta `cargo run -- status` y observa qué se genera en tu equipo. Recuerda quitarlo antes de
commitear: `clippy` con `-D warnings` no lo permite.

## 5. Día 3 — Cómo está organizado el repositorio

```text
src/
├── main.rs        ← arranque: decide CLI o GUI
├── cli.rs         ← 19 comandos
├── app.rs         ← interfaz (el archivo más grande)
├── models.rs      ← el vocabulario común; no importa a nadie
├── config.rs      ← configuración y valores por defecto
├── i18n.rs        ← traducción ES/EN
├── meta.rs        ← constantes del producto
└── services/
    ├── inspector.rs   ← orquesta la captura   ← empieza aquí
    ├── macos.rs       ← ÚNICO punto que habla con el sistema
    ├── launchd.rs · security.rs · tcc.rs · network.rs · netscan.rs · temp_scan.rs
    │                   ← un recolector por superficie
    ├── rules.rs · anomaly.rs · baseline.rs
    │                   ← deciden qué significan los datos
    ├── persistence.rs ← SQLite
    ├── resilience.rs  ← salud del propio agente
    ├── report.rs      ← reporte Markdown
    └── ai.rs          ← adaptador opcional
```

Regla de oro para orientarte: **si toca el sistema operativo, va en `macos.rs`; si decide, va
en `rules.rs` o `anomaly.rs`; si solo transforma datos, va en el recolector de su
superficie.**

## 6. Dónde añadir cosas

### 6.1 Una comprobación nueva en una superficie existente

Ejemplo: añadir un control de seguridad.

1. Escribe una función privada en `src/services/security.rs` que ejecute el comando con
   `macos::run_combined` y devuelva un `SecurityControl`.
2. Añádela al `vec![…]` de `scan_controls`.
3. Rellena `evidence` con la primera línea útil de la salida y `explanation` con qué significa
   y qué hacer si está apagado.
4. Usa `severity_for(enabled, known, cuando_apagado)`: **no calcules la severidad a mano**,
   porque esa función codifica la regla de que «desconocido» es amarillo.
5. Añade un test con una salida literal del comando.

No hace falta tocar nada más: la baseline, las alertas y la interfaz lo recogen solas, porque
`control_watch_items` recorre la lista completa.

### 6.2 Una heurística nueva

1. Añade sus parámetros a `AnomalyConfig` en `src/config.rs`, con su función `default_*()`.
2. Impleméntala dentro del bucle de `AnomalyTracker::analyze`, usando `process_event` para
   construir el evento.
3. Elige un `kind` en minúsculas con guiones y no reutilices uno existente.
4. Decide si necesita racha (estado en `ProcessHistory`) o si es inmediata.
5. Añade al menos dos tests: uno que dispare y **uno que compruebe que no dispara con
   software normal**. El segundo importa más que el primero.

### 6.3 Una superficie vigilada nueva

Es el cambio más grande y el que mejor muestra la arquitectura:

1. Modelo en `src/models.rs` con `Serialize`/`Deserialize` y `#[serde(default)]` en los
   campos nuevos.
2. Recolector en `src/services/<superficie>.rs` con una función `scan()` y una
   `*_watch_items()`.
3. Si necesita el sistema, añade la llamada a `src/services/macos.rs`, nunca directamente.
4. `SurfaceSpec` en `src/services/baseline.rs` con sus textos y su riesgo.
5. Campo en `SystemSnapshot` y llamada en `collect_snapshot`, junto con una función
   `detect_<superficie>_changes` siguiendo el patrón de las cuatro existentes.
6. Interruptor `watch_<superficie>` en `AnomalyConfig`.
7. Sección en `src/app.rs` y comando en `src/cli.rs`.
8. Tests del parser y de la clasificación.

### 6.4 Un comando nuevo del CLI

1. Añade la rama al `match` de `cli::run`.
2. Escribe `cmd_<nombre>` siguiendo el patrón: crear servicio, obtener datos, `--json` si se
   pide, salida tabulada si no.
3. Documenta el comando en `print_help`, en [`docs/COMMANDS.md`](../COMMANDS.md) y en
   [05 · Referencia técnica](05-technical-reference.md).
4. Usa `truncate` para las columnas: hay un test que protege el corte con acentos.

## 7. Cómo escribir pruebas aquí

- Van **al final del archivo**, en un módulo `#[cfg(test)] mod tests`.
- Nombres en español, descriptivos y en forma de afirmación:
  `un_pico_de_cpu_no_dispara_nada`, no `test_cpu`.
- Los datos de muestra son constantes literales dentro del test; no hay fixtures.
- Prueba **el criterio**, no la implementación: si mañana cambia la forma de calcular el
  puntaje pero un compilador legítimo sigue sin pintarse de rojo, el test debe seguir en
  verde.

## 8. Convenciones que hay que respetar

| Ámbito | Convención |
|---|---|
| Formato | `cargo fmt`; ancho 100, saltos Unix. La CI lo verifica |
| Lints | `clippy -D warnings`. Nada de `#[allow]` sin comentario que lo justifique |
| Idioma del código | Identificadores en inglés; comentarios, mensajes y documentación en español |
| Comentarios | Explican **por qué**, no qué. Si el comentario repite la línea, sobra |
| Documentación de módulo | Todo archivo empieza con `//!` explicando su papel |
| Documentación pública | Todo elemento `pub` lleva `///`. Actualmente 232 de 232 |
| Errores | `anyhow::Result` con `.context(...)` en las rutas que ve el usuario |
| Rutas del sistema | Siempre absolutas (`/usr/sbin/lsof`), nunca por `PATH` |
| Markdown | Líneas de hasta 100 caracteres; `markdownlint-cli2` lo verifica |
| Commits | Mensaje en español, imperativo, explicando el porqué |

## 9. Partes que requieren especial cuidado

| Zona | Por qué | Qué hacer antes de tocarla |
|---|---|---|
| `baseline.rs` y `persistence.rs` | Un error aquí silencia detecciones sin que nadie lo note | Entender la propiedad «pegajosa» y la siembra silenciosa |
| `persistence_entry_key` | Cambiar la clave invalida todas las baselines existentes de los usuarios | Considerar la migración |
| `can_terminate_process` y la lista de protegidos | Aflojarla permite matar procesos que dejan el equipo inutilizable | No la hagas configurable |
| `clean_user_caches` | Es el único código que borra archivos | Mantener las tres salvaguardas y el `dry_run` por defecto |
| `ai.rs` | Es la única salida de red | No añadas datos al payload sin actualizar la política de privacidad y su test |
| `classify_*` | Los puntajes son el producto | Cambiar un peso cambia el comportamiento para todos los usuarios; hazlo con un test que lo justifique |
| `macos.rs` | Un cambio de formato en macOS rompe el parser | Preferir «desconocido» a adivinar |
| Esquema SQLite | No hay migraciones | Añadir una columna a una tabla existente **no funcionará** en instalaciones ya creadas |

## 10. Errores que probablemente cometerás

Todos evitables, todos vistos en el código que ya está:

1. **Llamar a `Command` fuera de `macos.rs`.** Rompe la capa de adaptador y hace el módulo
   imposible de probar.
2. **Calcular una severidad a mano** en vez de usar el helper de su superficie.
3. **Cortar cadenas por bytes.** Los textos están en español; usa `truncate`, que corta por
   caracteres.
4. **Añadir un `unwrap()` sobre una salida del sistema.** Todo lo que viene de fuera puede
   faltar o cambiar de forma.
5. **Actualizar la baseline dentro del diff.** Solo se actualiza al sembrar y al aceptar
   explícitamente.
6. **Añadir una dependencia.** Es el proyecto de un producto que presume de tener diez;
   justifica muy bien cualquier crate nuevo.
7. **Olvidar que hay dos ediciones.** Si tocas algo compartido, comprueba también
   `cargo build --no-default-features`.

## 11. Primeras tareas sugeridas

Ordenadas de menor a mayor dificultad. Las tres primeras se pueden hacer el primer día.

| # | Tarea | Qué aprendes | Dificultad |
|---|---|---|---|
| 1 | Añadir tres prefijos OUI nuevos a `netscan::OUI_TABLE` con su test | Estructura de un recolector y de sus tests | Muy baja |
| 2 | Añadir un servicio TCC más a `service_label` | Cómo se traduce el vocabulario del sistema | Muy baja |
| 3 | Extraer `expand_home` y `bytes_to_mb` a un módulo común (R-14) | Organización del proyecto | Baja |
| 4 | Sustituir `static mut ACTIVE_DARK` por `AtomicBool` (R-13) | El modelo de estado de la interfaz | Baja |
| 5 | Implementar `notification_cooldown_secs` (R-05) | Estado entre capturas en `InspectorService` | Media |
| 6 | Validar que los umbrales críticos superen a los de aviso (R-11) | Carga de configuración y advertencias | Media |
| 7 | Escribir los tests de `PersistenceStore` sobre una base temporal (R-18) | Toda la capa de persistencia | Media |
| 8 | Añadir `PRAGMA user_version` y migraciones (R-06) | Evolución del esquema | Media-alta |
| 9 | Implementar el deshacer de baseline (R-01) | El núcleo del producto | Alta |

Las referencias `R-xx` remiten a [15 · Riesgos y deuda técnica](15-risks-and-technical-debt.md),
donde cada una está descrita con su recomendación.

## 12. Itinerario de una semana

| Día | Objetivo | Entregable |
|---|---|---|
| 1 | Entorno listo y producto ejecutándose; leer README, 01, 03 y 04 | `ci-local.sh` en verde en tu equipo |
| 2 | Seguir una captura de punta a punta con el documento 06 al lado | Poder explicar qué hace `collect_snapshot` |
| 3 | Entender baseline y persistencia (documentos 07 y 08) | Poder explicar por qué un cambio es «pegajoso» |
| 4 | Primera tarea (1, 2 o 3 de la lista) con su test | Pull request pequeño y en verde |
| 5 | Leer 10, 11 y 12; ejecutar el generador de PDF | Segunda tarea, de dificultad media |

## 13. A quién preguntar y dónde mirar

| Duda | Fuente |
|---|---|
| «¿Por qué está hecho así?» | El comentario junto al código; casi siempre está respondido ahí |
| «¿Qué hace este comando?» | [`docs/COMMANDS.md`](../COMMANDS.md) y [05](05-technical-reference.md) |
| «¿Qué significa este término?» | [16 · Glosario](16-glossary.md) |
| «¿Esto es un fallo conocido?» | [15 · Riesgos](15-risks-and-technical-debt.md) |
| «¿Cómo se prueba esto?» | [12 · Pruebas](12-testing-and-quality.md) |
| «¿Dónde vive este dato?» | [07 · Base de datos](07-database.md) y [19 · Trazabilidad](19-traceability-matrix.md) |
| «¿Por qué falla mi entorno?» | [14 · Solución de problemas](14-troubleshooting.md) |

---

**Siguiente lectura recomendada:** [19 · Matriz de trazabilidad](19-traceability-matrix.md).
