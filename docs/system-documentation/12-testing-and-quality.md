# 12 · Pruebas y calidad

> Qué se prueba, cómo se ejecuta, qué queda sin cubrir y qué pruebas faltan, ordenadas por
> prioridad. Los 112 tests de este documento se ejecutaron en este análisis.

---

## 1. Resumen

| Indicador | Valor |
|---|---|
| Tests unitarios | **112** |
| Tests de integración (`tests/`) | **0** — no existe el directorio |
| Tests de interfaz | **0** |
| Módulos con tests | 20 de 23 archivos `.rs` |
| Módulos sin tests | `main.rs`, `meta.rs`, `services/mod.rs` |
| Resultado de la última ejecución | `112 passed; 0 failed; 0 ignored`, 1,47 s |
| Análisis estático | `cargo clippy --all-targets --all-features -- -D warnings`, sin advertencias |
| Formato | `cargo fmt --all -- --check`, sin diferencias |
| Lint de documentación | `markdownlint-cli2` en la CI |
| Cobertura medida | **No se mide**: no hay `tarpaulin`, `llvm-cov` ni informe de cobertura |

## 2. Cómo se ejecutan

```bash
cargo test --all-features                    # los 112
cargo test --all-features -- --nocapture     # con stdout, igual que la CI
cargo test services::rules                   # solo un módulo
cargo test la_huella_agrupa                  # un test por nombre parcial
./scripts/ci-local.sh                        # formato + clippy + tests + dos builds + humo
```

Los tests están **junto al código**, en módulos `#[cfg(test)]` al final de cada archivo. Es la
convención idiomática de Rust y tiene una ventaja concreta aquí: permiten probar funciones
privadas (`severity_for`, `age_severity`, `first_line`, `expand_home`, `ip_sort_key`) sin
hacerlas públicas solo para el test.

## 3. Inventario por módulo

| Módulo | Tests | Qué fijan |
|---|---:|---|
| `services/anomaly.rs` | 14 | Las ocho heurísticas y sus guardas contra falsos positivos |
| `services/rules.rs` | 11 | Clasificación de procesos, orden de alertas, derivación de incidentes |
| `cli.rs` | 8 | Parseo de banderas, códigos de salida, recorte multibyte |
| `services/netscan.rs` | 8 | Parseo ARP, normalización de MAC, caso crítico de la puerta de enlace |
| `services/macos.rs` | 7 | Clasificación de `codesign` y declaración del entorno |
| `services/network.rs` | 6 | Parseo de `lsof`, IP públicas y privadas, agrupación por PID |
| `services/tcc.rs` | 6 | Ambos esquemas de TCC, severidad y filtro de baseline |
| `services/ai.rs` | 6 | Guardas de la IA, parseo de respuesta, contenido del payload |
| `app.rs` | 5 | Buffer de tendencia, porcentajes, secciones únicas |
| `models.rs` | 5 | Orden de severidades y pesos de riesgo |
| `services/launchd.rs` | 5 | Clasificación de entradas de persistencia |
| `services/security.rs` | 5 | Regla «desconocido no es verde», antigüedad de firmas |
| `services/temp_scan.rs` | 5 | Umbrales de tamaño y simulacro de limpieza |
| `services/baseline.rs` | 4 | Eventos por tipo de cambio y su evidencia |
| `services/report.rs` | 4 | Secciones del reporte, escapado y declaración de privacidad |
| `config.rs` | 3 | Coherencia de los valores por defecto y viaje a JSON |
| `services/inspector.rs` | 3 | Conversiones y efecto de la salud del agente |
| `services/resilience.rs` | 3 | Huella de configuración y serialización del estado |
| `i18n.rs` | 2 | Traducción según idioma activo |
| `services/persistence.rs` | 2 | Estabilidad de la clave de baseline |

## 4. Qué protege cada grupo de tests

### 4.1 Los que fijan decisiones de producto

Estos no comprueban código: **comprueban criterios**. Si alguien los rompe, ha cambiado el
producto, no solo la implementación.

| Test | Criterio que fija |
|---|---|
| `una_cpu_alta_por_si_sola_no_es_critica` | Compilar no debe pintarse de rojo |
| `un_pico_de_cpu_no_dispara_nada` | Sostenido, no instantáneo |
| `control_desconocido_no_se_pinta_de_verde` | «No lo sé» nunca es verde |
| `permiso_denegado_nunca_es_advertencia` | Un permiso denegado no es un hallazgo |
| `salida_desconocida_no_asume_confianza` | Lo que no se entiende no se da por bueno |
| `el_payload_solo_lleva_el_incidente_resumido` | La IA no recibe la captura completa |
| `la_ia_desactivada_falla_sin_tocar_la_red` | Apagado significa apagado |
| `el_reporte_declara_que_nada_sale_del_equipo` | La promesa de privacidad está en la salida |
| `el_simulacro_nunca_marca_borrado_real` | `dry_run` no borra |
| `un_navegador_instalado_con_normalidad_no_dispara_trafico_inusual` | Sin falsos positivos con software normal |
| `un_binario_sin_firmar_en_applications_si_dispara_trafico_inusual` | …pero la señal se mantiene donde importa |
| `gateway_nuevo_es_incidente_critico` | Cambio de MAC del router = crítico |
| `persistencia_nueva_y_sospechosa_escala_a_critica` | Dos señales independientes escalan |
| `la_clave_ignora_el_comando` | Cambiar el comando es modificación, no reemplazo |
| `la_huella_agrupa_incidentes_equivalentes` | No se persiste el mismo hallazgo cada segundo |

