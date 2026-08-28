# 15 · Riesgos y deuda técnica

> Registro de hallazgos de este análisis, clasificados por severidad, impacto, probabilidad y
> evidencia. **Documento informativo: ningún hallazgo se corrigió automáticamente.** El único
> cambio en el código de este análisis fue añadir 71 líneas de comentarios de documentación.

---

## 1. Cómo se clasifica

| Campo | Valores |
|---|---|
| **Severidad** | Crítica · Alta · Media · Baja · Informativa |
| **Impacto** | Qué pasa si se materializa |
| **Probabilidad** | Alta · Media · Baja |
| **Prioridad** | P1 (antes de la siguiente release) · P2 (corto plazo) · P3 (cuando toque) · P4 (opcional) |

La severidad refleja el efecto sobre la **función del producto** —detectar y explicar— no
solo sobre el código.

## 2. Resumen

| Severidad | Hallazgos | Prioridad más alta |
|---|---:|---|
| Crítica | 0 | — |
| Alta | 3 | P1 |
| Media | 9 | P2 |
| Baja | 8 | P3 |
| Informativa | 4 | P4 |
| **Total** | **24** | — |

No se encontró ningún fallo que comprometa la corrección de la detección con la configuración
por defecto. Los tres hallazgos altos son un límite estructural del enfoque, una acción
irreversible sin red de seguridad y una exposición transitoria de credencial.

---

## 3. Severidad alta

### R-01 · Aceptar una baseline es irreversible

| Campo | Valor |
|---|---|
| **Severidad** | Alta |
| **Impacto** | Un hallazgo real aceptado por error deja de reportarse para siempre. La única salida es borrar el SQLite, lo que destruye también el historial y la auditoría |
| **Probabilidad** | Media: la aceptación está a un clic y a un `--accept` |
| **Evidencia** | `replace_persistence_baseline` y `replace_baseline` hacen `DELETE` + `INSERT` sin conservar la versión anterior; no existe ninguna función de deshacer |
| **Ubicación** | `src/services/persistence.rs`, `src/services/inspector.rs::accept_*_baseline` |
| **Recomendación** | Versionar la baseline: una tabla `baseline_history` con la foto anterior y un comando `rootcause baseline --undo`. Alternativa mínima: volcar la baseline saliente a un JSON en la carpeta de datos antes de reemplazarla |
| **Prioridad** | **P1** |

### R-02 · La primera baseline se siembra en silencio

| Campo | Valor |
|---|---|
| **Severidad** | Alta |
| **Impacto** | Si el equipo ya está comprometido al instalar RootCause, la persistencia del atacante queda registrada como «estado bueno conocido» y nunca se reporta como cambio |
| **Probabilidad** | Baja en un equipo sano; **alta en el escenario en que más se necesitaría la herramienta** |
| **Evidencia** | `baseline::diff_surface`: si la baseline está vacía, la siembra y devuelve `false` sin generar eventos. Está documentado como decisión deliberada |
| **Ubicación** | `src/services/baseline.rs`, `src/services/inspector.rs::diff_persistence_baseline` |
| **Recomendación** | Mitigación parcial, no eliminación: en la primera captura, listar en un aviso destacado las entradas que **por sí solas** tienen riesgo alto o crítico, aunque no sean «cambios». Los datos ya existen (`classify_entry` los puntúa); solo falta presentarlos como «revisión inicial obligatoria» |
| **Prioridad** | **P1** |
| **Nota** | El propio `README.md` reconoce el enfoque; lo que falta es la revisión inicial, no un cambio de diseño |

### R-03 · La clave de la IA es visible en la lista de procesos

