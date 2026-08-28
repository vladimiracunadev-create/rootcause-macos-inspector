# 09 · APIs e integraciones

> Qué interfaces expone el sistema, qué servicios externos consume y bajo qué condiciones.
> RootCause **no expone ningún servidor ni endpoint HTTP propio**: sus «APIs» son la línea
> de comandos, el JSON de salida y el conjunto de utilidades de macOS que consume.

---

## 1. Inventario de interfaces

| Interfaz | Dirección | Protocolo | Autenticación | Estado |
|---|---|---|---|---|
| CLI | Entrada | Argumentos de proceso | La del sistema operativo | Activa |
| JSON de salida | Salida | Archivo / `stdout` | — | Activa |
| Reporte Markdown | Salida | Archivo | — | Activa |
| SQLite propio | Ambas | Archivo local | Permisos del sistema de archivos | Activa |
| Utilidades de macOS | Salida→Entrada | Proceso hijo | La del usuario que ejecuta | Activa |
| `TCC.db` | Entrada | SQLite en solo lectura | Acceso total al disco | Activa, condicionada |
| Proveedor de IA | Salida | HTTPS vía `curl` | Bearer token en cabecera | **Opcional, apagada** |
| Notificaciones de macOS | Salida | `osascript` | — | Activa, condicionada |
| GitHub Actions | — | — | Token del repositorio | Activa |
| GitHub Pages | Salida | HTTPS | — | Activa |
| Homebrew cask | Salida | — | — | **Plantilla** |

**No existe:** servidor HTTP, WebSocket, gRPC, cola de mensajes, webhook entrante, API REST,
socket de escucha ni IPC con otros procesos.

## 2. La CLI como API

Es la interfaz de integración pensada para automatización. Contrato completo en
[05 · Referencia técnica](05-technical-reference.md); aquí van sus propiedades como API.

### 2.1 Contrato

| Propiedad | Valor |
|---|---|
| Invocación | `rootcause <comando> [banderas]` |
| Salida legible | `stdout`, texto tabulado en español |
| Salida estructurada | `stdout`, JSON con formato, con `--json` |
| Errores | `stderr`, texto |
| Códigos | `0` correcto · `1` error de ejecución · `2` uso incorrecto |
| Concurrencia | Cada invocación es un proceso independiente |
| Idempotencia | Los comandos de consulta sí; `--accept`, `clean-caches --yes` y `kill` no |

### 2.2 Comandos con salida JSON

`status`, `snapshot`, `history`, `incidents`, `audit`, `persistence`, `security`, `xprotect`,
`tcc`, `connections`, `network`, `events`, `config show` y `ai explain-latest`.

La forma del JSON es la serialización directa de los modelos de `src/models.rs` con `serde`.
Eso significa que **el esquema del JSON es el esquema de los modelos**: añadir un campo a un
struct lo añade a la salida.

### 2.3 Ejemplos de integración

Comprobar en un script si Gatekeeper está activo:

```bash
rootcause security --json | /usr/bin/python3 -c \
  'import json,sys; d=json.load(sys.stdin); print(next(c["status"] for c in d if c["id"]=="gatekeeper"))'
```

Fallar un script si hay algún control de seguridad en crítico:

```bash
rootcause security --json | grep -q '"severity": "Critical"' && exit 1
```

Guardar una captura completa fechada:

```bash
rootcause snapshot --output "$HOME/capturas/rootcause-$(date +%Y%m%d-%H%M).json"
```

Revisar los incidentes del último arranque en formato legible:

```bash
rootcause incidents 5
```

`REQUIERE VALIDACIÓN`: no existe en el repositorio ningún ejemplo de integración con
herramientas externas (SIEM, Jamf, Munki); los de arriba son construcciones de este análisis
basadas en el contrato real del CLI.

### 2.4 Estabilidad

`NO DOCUMENTADO EN EL REPOSITORIO`: no hay compromiso explícito de estabilidad del formato
JSON entre versiones. Los modelos usan `#[serde(default)]` en los campos añadidos, lo que
sugiere intención de compatibilidad hacia atrás al **leer**, pero nada garantiza que un campo
no se renombre. En la versión 0.1.0 conviene tratar el esquema como inestable.

## 3. Utilidades de macOS consumidas

Es la «API» real del producto: veinte binarios del sistema. Todos se invocan con **ruta
absoluta**, lo que evita que un `PATH` manipulado sustituya una utilidad por otra cosa.

