# 10 · Configuración

> `rootcause-config.json` campo por campo: valor por defecto, rango razonable, qué hace y
> qué pasa si se pone mal. Todos los valores de ejemplo de este documento son ficticios;
> el producto **no almacena ningún secreto** en su configuración.

---

## 1. Dónde vive

| Elemento | Ruta |
|---|---|
| Archivo | `~/Library/Application Support/RootCauseInspector/rootcause-config.json` |
| Cómo se resuelve | `dirs::data_local_dir()` → respaldo `dirs::data_dir()` → respaldo `.` |
| Carpeta | `meta::APP_DIR` = `RootCauseInspector` |
| Nombre | `DEFAULT_CONFIG_FILE` = `rootcause-config.json` |

**El archivo no existe hasta que se crea explícitamente.** Sin él, el producto funciona con
los valores por defecto:

```bash
rootcause config init    # lo crea con los valores por defecto
rootcause config show    # muestra rutas y configuración efectiva
```

## 2. Cómo se carga

`ConfigManager::load_or_default` nunca falla. Tres caminos:

| Situación | Configuración usada | Advertencia |
|---|---|---|
| El archivo no existe | Valores por defecto | Ninguna |
| El archivo existe y es válido | La del archivo | Ninguna |
| El archivo existe y el JSON es inválido | Valores por defecto | `Configuración inválida en <ruta>. Se usan valores por defecto: <error>` |
| El archivo existe y no se puede leer | Valores por defecto | `No se pudo leer <ruta>. Se usan valores por defecto: <error>` |

La advertencia se convierte en una **alerta de la captura** en cada refresco, con la pista de
ejecutar `rootcause config init`. No se silencia sola.

**Un archivo parcial es válido.** Cada campo declara `#[serde(default = "…")]`, así que este
JSON es perfectamente correcto y solo cambia el intervalo:

```json
{ "collection": { "refresh_interval_secs": 15 } }
```

## 3. Cómo se guarda

| Vía | Efecto |
|---|---|
| `rootcause config init` | Crea el archivo si falta; no toca uno existente |
| Sección Configuración de la GUI | Guarda al pulsar «Guardar configuración» **y también al cambiar cualquier control** |
| Edición manual del JSON | Se lee en el siguiente arranque; la resiliencia lo detecta como cambio de huella |

Editar el archivo a mano mientras la aplicación está abierta no tiene efecto hasta reiniciar,
y en el arranque siguiente el monitor de resiliencia marcará el estado como `Degraded` con
el motivo «La configuración cambió respecto a la sesión anterior».

## 4. Configuración completa por defecto

Equivalente a la salida de `rootcause config show --json` con los valores de fábrica (los
arrays se muestran compactados para que quepan en la página):

```json
{
  "collection": {
    "refresh_interval_secs": 5,
    "history_limit": 1000,
    "incident_limit": 300,
    "verify_signatures": true,
    "signature_budget": 12
  },
  "thresholds": {
    "process": {
      "cpu_warning_percent": 30.0,
      "cpu_critical_percent": 65.0,
      "memory_warning_mb": 1000.0,
      "memory_critical_mb": 2500.0,
      "io_write_warning_mb": 40.0,
      "io_write_critical_mb": 200.0
    },
    "cache": {
      "warning_mb": 2048.0,
      "critical_mb": 8192.0
    },
    "xprotect": {
      "warning_days": 30,
      "critical_days": 90
    }
  },
  "anomaly": {
    "enabled": true,
    "cpu_sustained_percent": 55.0,
    "cpu_sustained_samples": 3,
    "memory_growth_mb": 250.0,
    "memory_growth_samples": 2,
    "aggressive_write_mb": 120.0,
    "aggressive_write_samples": 2,
    "public_destination_count": 4,
    "local_scan_destination_count": 8,
    "respawn_window_secs": 180,
    "respawn_count": 2,
    "suspicious_path_keywords": [
      "/tmp/", "/private/tmp/", "/var/tmp/", "/users/shared/",
      "/downloads/", "/.hidden", "/library/application support/."
    ],
    "trusted_process_names": [
      "kernel_task", "launchd", "windowserver", "mds", "mds_stores",
      "mdworker_shared", "kernelmanagerd", "backupd"
    ],
    "trusted_path_prefixes": [
      "/system/", "/usr/", "/bin/", "/sbin/", "/applications/", "/library/apple/"
    ],
    "suspicious_parent_names": [
      "bash", "zsh", "sh", "osascript", "python3", "perl", "ruby", "curl",
      "microsoft word", "microsoft excel"
    ],
    "shell_interpreters": [
      "bash", "zsh", "sh", "osascript", "python", "python3", "perl", "ruby", "node"
    ],
    "watch_persistence": true,
    "watch_security_controls": true,
    "watch_tcc": true,
    "watch_network_devices": true,
    "watch_unsigned_binaries": true
  },
  "alerting": {
    "max_alerts": 8,
    "notify_on_critical": true,
    "notification_cooldown_secs": 90
  },
  "remediation": {
    "manual_actions_enabled": true,
    "automatic_actions_enabled": false
  },
  "resilience": {
    "enabled": true,
    "heartbeat_interval_secs": 15,
    "stale_after_secs": 90,
    "restart_window_secs": 600,
    "max_restarts_in_window": 3,
    "watch_config_integrity": true
  },
  "ai": {
    "enabled": false,
    "endpoint": "",
    "model": "gpt-4.1-mini",
    "api_key_env_var": "ROOTCAUSE_AI_API_KEY",
    "timeout_secs": 25
  },
  "ui": {
    "language": "es",
    "theme": "dark",
    "daily_report": false
  }
}
```