| Campo | Valor |
|---|---|
| **Severidad** | Alta (condicionada: solo si el usuario activa la IA) |
| **Impacto** | Cualquier usuario del equipo puede leer la clave con `ps` mientras dura la petición |
| **Probabilidad** | Baja: requiere IA activada y un observador local en la ventana de la petición |
| **Evidencia** | `ai::post_json` pasa `-H "Authorization: Bearer <clave>"` como argumento de `curl`. El cuerpo sí se protege por `stdin`, con comentario que explica esa protección |
| **Ubicación** | `src/services/ai.rs::post_json` |
| **Recomendación** | Pasar las cabeceras también por `stdin` (`--header @-` con `-K -` o un archivo de configuración temporal con permisos 600) |
| **Prioridad** | **P1** |

---

## 4. Severidad media

### R-04 · Cinco campos de configuración no tienen ningún efecto

| Campo | Valor |
|---|---|
| **Severidad** | Media |
| **Impacto** | El usuario configura algo que no ocurre. Es peor que no ofrecerlo: genera una falsa sensación de control |
| **Probabilidad** | Alta: aparecen en el JSON que genera `config init` |
| **Evidencia** | `grep` fuera de `src/config.rs` no devuelve ninguna lectura de `anomaly.suspicious_parent_names`, `anomaly.shell_interpreters`, `ui.daily_report`, `alerting.notification_cooldown_secs` ni `resilience.stale_after_secs` |
| **Ubicación** | `src/config.rs` |
| **Recomendación** | Implementarlos o retirarlos. Los dos primeros corresponden a heurísticas de linaje de proceso que el módulo de anomalías aún no cubre; `notification_cooldown_secs` es el más barato de implementar y el que más ruido evita |
| **Prioridad** | **P2** |

### R-05 · Las notificaciones críticas pueden repetirse en cada captura

| Campo | Valor |
|---|---|
| **Severidad** | Media |
| **Impacto** | Con una condición crítica persistente y refresco de 5 s, el centro de notificaciones se llena. Un usuario saturado deja de leer las alertas |
| **Probabilidad** | Alta cuando hay un hallazgo crítico real |
| **Evidencia** | `notify_if_critical` no consulta ningún estado previo; `notification_cooldown_secs` existe y no se lee |
| **Ubicación** | `src/services/inspector.rs::notify_if_critical` |
| **Recomendación** | Guardar la marca de tiempo y la huella de la última notificación y respetar el cooldown configurado |
| **Prioridad** | **P2** |

### R-06 · No hay migraciones de esquema

| Campo | Valor |
|---|---|
| **Severidad** | Media |
| **Impacto** | Añadir una columna a una tabla existente no se aplicará en instalaciones ya creadas: `CREATE TABLE IF NOT EXISTS` no altera nada. El síntoma sería un error de columna inexistente en tiempo de ejecución |
| **Probabilidad** | Media: ocurrirá en cuanto el esquema evolucione |
| **Evidencia** | `ensure_schema` solo contiene `CREATE TABLE IF NOT EXISTS` y un `CREATE INDEX IF NOT EXISTS` |
| **Ubicación** | `src/services/persistence.rs::ensure_schema` |
| **Recomendación** | Añadir `PRAGMA user_version` y una función `migrate()` con pasos numerados. Es barato ahora y caro después |
| **Prioridad** | **P2** |

### R-07 · `audit_log` crece sin límite

| Campo | Valor |
|---|---|
| **Severidad** | Media |
| **Impacto** | El archivo SQLite crece indefinidamente en instalaciones de larga duración |
| **Probabilidad** | Media |
| **Evidencia** | `trim_table` solo se llama con `"snapshots"` e `"incidents"` |
| **Ubicación** | `src/services/persistence.rs` |
| **Recomendación** | Decidir explícitamente: o un límite alto configurable, o una rotación a archivo externo. **No recortarla en silencio**: es evidencia |
| **Prioridad** | **P2** |

### R-08 · La auditoría es alterable por quien pueda escribir en el archivo