### 4.2 Los que protegen de errores concretos

| Test | Error que evita |
|---|---|
| `recorta_respetando_caracteres_multibyte` (×2) | Pánico al cortar cadenas con acentos |
| `una_bandera_sin_valor_no_entra_en_panico` | Pánico con una bandera al final de la línea |
| `normaliza_octetos_sin_cero_a_la_izquierda` | Falsos cambios de MAC en la baseline |
| `parsea_nombres_de_proceso_con_espacios` | Parseo roto con `Google Chrome` |
| `ordena_ips_numericamente` | `.10` antes que `.2` |
| `json_vacio_cae_a_defaults_sin_romper` | Configuración parcial que rompe el arranque |
| `un_estado_vacio_no_rompe_la_deserializacion` | Estado de agente vacío |
| `medir_directorio_inexistente_devuelve_cero` | Error al medir una ruta que no existe |
| `epoch_cero_no_produce_fecha` | Fecha inventada a partir de un cero |
| `las_barras_no_rompen_las_tablas` | Tabla Markdown rota por un `\|` |
| `varias_instancias_del_mismo_nombre_no_son_una_reaparicion` | Falso positivo con procesos multi-instancia |
| `el_buffer_de_tendencia_no_crece_sin_limite` | Fuga de memoria en las series |
| `el_porcentaje_tolera_total_cero` | División por cero |

### 4.3 El único test que toca el sistema real

`services::macos::tests::el_entorno_real_encuentra_las_utilidades_base_de_macos` comprueba
que `launchctl` existe en el equipo donde corre. Es el único test no determinista del
conjunto y **fallaría fuera de macOS**, lo que es coherente: el producto solo compila para
macOS en la práctica.

Todos los demás son puros: usan muestras literales (una salida de `lsof`, una tabla ARP, una
respuesta de `codesign`) y estructuras construidas a mano.

## 5. Fixtures y datos de prueba

No hay directorio de fixtures ni archivos de datos. Las muestras viven **como constantes en
el propio test**:

| Constante | Módulo | Qué reproduce |
|---|---|---|
| `SAMPLE` | `network.rs` | Salida de `lsof -F` con tres sockets, uno con nombre de proceso con espacios |
| `ARP_SAMPLE` | `netscan.rs` | Cuatro líneas de `arp -a -n`, dos válidas y dos de ruido |
| Cadenas de `codesign` | `macos.rs` | Cinco variantes de salida real |
| `base_entry()` | `launchd.rs` | Entrada de persistencia típica de una aplicación |
| `snapshot_with_anomaly()` | `rules.rs` | Captura con una anomalía de severidad dada |
| `snapshot()` | `report.rs` | Captura con alerta que contiene una barra vertical |

Es una decisión coherente con el tamaño del proyecto: menos archivos que mantener, y la
muestra se lee junto al test que la usa.

## 6. Módulos y funciones sin cobertura

### 6.1 Sin ningún test

| Elemento | Motivo | Riesgo |
|---|---|---|
| `main.rs` | Punto de entrada; probarlo exige lanzar el proceso | Bajo: 20 líneas de despacho |
| `meta.rs` | Solo constantes | Nulo |
| `services/mod.rs` | Solo declaraciones | Nulo |

### 6.2 Cubiertas parcialmente

| Función | Qué se prueba | Qué **no** |
|---|---|---|
| `InspectorService::collect_snapshot` | Nada directamente | **Toda la secuencia de captura**: es el hueco más grande |
| `PersistenceStore` | Solo `persistence_entry_key` | Ninguna operación SQL real |
| `baseline::diff_surface` | Solo `surface_change_event` | El diff contra una base real |
| `launchd::scan_persistence` | Solo `classify_entry` | Lectura y parseo de plists reales |
| `security::scan_controls` | Solo helpers | La consulta a los seis controles |
| `tcc::scan` | Solo helpers | La lectura de una base TCC |
| `temp_scan::clean_user_caches` | Solo el simulacro | El borrado real |
| `report::save_report` | Solo `build_report` | La escritura en disco |
| `ai::post_json` | Nada | La petición HTTP (razonable: es red externa) |
| Todo `app.rs` salvo helpers | Cinco helpers | El dibujado y el hilo de trabajo |