| Binario | Argumentos | Módulo | Qué aporta | Si falla |
|---|---|---|---|---|
| `/bin/ps` | `-axo pid=,user=,command=` | `macos.rs` | Usuario y línea de comandos | Tabla vacía; procesos sin usuario |
| `/usr/sbin/lsof` | `-i -n -P -FpcLftPnT` | `macos.rs` | Conexiones por proceso | Sección Conexiones vacía |
| `/usr/bin/codesign` | `-dvv <ruta>` | `macos.rs` | Firma de código | `CodeSignature::Unknown` |
| `/usr/sbin/spctl` | `--status` | `security.rs` | Gatekeeper | Control «Desconocido» en amarillo |
| `/usr/bin/csrutil` | `status` | `security.rs` | SIP | Idem |
| `/usr/bin/fdesetup` | `status` | `security.rs` | FileVault | Idem |
| `/usr/libexec/ApplicationFirewall/socketfilterfw` | `--getglobalstate`, `--getstealthmode` | `security.rs` | Firewall y modo encubierto | Respaldo por `defaults` |
| `/usr/bin/defaults` | `read /Library/Preferences/com.apple.alf globalstate` | `security.rs` | Respaldo del firewall | Control «Desconocido» |
| `/bin/launchctl` | `list` | `launchd.rs` | Servicios cargados y SSH | Sin estado de servicios |
| `/usr/bin/crontab` | `-l` | `launchd.rs` | Tareas `cron` del usuario | Sin tareas `cron` |
| `/usr/bin/osascript` | `-e <script>` | `launchd.rs`, `macos.rs` | Login items y notificaciones | Sin login items ni notificaciones |
| `/usr/sbin/arp` | `-a -n` | `macos.rs` | Vecinos de red | Sección Red vacía |
| `/sbin/route` | `-n get default` | `macos.rs` | Interfaz y puerta de enlace | Sin contexto de red |
| `/sbin/ifconfig` | `<interfaz>` | `macos.rs` | IP y MAC locales | Sin IP propia |
| `/sbin/ping` | `-c 1 -t 1 -q <ip>` | `macos.rs` | Barrido activo | Sin descubrimiento |
| `/usr/bin/dscacheutil` | `-q host -a ip_address <ip>` | `macos.rs` | DNS inverso | Sin nombres de host |
| `/usr/bin/log` | `show --last Nm --style compact --predicate …` | `macos.rs` | Eventos de seguridad | Sección de eventos vacía |
| `/usr/sbin/sysctl` | `-n <clave>` | `macos.rs` | Modelo y CPU | Campos vacíos en Acerca |
| `/usr/bin/sw_vers` | `-productVersion`, `-productName`, `-buildVersion` | `macos.rs` | Versión de macOS | Campos vacíos |
| `/usr/bin/id` | `-un`, `-u` | `macos.rs` | Usuario y UID | Respaldo `501` |
| `/bin/kill` | `-TERM <pid>` | `macos.rs` | Finalizar proceso | Error explícito |
| `/usr/bin/open` | `-R <ruta>` | `macos.rs` | Revelar en Finder | Error explícito |
| `/usr/bin/which` | `<programa>` | `macos.rs` | Comprobar disponibilidad | Se asume ausente |
| `/usr/bin/curl` | `--silent --show-error --fail --max-time N -X POST …` | `ai.rs` | Petición al proveedor IA | Error explícito |

### 3.1 Formato de las respuestas que se parsean

| Utilidad | Forma esperada | Fragilidad |
|---|---|---|
| `spctl --status` | Contiene `assessments enabled` o `assessments disabled` | Media: si Apple cambia el texto, pasa a «Desconocido» |
| `csrutil status` | Contiene `status: enabled` / `disabled` / `custom` | Media |
| `fdesetup status` | Contiene `FileVault is On` / `Off` | Media |
| `socketfilterfw --getglobalstate` | Contiene `state = 0/1/2` | Media, con respaldo |
| `socketfilterfw --getstealthmode` | `stealth mode enabled` o `stealth mode is on` (**ambas aceptadas**) | Baja: ya contempla dos redacciones |
| `arp -a -n` | `? (ip) at mac on iface …` | Baja: parseo posicional tolerante |
| `lsof -F` | Líneas `<letra><valor>` | Baja: formato estable y diseñado para máquinas |
| `codesign -dvv` | Líneas `Authority=…`, `Signature=…` | Baja para los casos cubiertos |
| `launchctl list` | `PID Status Label` | Baja |
| `log show --style compact` | `<hora> <tipo> <actividad> <pid> <proceso>: <mensaje>` | Media |