| Campo | Valor |
|---|---|
| **Severidad** | Media |
| **Impacto** | Un atacante con acceso al usuario puede borrar el rastro de sus acciones y de las baselines aceptadas |
| **Probabilidad** | Baja, pero es exactamente el escenario que la herramienta pretende cubrir |
| **Evidencia** | La tabla vive en el mismo SQLite del usuario, sin firma ni cadena de hashes |
| **Ubicación** | `src/services/persistence.rs` |
| **Recomendación** | Cadena de hashes encadenados por fila (cada registro incluye el hash del anterior). No impide el borrado, pero lo hace detectable |
| **Prioridad** | **P2** |

### R-09 · La huella de configuración no es criptográfica

| Campo | Valor |
|---|---|
| **Severidad** | Media |
| **Impacto** | Un cambio malicioso que preserve tamaño y fecha (`touch`) pasa inadvertido |
| **Probabilidad** | Baja |
| **Evidencia** | `config_fingerprint` devuelve `format!("{}-{}", len, modified)`. **El código lo declara abiertamente** y explica por qué no promete más |
| **Ubicación** | `src/services/resilience.rs::config_fingerprint` |
| **Recomendación** | SHA-256 del contenido. Requiere una dependencia nueva o una implementación propia; valorar si compensa frente a la política de cero dependencias |
| **Prioridad** | **P2** |

### R-10 · No se comprueba que el endpoint de IA sea HTTPS

| Campo | Valor |
|---|---|
| **Severidad** | Media |
| **Impacto** | Un `ai.endpoint` con `http://` enviaría el incidente y la clave en claro |
| **Probabilidad** | Baja: exige que el usuario lo configure así |
| **Evidencia** | La única validación de `endpoint` es que no esté vacío |
| **Ubicación** | `src/services/ai.rs::summarize_incident` |
| **Recomendación** | Rechazar cualquier esquema distinto de `https://`, salvo `localhost` para pruebas |
| **Prioridad** | **P2** |

### R-11 · Los umbrales no se validan entre sí

| Campo | Valor |
|---|---|
| **Severidad** | Media |
| **Impacto** | Con `cpu_critical_percent < cpu_warning_percent`, la rama de aviso es inalcanzable y el usuario cree tener dos niveles cuando solo tiene uno |
| **Probabilidad** | Baja |
| **Evidencia** | `classify_process` evalúa primero la rama crítica; no hay validación en `load_or_default` |
| **Ubicación** | `src/config.rs`, `src/services/rules.rs::classify_process` |
| **Recomendación** | Validar al cargar y emitir una advertencia como la de JSON inválido, que ya existe y es el mecanismo adecuado |
| **Prioridad** | **P2** |

### R-12 · `app.rs` concentra 2 750 líneas

| Campo | Valor |
|---|---|
| **Severidad** | Media |
| **Impacto** | Dificulta la revisión y las modificaciones; concentra el 21 % del código del proyecto en un archivo |
| **Probabilidad** | Alta (ya ocurre) |
| **Evidencia** | `wc -l src/app.rs` = 2 750; contiene paleta, hilo de trabajo, navegación, doce secciones y veinte widgets |
| **Ubicación** | `src/app.rs` |
| **Recomendación** | División natural sin cambiar comportamiento: `app/worker.rs` (canales y hilo), `app/theme.rs` (paleta y estilo), `app/widgets.rs` (widgets comunes) y `app/views/*.rs` (una por sección) |
| **Prioridad** | **P2** |

---

## 5. Severidad baja

### R-13 · `static mut` para la paleta activa

| Campo | Valor |
|---|---|
| **Severidad** | Baja |
| **Impacto** | Único `unsafe` del proyecto. Correcto mientras solo lo use el hilo de interfaz, pero es una invariante no verificada por el compilador |
| **Evidencia** | `static mut ACTIVE_DARK: bool` en `src/app.rs`, con comentario `SAFETY` |
| **Recomendación** | Sustituir por `AtomicBool` (como ya hace `i18n.rs` con el idioma) o por `thread_local!`. El coste es nulo y elimina el `unsafe` |
| **Prioridad** | **P3** |