## 5. `collection` — ritmo y alcance de la captura

| Campo | Tipo | Defecto | Qué hace | Consecuencia de un valor malo |
|---|---|---:|---|---|
| `refresh_interval_secs` | u64 | `5` | Segundos entre capturas automáticas de la GUI | La GUI fuerza un mínimo de 2 s. Un valor muy alto retrasa la detección de cambios |
| `history_limit` | usize | `1000` | Filas conservadas en `snapshots` | Muy bajo pierde tendencia; muy alto hace crecer el archivo |
| `incident_limit` | usize | `300` | Filas conservadas en `incidents` | Muy bajo pierde evidencia antigua |
| `verify_signatures` | bool | `true` | Ejecuta `codesign` sobre los procesos seleccionados | En `false` se pierde la señal más fuerte de macOS |
| `signature_budget` | usize | `12` | Máximo de binarios verificados por captura | Muy alto encarece cada captura (un proceso por binario) |

**Interacción importante:** con `verify_signatures = false`, las heurísticas
`unsigned-binary` y las señales de firma de `classify_process` y `classify_entry` dejan de
disparar, porque `signature` queda en `None`. La configuración de anomalías no cambia, pero
la señal desaparece.

## 6. `thresholds.process` — umbrales por proceso

| Campo | Defecto | Rango en la GUI | Efecto |
|---|---:|---|---|
| `cpu_warning_percent` | `30.0` | 5–100 | +18 puntos al superarlo |
| `cpu_critical_percent` | `65.0` | 5–100 | +35 puntos |
| `memory_warning_mb` | `1000.0` | 100–16 000 | +14 puntos |
| `memory_critical_mb` | `2500.0` | — | +28 puntos |
| `io_write_warning_mb` | `40.0` | — | +20 puntos |
| `io_write_critical_mb` | `200.0` | 10–2 000 | +40 puntos |

**No se valida que crítico > aviso.** Si se invierten, el código evalúa primero la rama
crítica, de modo que un `cpu_critical_percent` menor que el de aviso hace que la rama de
aviso sea inalcanzable. El test `valores_por_defecto_son_razonables` comprueba la relación
solo para los valores de fábrica.

## 7. `thresholds.cache` y `thresholds.xprotect`

| Campo | Defecto | Efecto |
|---|---:|---|
| `cache.warning_mb` | `2048.0` | Caché ≥ 2 GB → `Warning` |
| `cache.critical_mb` | `8192.0` | Caché ≥ 8 GB → `Critical` |
| `xprotect.warning_days` | `30` | Definiciones de más de 30 días → `Warning` |
| `xprotect.critical_days` | `90` | Más de 90 días → `Critical` |

Los umbrales de XProtect codifican una expectativa: Apple publica firmas con frecuencia, así
que 30 días sin actualizar sugiere que algo va mal en las actualizaciones automáticas y 90
días lo confirma.

## 8. `anomaly` — el motor de heurísticas

### 8.1 Interruptores

| Campo | Defecto | Qué apaga |
|---|---|---|
| `enabled` | `true` | **Todo el motor.** Además limpia el historial de rachas |
| `watch_persistence` | `true` | Eventos por cambios en launchd (la marca visual se conserva) |
| `watch_security_controls` | `true` | Eventos por cambios en Gatekeeper, SIP, FileVault… |
| `watch_tcc` | `true` | Eventos por permisos nuevos |
| `watch_network_devices` | `true` | Eventos por equipos nuevos en la red |
| `watch_unsigned_binaries` | `true` | La heurística `unsigned-binary` |