Los casos de severidad media están cubiertos por la regla «desconocido nunca es verde»: si el
formato cambia, el producto lo dice en vez de fingir que todo está bien.

## 4. Integración con el proveedor de IA

### 4.1 Condiciones para que exista

Las tres, en orden:

1. `ai.enabled = true` en la configuración.
2. `ai.endpoint` no vacío.
3. La variable de entorno cuyo nombre indica `ai.api_key_env_var` (por defecto
   `ROOTCAUSE_AI_API_KEY`) existe.

Con los valores de fábrica **ninguna se cumple** y el código no toca la red.

### 4.2 Petición

| Aspecto | Valor |
|---|---|
| Método | `POST` |
| URL | La configurada en `ai.endpoint` |
| Cabeceras | `Content-Type: application/json`, `Authorization: Bearer <clave>` |
| Cuerpo | Por `stdin` de `curl` (`--data @-`) |
| Tiempo máximo | `ai.timeout_secs` (25 s por defecto) |
| Reintentos | **Ninguno**. Un fallo se devuelve como error |
| Transporte | `curl --silent --show-error --fail` |

Cuerpo (estructura real generada por `build_payload`, con valores ficticios):

```json
{
  "model": "gpt-4.1-mini",
  "temperature": 0.1,
  "response_format": { "type": "json_object" },
  "messages": [
    {
      "role": "system",
      "content": "Eres un analista forense de macOS. Explicas hallazgos con precisión y sin alarmismo…"
    },
    {
      "role": "user",
      "content": "Incidente detectado en un Mac.\nTítulo: …\nTipo: …\nResumen: …"
    }
  ]
}
```

El `content` del mensaje de usuario es una sola cadena; desplegada en líneas, con un
incidente de ejemplo ficticio, dice exactamente esto:

```text
Incidente detectado en un Mac.
Título: Persistencia nueva detectada
Tipo: persistence-change
Resumen: LaunchDaemon 'com.ejemplo.helper' apareció respecto a la baseline conocida.
Hipótesis local: algo instaló, modificó o quitó un mecanismo de arranque automático
Evidencia: Ubicación: /Library/LaunchDaemons/com.ejemplo.helper.plist |
           Comando: /usr/local/bin/helper | Cambio: NUEVA

Devuelve SOLO un objeto JSON con las claves: summary (string),
probable_causes (array de strings), suggested_actions (array de strings),
confidence (string: alta|media|baja), warnings (array de strings).
```

### 4.3 Respuesta esperada

Compatible con la API de chat de OpenAI. El código navega `choices[0].message.content`, que
debe contener a su vez un JSON con esta forma:

```json
{
  "summary": "Un LaunchDaemon nuevo ejecuta un binario desde una ruta de usuario.",
  "probable_causes": ["Instalación reciente de software", "Persistencia añadida por un tercero"],
  "suggested_actions": ["Comprobar el origen del binario", "Revisar la fecha del plist"],
  "confidence": "media",
  "warnings": ["No se puede confirmar intención maliciosa solo con estos datos."]
}
```

`probable_causes`, `suggested_actions`, `confidence` y `warnings` son opcionales
(`#[serde(default)]`): si el modelo devuelve menos campos, no rompe. `summary` es obligatorio.

### 4.4 Errores

| Situación | Mensaje | Código CLI |
|---|---|---|
| IA desactivada | `La integración IA está desactivada en la configuración` | `1` |
| Sin endpoint | «Falta `ai.endpoint` en la configuración» | `1` |
| Sin clave | `No existe la variable de entorno … con la API key` | `1` |
| `curl` falla o el proveedor responde ≠ 2xx | `El proveedor IA respondió con error: <stderr>` | `1` |
| Respuesta no es JSON | `La respuesta del proveedor IA no es JSON válido` | `1` |
| Falta `choices[0].message.content` | «La respuesta IA no trae `choices[0].message.content`» | `1` |
| Contenido con forma inesperada | `El contenido devuelto por la IA no tiene la forma esperada` | `1` |

En todos los casos el CLI añade: *RootCause sigue funcionando con normalidad sin ella*. El
incidente local ya persistido **no se toca**.

### 4.5 Qué se registra de la integración