### R-14 · `expand_home` y `bytes_to_mb` están duplicadas

| Campo | Valor |
|---|---|
| **Severidad** | Baja |
| **Impacto** | Dos implementaciones idénticas que podrían divergir |
| **Evidencia** | `expand_home` en `launchd.rs` y `temp_scan.rs`; `bytes_to_mb` en `inspector.rs` y `temp_scan.rs` |
| **Recomendación** | Moverlas a un módulo `util` o a `macos.rs` |
| **Prioridad** | **P3** |

### R-15 · Sin `cargo audit` en la CI

| Campo | Valor |
|---|---|
| **Severidad** | Baja |
| **Impacto** | Una vulnerabilidad publicada en cualquiera de las 326 dependencias pasaría inadvertida |
| **Evidencia** | `ci.yml` no incluye el paso; no hay `deny.toml` ni `audit.toml` |
| **Recomendación** | Añadir un paso `cargo audit` y, opcionalmente, `cargo deny` |
| **Prioridad** | **P3** |
| **Nota** | El repositorio hermano `rootcause-server` sí incluye `deny.toml` |

### R-16 · Acciones de CI y `markdownlint-cli2` sin fijar

| Campo | Valor |
|---|---|
| **Severidad** | Baja |
| **Impacto** | `npx --yes markdownlint-cli2` descarga la última versión en cada ejecución: una versión nueva con reglas nuevas puede romper la CI sin que cambie el repositorio |
| **Evidencia** | `.github/workflows/ci.yml` |
| **Recomendación** | Fijar `markdownlint-cli2@<versión>` y, si se quiere ir más lejos, las acciones por SHA |
| **Prioridad** | **P3** |

### R-17 · Sin cobertura de tests medida

| Campo | Valor |
|---|---|
| **Severidad** | Baja |
| **Impacto** | No hay forma objetiva de saber si la cobertura sube o baja con cada cambio |
| **Evidencia** | No hay configuración de `tarpaulin` ni `llvm-cov` |
| **Recomendación** | `cargo llvm-cov` en la CI, sin puerta de calidad al principio |
| **Prioridad** | **P3** |

### R-18 · La orquestación y la persistencia no tienen tests

| Campo | Valor |
|---|---|
| **Severidad** | Baja (por cobertura indirecta del humo de la CI) |
| **Impacto** | `collect_snapshot`, `PersistenceStore` y `diff_surface` sobre base real dependen de la prueba de humo |
| **Evidencia** | Ver [12 · Pruebas §6](12-testing-and-quality.md) |
| **Recomendación** | Las tres pruebas de prioridad alta listadas en ese documento |
| **Prioridad** | **P3** |

### R-19 · Cask de Homebrew sin publicar

| Campo | Valor |
|---|---|
| **Severidad** | Baja |
| **Impacto** | El `README` lo lista como «Plantilla»; un usuario podría intentar `brew install` y no encontrar nada |
| **Evidencia** | `packaging/homebrew/rootcause.rb` existe; no hay tap ni referencia a uno |
| **Recomendación** | Publicar el tap o marcarlo aún más claramente como no disponible |
| **Prioridad** | **P3** |

### R-20 · Saneado parcial del AppleScript de notificación

