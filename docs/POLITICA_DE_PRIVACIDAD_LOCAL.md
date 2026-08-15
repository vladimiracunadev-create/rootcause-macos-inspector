# Política de privacidad local

## Resumen

**RootCause no envía datos a ninguna parte.** No hay servidor, ni cuenta, ni telemetría, ni
estadísticas de uso, ni comprobación de actualizaciones. Todo lo que recoge se queda en tu equipo,
en carpetas tuyas.

## Qué recoge y dónde lo guarda

```text
~/Library/Application Support/RootCauseInspector/
├── rootcause-config.json        ← tu configuración
├── rootcause-history.db         ← SQLite: historial, incidentes, auditoría, baselines
└── rootcause-agent-state.json   ← estado de salud del agente

~/Documents/RootCause/reports/   ← reportes que tú generas
~/Downloads/                     ← capturas JSON que tú exportas
```

### Contenido del SQLite

| Tabla | Qué guarda | Retención |
|---|---|---|
| `snapshots` | CPU, memoria, E/S, cachés, proceso dominante y alertas de cada captura | 1000 filas |
| `incidents` | Incidentes resumidos con evidencia | 300 filas |
| `audit_log` | Acciones ejecutadas, con su resultado | Sin límite |
| `persistence_baseline` | Estado bueno conocido de la persistencia | Estado actual |
| `baseline` | Estado bueno conocido de seguridad, TCC y red | Estado actual |

Los datos incluyen nombres de proceso, rutas de ejecutables, direcciones IP de destinos, rutas de
`.plist` y bundle ids de aplicaciones. **Ninguno sale del equipo.**

## La única excepción: el adaptador de IA opcional

Está **apagado por defecto**. Para que envíe algo hacen falta tres acciones deliberadas tuyas:

1. Poner `ai.enabled = true` en la configuración.
2. Definir `ai.endpoint` con la URL de tu proveedor.
3. Exportar la clave en la variable de entorno `ROOTCAUSE_AI_API_KEY`.

Cuando está activado y ejecutas `rootcause ai explain-latest`, se envía **únicamente el incidente
ya resumido**: título, tipo, resumen, hipótesis y evidencia. Nunca la captura completa, ni la lista
de procesos, ni las rutas de tu usuario, ni los permisos TCC, ni el historial.

La clave **no se guarda en el archivo de configuración**: solo se guarda el nombre de la variable
de entorno donde vive.

Cada llamada queda registrada en la auditoría local, así que siempre puedes revisar qué se envió y
cuándo:

```bash
rootcause audit | grep ai-explain-latest
```

## Qué NO hace

- No abre puertos ni acepta conexiones entrantes.
- No comprueba actualizaciones ni contacta con ningún servidor propio.
- No lee tu correo, mensajes, fotos, contactos ni calendario.
- No registra pulsaciones de teclado ni captura pantalla.
- No sube reportes a ninguna parte: los guarda en tu carpeta de Documentos.

## Antes de compartir un reporte

Un reporte forense o una captura JSON contienen información de tu equipo: nombre del host, rutas
que incluyen tu nombre de usuario, aplicaciones instaladas y direcciones IP con las que hablaste.
**Revísalos antes de adjuntarlos a un ticket o a un issue público.**

## Borrar todo

```bash
rm -rf ~/Library/Application\ Support/RootCauseInspector
rm -rf ~/Documents/RootCause
```

Y, si concediste Acceso total al disco, retíralo en Ajustes del Sistema → Privacidad y seguridad.