**El patrón es claro y consistente:** está probado todo lo que es función pura, y sin probar
todo lo que toca el sistema de archivos, la base de datos o la red. Es una elección
defendible —esas partes necesitarían un entorno controlado— pero deja fuera precisamente la
orquestación, que es donde se juntan las piezas.

## 7. Análisis estático y estilo

| Herramienta | Configuración | Estado |
|---|---|---|
| `rustfmt` | `rustfmt.toml`: edición 2021, ancho 100, saltos Unix | En verde |
| `clippy` | `--all-targets --all-features -- -D warnings` | En verde, sin excepciones |
| `markdownlint-cli2` | `.markdownlint-cli2.jsonc` | En verde en CI |
| `cargo audit` | **No configurado** | — |
| Cobertura | **No configurada** | — |

**Cero `#[allow(...)]` de conveniencia**, con dos excepciones justificadas en el código:

- `#![cfg_attr(not(feature = "gui"), allow(dead_code))]` en `main.rs`, con comentario que
  explica que el código lo usa la otra edición.
- `#[allow(clippy::too_many_arguments)]` en `anomaly::process_event`, un constructor común
  de eventos con nueve parámetros.

Que `clippy` pase con `-D warnings` sin silenciar lints es un indicador de calidad
significativo en un proyecto de 13 000 líneas.

## 8. Integración continua

`.github/workflows/ci.yml`, dos jobs:

| Job | Runner | Pasos |
|---|---|---|
| `docs` | `ubuntu-latest` | `npx --yes markdownlint-cli2` |
| `validate` | `macos-latest` | Toolchain + caché → versiones → `fmt --check` → `clippy -D warnings` → `test --all-features -- --nocapture` → build CLI-only → build completa → humo de la CLI → artefacto |

**El humo de la CLI** ejecuta cuatro comandos reales en el runner:

```bash
./target/release/rootcause --version
./target/release/rootcause --help > /dev/null
./target/release/rootcause security
./target/release/rootcause persistence
```

Es más valioso de lo que parece: comprueba que el binario arranca, que el motor se
inicializa, que se crea la carpeta de datos, que el esquema SQLite se aplica y que dos
superficies reales responden en un macOS limpio. Es, de hecho, **la única prueba de
integración del proyecto**, aunque no esté escrita como test.

## 9. Criterios de aceptación observables

Deducidos de la CI y de los scripts, no declarados como tales en el repositorio:

1. El código está formateado según `rustfmt.toml`.
2. `clippy` no emite ni una advertencia con todas las features.
3. Los 112 tests pasan.
4. Las dos ediciones compilan en release.
5. El binario responde a `--version`, `--help`, `security` y `persistence`.
6. Todos los Markdown pasan `markdownlint-cli2`.

`scripts/ci-local.sh` reproduce los cinco primeros y termina con «Todo en verde. Listo para
empujar».

## 10. Casos límite relevantes ya cubiertos

| Caso | Test |
|---|---|
| Cadena con acentos en una tabla | `recorta_respetando_caracteres_multibyte` |
| Bandera sin valor | `una_bandera_sin_valor_no_entra_en_panico` |
| JSON de configuración vacío | `json_vacio_cae_a_defaults_sin_romper` |
| MAC con octetos cortos | `normaliza_octetos_sin_cero_a_la_izquierda` |
| Entradas ARP incompletas y de difusión | `parsea_vecinos_y_descarta_ruido` |
| Proceso con nombre compuesto | `parsea_nombres_de_proceso_con_espacios` |
| CGNAT y link-local | `ips_privadas_y_loopback_no_son_publicas` |
| IPv6 entre corchetes | `extrae_ip_de_extremos_v4_y_v6` |
| Directorio inexistente | `medir_directorio_inexistente_devuelve_cero` |
| Epoch cero en TCC | `epoch_cero_no_produce_fecha` |
| Esquema TCC heredado | `decodifica_el_esquema_heredado` |
| Salida de `codesign` no reconocida | `salida_desconocida_no_asume_confianza` |
| Respuesta de IA sin `choices` | `una_respuesta_sin_choices_es_error_claro` |
| Captura sin hallazgos | `una_captura_sana_no_genera_incidente` |

## 11. Casos límite **no** cubiertos