| Campo | Valor |
|---|---|
| **Severidad** | Baja |
| **Impacto** | Se sustituyen comillas dobles pero no barras invertidas; un texto con `\"` podría alterar el script |
| **Probabilidad** | Muy baja: los textos los genera el propio producto, no una entrada externa |
| **Evidencia** | `macos::notify` |
| **Recomendación** | Escapar también `\` o usar el paso de argumentos de `osascript` en vez de interpolar |
| **Prioridad** | **P3** |

---

## 6. Informativos

### R-21 · Los incidentes no referencian su captura

Sin clave foránea ni columna de referencia: la relación es por marca de tiempo. Si
`snapshots` recorta la fila correspondiente, el incidente queda huérfano. **Impacto real
bajo** porque `payload_json` es autosuficiente. Prioridad P4.

### R-22 · El CLI no valida banderas desconocidas

`rootcause status --inventada` se ejecuta ignorando la bandera. Es un comportamiento
razonable para una herramienta de diagnóstico, pero podría ocultar una errata en un script.
Prioridad P4.

### R-23 · El CLI solo está en español

`i18n::tr` solo lo usa `app.rs`. La interfaz gráfica es bilingüe; la consola, no. Es una
decisión implícita, no declarada. Prioridad P4.

### R-24 · Reportes y exportes sin modo redactado

Contienen nombre de equipo, modelo, rutas de usuario e IP. Quien comparta un reporte debe
revisarlo antes. Un modo `--redact` sería útil para compartir evidencia con terceros.
Prioridad P4.

---

## 7. Lo que se buscó y **no** se encontró

Registrar lo que está bien es tan útil como registrar lo que falla:

| Comprobación | Resultado |
|---|---|
| Credenciales o tokens en el repositorio | Ninguno |
| Ejecución de shell (`sh -c`) | Ninguna |
| Inyección SQL desde datos externos | Ninguna: parámetros ligados en todas las consultas con datos |
| Llamadas a `sudo` | Ninguna |
| `unwrap()` sobre entrada externa en la ruta de captura | Ninguno |
| `#[allow(...)]` sin justificación | Ninguno: los dos existentes están comentados |
| Dependencias no usadas | Ninguna |
| Módulos o funciones muertas | Ninguna |
| Ciclos de dependencia entre módulos | Ninguno |
| `--insecure` o verificación TLS desactivada | Ninguna |
| Telemetría o salida de red no declarada | Ninguna |
| Advertencias de `clippy` | Ninguna |
| Tests fallando o ignorados | Ninguno |

## 8. Decisiones que requieren validación humana

No son fallos: son elecciones que solo el responsable del producto puede confirmar.

| # | Decisión | Pregunta abierta |
|---|---|---|
| 1 | Los cinco campos de configuración sin efecto | ¿Implementarlos o retirarlos? Retirarlos rompe la compatibilidad del JSON |
| 2 | Alcance del aviso inicial (R-02) | ¿Un aviso destacado en la primera captura o un comando `rootcause review-initial`? |
| 3 | Hash criptográfico de configuración (R-09) | ¿Merece una dependencia nueva en un proyecto que presume de tener pocas? |
| 4 | Retención de la auditoría (R-07) | ¿Límite, rotación o crecimiento indefinido asumido? |
| 5 | División de `app.rs` (R-12) | ¿Ahora o cuando se añada la siguiente sección? |
| 6 | Publicación del cask (R-19) | ¿Se mantiene la vía de compilación como única recomendada? |
| 7 | Firma y notarización | Requiere una cuenta de desarrollador de Apple: decisión de coste |

## 9. Elementos no verificados en este análisis

Repetidos aquí desde la portada, con su motivo:

| # | Sin verificar | Motivo |
|---|---|---|
| 1 | Lectura real de `TCC.db` | El terminal del análisis no tiene Acceso total al disco |
| 2 | Comportamiento como root | No se ejecutó con privilegios elevados |
| 3 | Consulta de login items | Dispararía un diálogo de permisos que alteraría el estado del equipo |
| 4 | Barrido activo de red | Generaría tráfico hacia 254 direcciones del segmento |
| 5 | Contrato del proveedor IA | Depende del endpoint que configure cada usuario |
| 6 | Vulnerabilidades de dependencias | `cargo audit` no ejecutado |
| 7 | Publicación real de una release | Requiere `gh` autenticado y crear una etiqueta |
| 8 | Comportamiento en macOS 13 y 14 | Solo se probó en macOS 26.3.1 |

---

**Siguiente lectura recomendada:** [16 · Glosario](16-glossary.md).