Apagar un `watch_*` **no impide el diff**: el `change_status` sigue calculándose y la interfaz
sigue mostrando NUEVA/MODIFICADA/ELIMINADA. Lo que desaparece es el evento de anomalía y, con
él, la alerta.

### 8.2 Umbrales de comportamiento

| Campo | Defecto | Heurística | Lectura |
|---|---:|---|---|
| `cpu_sustained_percent` | `55.0` | `sustained-cpu` | Porcentaje que hay que superar |
| `cpu_sustained_samples` | `3` | `sustained-cpu` | Muestras consecutivas; con 5 s son 15 s |
| `memory_growth_mb` | `250.0` | `memory-growth` | Crecimiento sobre la línea base |
| `memory_growth_samples` | `2` | `memory-growth` | Muestras con crecimiento |
| `aggressive_write_mb` | `120.0` | `aggressive-write` | MB escritos en un intervalo |
| `aggressive_write_samples` | `2` | `aggressive-write` | Muestras consecutivas |
| `public_destination_count` | `4` | `unusual-outbound` | Destinos públicos distintos |
| `local_scan_destination_count` | `8` | `local-scan` | Equipos privados distintos |
| `respawn_window_secs` | `180` | `fast-respawn` | Ventana de observación |
| `respawn_count` | `2` | `fast-respawn` | Cambios de PID dentro de la ventana |

**Bajar `*_samples` a 1 anula el principio de diseño del módulo**: un pico instantáneo pasaría
a generar eventos. El test `un_pico_de_cpu_no_dispara_nada` protege el comportamiento con los
valores por defecto, no con valores arbitrarios.

### 8.3 Listas

| Campo | Uso real | Efecto de ampliarla |
|---|---|---|
| `suspicious_path_keywords` | `suspicious-path`; se compara en minúsculas con `contains` | Más rutas marcadas; riesgo de falsos positivos si se añade algo genérico como `/users/` |
| `trusted_process_names` | `is_trusted`, comparación exacta en minúsculas | Excluye ese nombre de **todas** las heurísticas |
| `trusted_path_prefixes` | `is_trusted_path` y selección de firmas | Amplía lo que se considera «instalado con normalidad» |
| `suspicious_parent_names` | **Ninguno**: se declara y no se lee | Sin efecto |
| `shell_interpreters` | **Ninguno**: se declara y no se lee | Sin efecto |

Los dos últimos son configuración sin efecto en el commit analizado; se registra en
[15 · Riesgos](15-risks-and-technical-debt.md). Añadir un prefijo demasiado amplio a
`trusted_path_prefixes` (por ejemplo `/users/`) **desactivaría en la práctica** las
heurísticas de red y de binario sin firmar: es el cambio de configuración con más impacto
negativo posible.

## 9. `alerting`

| Campo | Defecto | Efecto |
|---|---:|---|
| `max_alerts` | `8` | Alertas conservadas tras ordenar por severidad |
| `notify_on_critical` | `true` | Notificación del sistema ante alerta crítica |
| `notification_cooldown_secs` | `90` | **No implementado**: ninguna lectura fuera de `config.rs` |

Con `max_alerts` muy bajo (1 o 2), las alertas de menor severidad pero mayor especificidad
—una persistencia nueva de riesgo medio, por ejemplo— pueden quedar fuera de la vista aunque
sigan en la captura exportada.

## 10. `remediation`

| Campo | Defecto | Efecto |
|---|---|---|
| `manual_actions_enabled` | `true` | Permite `terminate_process`. En `false`, el intento falla con mensaje explícito y queda auditado |
| `automatic_actions_enabled` | `false` | **Nunca se lee.** Existe para dejar la política explícita: RootCause no ejecuta acciones automáticas |

El comentario del código es inequívoco: *RootCause nunca ejecuta acciones automáticas: el
interruptor existe para dejar explícita la política y se mantiene apagado.* Ponerlo a `true`
no habilita nada.

## 11. `resilience`

| Campo | Defecto | Efecto |
|---|---:|---|
| `enabled` | `true` | Activa latido, detección de cierre abrupto y de cambio de configuración |
| `heartbeat_interval_secs` | `15` | Mínimo entre escrituras del archivo de estado |
| `stale_after_secs` | `90` | **No implementado** |
| `restart_window_secs` | `600` | Ventana para contar cierres inesperados |
| `max_restarts_in_window` | `3` | Cierres inesperados que pasan el agente a `Degraded` |
| `watch_config_integrity` | `true` | Compara la huella de la configuración entre sesiones |

Con `enabled = false`, el estado del agente siempre es `Healthy` y no se generan alertas de
resiliencia, pero el archivo de estado se sigue escribiendo en el arranque.

## 12. `ai` — el adaptador opcional

