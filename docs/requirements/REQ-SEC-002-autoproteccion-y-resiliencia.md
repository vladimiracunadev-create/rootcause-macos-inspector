# REQ-SEC-002 · Autoprotección y resiliencia del agente

| Campo | Valor |
|---|---|
| Prioridad | Media |
| Estado | Base inicial implementada |
| Módulos | `services/resilience.rs`, `services/persistence.rs` |

## Problema

Una herramienta de diagnóstico puede convertirse en objetivo. Lo primero que hace un atacante
consciente es callar al que observa. Si RootCause se cierra de golpe y al volver a abrirse empieza
de cero como si nada, esa manipulación pasa desapercibida.

## Requerimiento

El agente debe detectar y dejar constancia de su propia interrupción, de cambios en su
configuración y de patrones de reinicio anómalos, **sin prometer invulnerabilidad**.

## Criterios de aceptación

| # | Criterio | Estado |
|---|---|---|
| 1 | Heartbeat local persistido entre capturas | ✅ |
| 2 | Detección de cierre abrupto de la sesión anterior | ✅ |
| 3 | Conteo de cierres inesperados dentro de una ventana temporal | ✅ |
| 4 | Detección de cambios en el archivo de configuración entre sesiones | ✅ |
| 5 | Estado de salud visible en GUI y CLI | ✅ |
| 6 | Cada evento de resiliencia queda en la auditoría | ✅ |
| 7 | El estado degradado eleva el veredicto de la captura | ✅ |
| 8 | Supervisor de nivel sistema que relance el agente | ❌ Fuera de alcance actual |
| 9 | Verificación criptográfica de la integridad de la configuración | ❌ Fuera de alcance actual |

## Posicionamiento honesto

La huella de integridad de la configuración es **tamaño + fecha de modificación**, no un hash
criptográfico. Detecta cambios accidentales entre sesiones; **no** defiende contra un atacante que
quiera falsificarla. Decir lo contrario sería vender humo.

Del mismo modo, el agente **no resiste a un proceso con root**: puede finalizarlo, alterar su
SQLite o falsear la salida de los comandos que consulta. Lo que sí consigue es que esa manipulación
deje rastro.

## Fuera de alcance

- Protección contra manipulación por parte de root.
- Watchdog persistente de nivel sistema (sería, irónicamente, un LaunchDaemon más — exactamente el
  tipo de cosa que el producto enseña a vigilar).
- Cifrado de la base de datos local.

## Evolución prevista

- Hash criptográfico de la configuración y de las baselines.
- Detección de manipulación del propio SQLite (comparación de contadores esperados).
