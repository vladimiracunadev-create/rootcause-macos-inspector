# Qué detecta RootCause, amenaza por amenaza

Mapa honesto. Para cada familia de amenaza: qué señal vería RootCause hoy, o por qué queda fuera de
alcance.

El punto de partida es que **toda distorsión de recursos o de configuración puede ser el primer
indicio**. RootCause no identifica familias de malware; detecta comportamiento y cambios.

## Leyenda

- ✅ **Detecta** — hay una señal directa y accionable.
- 🟡 **Indicio** — hay una señal indirecta que apunta en esa dirección.
- ❌ **Fuera de alcance** — RootCause no lo ve, y decirlo es más útil que insinuar que sí.

## Persistencia y post-explotación

| Amenaza | Estado | Señal |
|---|---|---|
| LaunchAgent/Daemon malicioso instalado | ✅ | Entrada NUEVA vs baseline + puntaje de riesgo |
| Secuestro de un LaunchAgent existente | ✅ | Entrada MODIFICADA (mismo `Label`, distinto comando) |
| Implante que imita a Apple (`com.apple.*`) | ✅ | Heurística de `Label` que imita a Apple fuera de las carpetas del sistema |
| Binario sin firmar con persistencia | ✅ | Firma ausente + ámbito + ruta, acumulados |
| Persistencia vía `cron` | ✅ | Se inventaría `crontab -l` del usuario |
| Login item malicioso | 🟡 | Solo bajo demanda (requiere permiso de Automatización) |
| Persistencia en `/System` | ❌ | Protegida por SIP; si algo escribe ahí, SIP ya está comprometido |
| Persistencia por firmware o EFI | ❌ | Fuera del alcance del espacio de usuario |

## Robo de datos y espionaje

| Amenaza | Estado | Señal |
|---|---|---|
| App con Acceso total al disco no autorizada | ✅ | Permiso TCC NUEVO vs baseline |
| Keylogger vía Accesibilidad o Input Monitoring | ✅ | Permiso TCC sensible concedido, con explicación |
| Grabación de pantalla encubierta | ✅ | Permiso `kTCCServiceScreenCapture` concedido |
| Exfiltración a un destino público | 🟡 | Tráfico saliente inusual desde un binario fuera de rutas de instalación |
| Exfiltración lenta y de bajo volumen | ❌ | Indistinguible del tráfico normal sin inspección de contenido |
| Exfiltración por canal encubierto (DNS, ICMP) | ❌ | Requiere análisis de red que RootCause no hace |

## Ransomware

| Amenaza | Estado | Señal |
|---|---|---|
| Cifrado masivo en curso | 🟡 | Escritura agresiva sostenida por proceso |
| Binario de cifrado ejecutándose desde `/tmp` | ✅ | Ruta sospechosa + sin firma, acumulados |
| Borrado de snapshots de Time Machine | ❌ | No se vigila `tmutil` |
| Cifrado lento y por lotes | ❌ | Diseñado precisamente para no superar umbrales |

## Cryptojacking y abuso de recursos

| Amenaza | Estado | Señal |
|---|---|---|
| Minero consumiendo CPU de forma continuada | ✅ | CPU sostenido + ruta + firma |
| Minero con límite de CPU para pasar desapercibido | 🟡 | Solo si además persiste o vive en ruta rara |
| Proceso que se relanza al matarlo | ✅ | Heurística de reapariciones rápidas |

## Debilitamiento de defensas

| Amenaza | Estado | Señal |
|---|---|---|
| Gatekeeper desactivado | ✅ | Control apagado + cambio vs baseline |
| SIP desactivado | ✅ | Control apagado + cambio vs baseline |
| FileVault desactivado | ✅ | Control apagado + cambio vs baseline |
| Firewall apagado | ✅ | Control apagado + cambio vs baseline |
| SSH habilitado sin que lo sepas | ✅ | Servicio cargado en launchd |
| XProtect sin actualizar | ✅ | Antigüedad de las definiciones |
| Manipulación del propio RootCause | 🟡 | Cierre abrupto y cambio de configuración quedan auditados |

## Red

| Amenaza | Estado | Señal |
|---|---|---|
| Equipo desconocido en el segmento | ✅ | Dispositivo NUEVO vs baseline de red conocida |
| Suplantación ARP de la puerta de enlace | ✅ | Cambio de MAC del gateway → crítico |
| Puerto expuesto a toda la red | ✅ | Socket a la escucha en `*:puerto` |
| Movimiento lateral desde este Mac | 🟡 | Heurística de barrido de la red local |
| Ataque desde fuera del segmento | ❌ | RootCause no es un IDS |

## Ingeniería social y ejecución inicial

| Amenaza | Estado | Señal |
|---|---|---|
| Binario descargado ejecutándose desde `~/Downloads` | ✅ | Ruta sospechosa |
| Script malicioso lanzado por `osascript` o `curl` | 🟡 | Si además persiste, la entrada se marca por «ejecuta un intérprete» |
| Phishing / robo de credenciales en el navegador | ❌ | Ocurre dentro del navegador, no en el sistema |
| Extensión maliciosa de navegador | ❌ | Fuera de alcance |

## Lo que RootCause nunca va a hacer

- **Eliminar malware.** No hay cuarentena, ni desinfección, ni borrado automático.
- **Bloquear por firma.** No hay base de firmas ni descarga de definiciones.
- **Interceptar en tiempo real.** No hay extensión de sistema ni driver de kernel; el modelo es de
  muestreo periódico, no de intercepción.
- **Resistir a un atacante con root.** Ver [`../SECURITY.md`](../SECURITY.md).

## Posicionamiento

RootCause **complementa** a XProtect, al firewall de macOS y a tu EDR si tienes uno. Su aporte es
el que ninguno de ellos da: **un inventario con memoria** de lo que cambió en tu equipo, con la
evidencia al lado.