`update_incident_ai` guarda el consejo dentro del `payload_json` del incidente, y la auditoría
registra `ai-explain-latest` con el `incident_id` y el resumen. El campo `provider` se deduce
del **host** del endpoint (`provider_from_endpoint`), no de la URL completa: se deja
constancia de a dónde se envió sin guardar rutas ni parámetros.

### 4.6 Límites y dependencias del proveedor

| Aspecto | Estado |
|---|---|
| Control de tarifa / cuotas | **No implementado**. Cada invocación es una petición |
| Reintento con espera | **No implementado** |
| Caché de respuestas | **No implementado**; sí se guarda el consejo en el incidente |
| Coste | A cargo del usuario, con su propia clave |
| Modelo por defecto | `gpt-4.1-mini` (`config.ai.model`) |
| Compatibilidad | Cualquier proveedor con API de chat estilo OpenAI: `REQUIERE VALIDACIÓN` para cada uno |

## 5. Notificaciones del sistema

| Aspecto | Detalle |
|---|---|
| Mecanismo | `osascript -e 'display notification "…" with title "…"'` |
| Cuándo | Captura con alerta crítica **y** `alerting.notify_on_critical = true` |
| Contenido | `RootCause · <título>` y el detalle de la alerta |
| Saneado | Las comillas dobles se sustituyen por simples antes de componer el script |
| Fallo | Silencioso: si el usuario deshabilitó las notificaciones, no pasa nada |
| Anti-repetición | `notification_cooldown_secs` existe en la configuración pero **no se usa** |

Esa última fila es un hallazgo: con una condición crítica persistente y refresco de 5
segundos, la notificación puede repetirse en cada captura. Se registra en
[15 · Riesgos](15-risks-and-technical-debt.md).

## 6. Integración con GitHub

### 6.1 Workflows

| Workflow | Disparo | Permisos | Acciones de terceros |
|---|---|---|---|
| `ci.yml` | push/PR a `main`, manual | `contents: read` | `actions/checkout@v7`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `actions/upload-artifact@v7` |
| `release-macos.yml` | tags `v*`, manual | `contents: write` | Las anteriores + `softprops/action-gh-release@v3` |
| `deploy-landing.yml` | push a `main` en `landing/**`, manual | `contents: read`, `pages: write`, `id-token: write` | `actions/configure-pages@v6`, `actions/upload-pages-artifact@v5`, `actions/deploy-pages@v5` |

Los permisos son mínimos por workflow: solo el de release puede escribir en el repositorio.
`markdownlint-cli2` se ejecuta con `npx --yes`, es decir, **se descarga en cada ejecución**
sin versión fijada: se anota como riesgo de cadena de suministro en
[11 · Seguridad](11-security.md).

### 6.2 Publicación de la release

`scripts/release-product.sh --publish` usa la CLI `gh` para etiquetar y publicar. El propio
script comprueba seis condiciones antes de tocar el remoto (`REQUIERE VALIDACIÓN`: no se
ejecutó en este análisis).

## 7. Distribución

| Canal | Estado | Detalle |
|---|---|---|
| Compilación desde el código | **Recomendado** | `cargo build --release` |
| `.dmg` de la release de GitHub | Activo | Sin firmar ni notarizar |
| `RootCause-app.zip` | Activo | Bundle comprimido |
| `SHA256SUMS.txt` | Activo | Integridad de los artefactos |
| Homebrew cask | **Plantilla** | `packaging/homebrew/rootcause.rb`, sin tap publicado |
| Mac App Store | No | Incompatible con lo que hace el producto: `INFERENCIA` |
| crates.io | No | `publish = false` en `Cargo.toml` |

## 8. Lo que el producto deliberadamente no integra

| No hace | Por qué (según el propio repositorio) |
|---|---|
| Telemetría o analítica | «Todo el análisis es local» |
| Comprobación de actualizaciones | Implicaría una salida a red constante |
| Envío de informes de fallo | Idem |
| Servidor local de administración | Fuera del alcance de la edición de escritorio |
| Aplicar reglas de firewall | `block-ip` entrega el comando; ejecutarlo es decisión del usuario |
| Eliminar archivos sospechosos | «Diagnóstico primero. Intervención después» |
| Instalarse como LaunchAgent | Una herramienta que vigila la persistencia ajena no añade la suya en silencio |

---

**Siguiente lectura recomendada:** [10 · Configuración](10-configuration.md).