| Campo | Defecto | Notas de seguridad |
|---|---|---|
| `enabled` | `false` | Primera de las tres guardas |
| `endpoint` | `""` | URL completa del proveedor. Segunda guarda |
| `model` | `"gpt-4.1-mini"` | Se envía tal cual en el payload |
| `api_key_env_var` | `"ROOTCAUSE_AI_API_KEY"` | **Nombre** de la variable, nunca la clave |
| `timeout_secs` | `25` | `--max-time` de `curl` |

**La clave nunca se guarda en el archivo de configuración.** Ejemplo de activación con valores
ficticios:

```json
{
  "ai": {
    "enabled": true,
    "endpoint": "https://api.ejemplo-ficticio.test/v1/chat/completions",
    "model": "modelo-ejemplo",
    "api_key_env_var": "ROOTCAUSE_AI_API_KEY",
    "timeout_secs": 25
  }
}
```

Y la clave, fuera del archivo:

```bash
export ROOTCAUSE_AI_API_KEY="clave-de-ejemplo-no-real"
rootcause ai explain-latest
```

**Consecuencia de activarlo:** el producto deja de ser 100 % local. Cada
`ai explain-latest` envía el incidente resumido al endpoint configurado, con todo lo que eso
implica respecto a la política de privacidad del proveedor.

## 13. `ui`

| Campo | Defecto | Valores | Efecto |
|---|---|---|---|
| `language` | `"es"` | `"es"`, `"en"` | Idioma de la interfaz gráfica. **El CLI está solo en español** |
| `theme` | `"dark"` | `"dark"`, `"light"`, `"system"` | Paleta; `system` consulta `defaults read -g AppleInterfaceStyle` |
| `daily_report` | `false` | booleano | **No implementado**: ninguna lectura fuera de `config.rs` |

## 14. Variables de entorno

| Variable | Obligatoria | Efecto |
|---|---|---|
| La declarada en `ai.api_key_env_var` | Solo si se usa la IA | Clave del proveedor |
| `TMPDIR` | No | La define macOS; añade el temporal de sesión a las raíces medidas |
| `USER` | No | Nombre de usuario; respaldo `id -un` |

**No hay soporte de archivo `.env`** ni de variables para sobrescribir la configuración: si
se necesita una configuración distinta por entorno, hay que editar el JSON.

## 15. Diferencias entre entornos

El producto **no distingue desarrollo, pruebas y producción**: no hay perfiles, ni variables
de entorno de modo, ni configuración por entorno. Las únicas diferencias reales son:

| Aspecto | Desarrollo | Distribución |
|---|---|---|
| Binario | `target/debug/rootcause` o `target/release/rootcause` | `RootCause.app` |
| Permisos TCC | Los del terminal que lo ejecuta | Los concedidos al `.app` |
| Datos | Los mismos: `~/Library/Application Support/RootCauseInspector/` | Idem |
| Ediciones | `--no-default-features` para CLI-only | GUI completa |

Que ambos compartan carpeta de datos tiene una consecuencia práctica: **el binario de
desarrollo y el `.app` comparten historial y baselines**. Es cómodo para probar y conviene
saberlo antes de interpretar un cambio inesperado.

## 16. Configuraciones sensibles y sus consecuencias

Resumen de los cambios con más impacto:

| Cambio | Consecuencia |
|---|---|
| `anomaly.enabled = false` | Se apagan las ocho heurísticas y se limpia el historial de rachas |
| `collection.verify_signatures = false` | Desaparece la señal de firma en procesos y en persistencia |
| Añadir un prefijo amplio a `trusted_path_prefixes` | Silencia heurísticas de red y de binarios sin firmar |
| Añadir un nombre común a `trusted_process_names` | Ese proceso queda exento de todo el motor |
| `remediation.manual_actions_enabled = false` | No se puede finalizar ningún proceso desde el producto |
| `alerting.notify_on_critical = false` | No hay aviso fuera de la ventana |
| `ai.enabled = true` | El producto deja de ser exclusivamente local |
| `collection.refresh_interval_secs` muy alto | Ventana ciega mayor entre capturas |
| Borrar el archivo de configuración | Se vuelve a los valores por defecto; el agente lo marca como cambio |

## 17. Qué hacer si la configuración se corrompe

```bash
mv ~/Library/Application\ Support/RootCauseInspector/rootcause-config.json \
   ~/Library/Application\ Support/RootCauseInspector/rootcause-config.json.roto
rootcause config init
rootcause config show
```

El historial, los incidentes y las baselines **no se pierden**: viven en el SQLite, que es un
archivo distinto.

---

**Siguiente lectura recomendada:** [11 · Seguridad](11-security.md).