| Caso | Qué podría pasar | Prioridad |
|---|---|---|
| Umbrales invertidos (`critical < warning`) | La rama de aviso queda inalcanzable | Media |
| `refresh_interval_secs = 0` | La GUI fuerza 2 s; el CLI no lo usa | Baja |
| Plist con `KeepAlive` como diccionario | Cubierto en el código, sin test | Media |
| Plist con `ProgramArguments` vacío | Comando vacío en la baseline | Media |
| Baseline con miles de entradas | Rendimiento del diff | Baja |
| SQLite bloqueado por otro proceso | Error de escritura → alerta | Media |
| Directorio de datos sin permiso de escritura | El servicio no arranca | Media |
| `TCC.db` con esquema desconocido | `PRAGMA` no encuentra columnas esperadas | Media |
| Reloj del sistema hacia atrás | Antigüedad negativa (ya se satura con `max(0)`) | Baja |
| Proceso que muere entre `ps` y `sysinfo` | Datos parciales de ese PID | Baja |

## 12. Pruebas faltantes, priorizadas

### Prioridad alta

1. **`PersistenceStore` sobre una base temporal.** Crear el store en un directorio temporal
   y probar: creación de esquema, inserción y recorte (`trim_table`), deduplicación por
   huella, y el ciclo completo `replace_baseline` → `load_baseline` → `diff_surface`. Es el
   componente con más lógica sin cobertura y el que sostiene toda la detección de cambios.
2. **`baseline::diff_surface` de punta a punta.** Con el store real: primera siembra
   silenciosa, detección de `Added`/`Modified`/`Removed`, y que **los cambios sigan
   reportándose** en la captura siguiente (la propiedad «pegajosa» no tiene ningún test).
3. **`launchd::parse_launch_plist` con plists sintéticos.** Escribir plists en un directorio
   temporal y comprobar: `Program` y `ProgramArguments` combinados, `KeepAlive` como
   diccionario, `StartInterval` inválido, plist corrupto y plist sin `Label`.

### Prioridad media

1. **`temp_scan::clean_user_caches` real**, sobre un árbol temporal: que respete el corte de
   24 horas, que no salga de la raíz y que cuente correctamente lo saltado.
2. **`tcc::read_database` con bases sintéticas** de ambos esquemas, creadas con `rusqlite`.
3. **Validación de configuración**: test que compruebe que umbrales invertidos se detectan (y,
   antes, decidir si deben corregirse o solo advertirse).
4. **`report::save_report`** en un directorio temporal.

### Prioridad baja

1. Test de humo del ciclo completo `collect_snapshot` con un `InspectorService` apuntando a
   un directorio de datos temporal. Requiere parametrizar `meta::APP_DIR`, que hoy es una
   constante: es un cambio de diseño, no solo un test.
2. Pruebas de la interfaz con el arnés de pruebas de egui.
3. Comprobación de rendimiento del diff con baselines grandes.

### Lo que no merece la pena probar

- El dibujado de widgets: coste alto, valor bajo, cambia con cada ajuste visual.
- La petición HTTP real a la IA: depende de un tercero; lo que importa —las guardas y el
  contenido del payload— ya está cubierto.

## 13. Mejoras de proceso recomendadas

| Mejora | Beneficio | Coste |
|---|---|---|
| Añadir `cargo audit` a la CI | Vigila vulnerabilidades de las 326 dependencias | Bajo |
| Medir cobertura con `cargo llvm-cov` | Da un número objetivo y detecta huecos nuevos | Bajo |
| Fijar la versión de `markdownlint-cli2` | Evita que una versión nueva rompa la CI sin cambios en el repositorio | Muy bajo |
| Añadir un job que compile en una segunda versión de Rust | Detecta lints nuevos antes de que rompan la CI | Bajo |
| Directorio `tests/` con las pruebas de alta prioridad | Cubre el hueco de persistencia y baseline | Medio |

La cuarta merece un comentario: el historial del repositorio incluye un commit titulado
*«Corregir dos lints que solo aparecen en rustc 1.97 (el del runner)»*, es decir, **ya ha
ocurrido** que la CI detectara antes que el equipo local un lint nuevo.

## 14. Resultados de la ejecución de este análisis

Equipo: macOS 26.3.1 (25D2128), Apple Silicon.

| Comando | Resultado |
|---|---|
| `cargo fmt --all -- --check` | Código de salida `0`, sin diferencias |
| `cargo clippy --all-targets --all-features -- -D warnings` | Código de salida `0`, sin advertencias |
| `cargo test --all-features` | `112 passed; 0 failed; 0 ignored; 0 measured`, 1,47 s |
| `cargo build --no-default-features` | Código de salida `0` |
| `./target/debug/rootcause security` | Seis controles con evidencia real |
| `./target/debug/rootcause tcc` | Declara la falta de Acceso total al disco, como está diseñado |
| `./target/debug/rootcause config show --json` | Configuración por defecto idéntica a la documentada |

Estos cuatro últimos comandos son, en la práctica, la misma prueba de humo que ejecuta la CI.

---

**Siguiente lectura recomendada:** [13 · Despliegue y operación](13-deployment-and-operations.md).
