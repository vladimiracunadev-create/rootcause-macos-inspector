# REQ-SEC-003 · Cobertura de superficies nativas de macOS

| Campo | Valor |
|---|---|
| Prioridad | Alta |
| Estado | V1 implementada |
| Módulos | `services/launchd.rs`, `services/security.rs`, `services/tcc.rs`, `services/netscan.rs` |

## Problema

La edición Windows de RootCause vigila el registro, los servicios y las tareas programadas. En
macOS esas superficies no existen: la persistencia vive en archivos `.plist`, las defensas son
Gatekeeper, SIP, FileVault y XProtect, y los permisos se gobiernan por TCC.

Trasladar el producto sin traducir las superficies produciría una herramienta que compila en macOS
pero no dice nada útil sobre macOS.

## Requerimiento

RootCause debe cubrir las superficies **idiomáticas de macOS**, con la evidencia nativa de cada una
y con el mismo motor de baseline que la edición Windows.

## Criterios de aceptación

| # | Criterio | Estado |
|---|---|---|
| 1 | Inventario de las 5 carpetas de launchd, con las de Apple opcionales | ✅ |
| 2 | Lectura de `Label`, `Program`, `ProgramArguments`, `RunAtLoad`, `KeepAlive`, `StartInterval` | ✅ |
| 3 | Inventario de `cron` del usuario | ✅ |
| 4 | Login items bajo demanda, sin diálogos de permiso no solicitados | ✅ |
| 5 | Estado de Gatekeeper, SIP, FileVault, firewall, modo encubierto y SSH | ✅ |
| 6 | Cada control muestra la evidencia textual del comando consultado | ✅ |
| 7 | Versión y antigüedad de XProtect, XProtect Remediator y MRT | ✅ |
| 8 | Lectura de `TCC.db` de usuario y de sistema, con ambos esquemas soportados | ✅ |
| 9 | Declaración explícita cuando falta Acceso total al disco | ✅ |
| 10 | Verificación de firma de código con presupuesto acotado | ✅ |
| 11 | Vecinos de red con detección de cambio de MAC del gateway | ✅ |
| 12 | Baseline sobre persistencia, seguridad, TCC y red | ✅ |

## Decisiones de traducción

| Superficie Windows | Equivalente macOS | Nota |
|---|---|---|
| Registro `Run` / `RunOnce` | `~/Library/LaunchAgents`, `/Library/LaunchAgents` | Mismo rol: arranque por usuario |
| Servicios de Windows | `/Library/LaunchDaemons` + `launchctl list` | Mismo rol: arranque como root |
| Tareas programadas | `StartInterval`, `StartCalendarInterval`, `cron` | Repartido entre launchd y cron |
| Windows Defender | XProtect + Gatekeeper | Antimalware nativo, sin interfaz propia |
| UAC | TCC | Gobierno de permisos, con modelo distinto |
| `netstat` con PID | `lsof -i` | En macOS `netstat` no da PID |
| Firma Authenticode | `codesign` | Misma pregunta: quién se hace responsable |
| Trazas ETW/WPR | Log unificado (`log show`) | Mucho más caro; solo bajo demanda |

## Fuera de alcance

- Extensiones de sistema y `.kext` (previsto para v0.2).
- Perfiles de configuración de MDM (previsto para v0.3).
- Snapshots de Time Machine.
- Contenido de los llaveros.
