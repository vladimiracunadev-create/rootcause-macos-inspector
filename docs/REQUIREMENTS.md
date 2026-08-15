# Requisitos

## Entorno de ejecución

| Requisito | Mínimo | Recomendado |
|---|---|---|
| macOS | 13 (Ventura) | 14 o superior |
| Arquitectura | Apple Silicon o Intel | Apple Silicon |
| RAM | 4 GB | 8 GB |
| Espacio en disco | 60 MB | 100 MB (con historial) |
| Privilegios | Usuario normal | Usuario normal + Acceso total al disco |

## Entorno de compilación

| Requisito | Versión |
|---|---|
| Rust | estable reciente (edición 2021) |
| Herramientas de línea de comandos de Xcode | cualquiera |

## Utilidades del sistema que utiliza

Todas vienen con macOS. Si alguna falta, la superficie correspondiente queda vacía y se declara en
la sección «Contexto de ejecución».

| Utilidad | Para qué |
|---|---|
| `/usr/sbin/lsof` | Conexiones activas por proceso |
| `/usr/sbin/spctl` | Estado de Gatekeeper |
| `/usr/bin/csrutil` | Estado de SIP |
| `/usr/bin/fdesetup` | Estado de FileVault |
| `/usr/libexec/ApplicationFirewall/socketfilterfw` | Firewall y modo encubierto |
| `/usr/bin/codesign` | Firma de código |
| `/bin/launchctl` | Servicios de launchd |
| `/usr/sbin/arp`, `/sbin/route`, `/sbin/ifconfig` | Red local |
| `/bin/ps` | Usuario y línea de comandos por proceso |
| `/usr/bin/log` | Eventos de seguridad (solo bajo demanda) |
| `/usr/bin/osascript` | Notificaciones y login items (bajo demanda) |
| `/usr/bin/curl` | Adaptador de IA opcional (apagado por defecto) |

## Requisitos funcionales

### RF-01 · Inventario de persistencia

Listar todas las entradas de LaunchAgents, LaunchDaemons, login items y `cron`, con su comando,
ámbito, firma y riesgo calculado.

### RF-02 · Detección de cambios

Comparar cada superficie vigilada contra un estado bueno conocido y clasificar cada elemento como
NUEVO, MODIFICADO o ELIMINADO. La primera captura siembra la baseline sin generar alertas.

### RF-03 · Estado de los controles nativos

Reportar el estado de Gatekeeper, SIP, FileVault, firewall, modo encubierto y acceso remoto, cada
uno con la evidencia textual del comando consultado.

### RF-04 · Auditoría de permisos de privacidad

Leer las bases TCC del usuario y del sistema y listar los permisos concedidos, con severidad por
servicio. Si no se puede leer, declararlo explícitamente.

### RF-05 · Observación de procesos y red

Consumo por proceso, firma de código, conexiones activas, destinos públicos, puertos a la escucha y
vecinos del segmento local.

### RF-06 · Correlación en incidentes

Fusionar señales en un incidente resumido con severidad, hipótesis de causa, evidencia y acción
recomendada. Persistirlo sin duplicar la misma condición en capturas consecutivas.

### RF-07 · Evidencia exportable

Exportar la captura en JSON y generar un reporte forense en Markdown legible por una persona.

### RF-08 · Auditoría de acciones

Registrar toda acción ejecutada desde la GUI o la CLI, con su resultado.

### RF-09 · Doble interfaz

Ofrecer la misma funcionalidad por GUI y por CLI, con `--json` para automatización.

### RF-10 · Salud del propio agente

Detectar cierres abruptos, cambios de configuración y reinicios repetidos, y exponerlos.

## Requisitos no funcionales

### RNF-01 · La interfaz no se bloquea

Ninguna operación de captura puede congelar la ventana. El motor corre en un hilo propio.

### RNF-02 · Análisis local

Ningún dato sale del equipo, salvo el adaptador de IA opcional (apagado por defecto), que envía
únicamente el incidente ya resumido.

### RNF-03 · Sin escalada de privilegios

No se usa `sudo` implícito ni se solicitan privilegios que el usuario no haya concedido.

### RNF-04 · Fallo suave

Una superficie que falla no impide el resto de la captura; su ausencia se declara.

### RNF-05 · Sin acciones automáticas

Toda intervención parte de una acción explícita del usuario y queda auditada.

### RNF-06 · Coste acotado

El escaneo de cachés tiene tope de entradas; la verificación de firmas tiene presupuesto por
captura; las consultas caras (log unificado, escaneo profundo) solo se ejecutan bajo demanda.
