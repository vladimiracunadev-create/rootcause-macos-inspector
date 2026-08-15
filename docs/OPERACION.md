# Operación

## El ciclo de vida de una baseline

Es el concepto que hay que entender para usar bien el producto.

1. **Siembra.** La primera captura de cada superficie se guarda en silencio como estado bueno
   conocido. No genera alertas.
2. **Comparación.** Cada captura posterior se compara contra esa foto.
3. **Reporte pegajoso.** Los cambios se siguen reportando en cada captura, incluso tras reiniciar la
   app o el equipo. Una alerta que se auto-silencia es peor que no tenerla.
4. **Aceptación.** Cuando confirmas que un cambio es legítimo, pulsas «Aceptar baseline» (o
   `--accept` en la CLI) y esa pasa a ser la nueva referencia.

```bash
rootcause persistence --accept   # tras instalar software nuevo
rootcause security --accept      # tras cambiar una configuración a propósito
rootcause tcc --accept           # tras conceder un permiso a una app
rootcause network --accept       # tras incorporar un equipo nuevo a la red
```

## Cuándo sembrar una baseline confiable

El mejor momento es **justo después de instalar el sistema** o tras una limpieza. Cuanto más
conocido sea el estado de partida, más significativo será cada cambio posterior.

Si empiezas con un Mac que llevas años usando, la baseline incluirá lo que ya haya. Eso no invalida
la herramienta: seguirá detectando todo lo que aparezca **a partir de ahora**, que es donde está el
valor.

## Rutinas recomendadas

| Frecuencia | Qué revisar |
|---|---|
| Al instalar RootCause | Seguridad y Privacidad completos; decide qué controles quieres activos |
| Semanal | Persistencia. Es donde aparece lo que sobrevive a un reinicio |
| Tras instalar software | Persistencia: casi todo instalador deja un LaunchAgent |
| Tras un incidente | `rootcause report` **antes** de tocar nada |
| Mensual | XProtect (que las definiciones sigan al día) e Historial |

## Antes de investigar: captura la evidencia

```bash
rootcause report            # Markdown legible, en ~/Documents/RootCause/reports/
rootcause snapshot --output ~/Desktop/evidencia.json
```

Investigar altera el sistema. Capturar primero cuesta dos segundos y evita perder el estado que
querías analizar.

## Interpretar un cambio

Preguntas en este orden:

1. **¿Instalé o actualicé algo recientemente?** Es la causa del 90 % de los cambios.
2. **¿El nombre identifica al software?** Un `Label` como `com.docker.vmnetd` se explica solo.
3. **¿Qué firma tiene el binario?** Developer ID significa que hay alguien identificable detrás.
4. **¿Desde dónde se ejecuta?** `/Applications` y `/Library/PrivilegedHelperTools` son normales;
   `/tmp` y `/Users/Shared` no.
5. **¿Coincide con algo más?** Un LaunchDaemon nuevo *y* un permiso TCC nuevo *y* tráfico saliente
   inusual del mismo binario no son tres coincidencias.

## Dónde vive todo

```text
~/Library/Application Support/RootCauseInspector/
├── rootcause-config.json          ← configuración
├── rootcause-history.db           ← historial, incidentes, auditoría y baselines
└── rootcause-agent-state.json     ← salud del agente

~/Documents/RootCause/reports/     ← reportes en Markdown
```

Copia de seguridad del historial:

```bash
rootcause history --backup
```

Reiniciar todo (borra baselines e historial):

```bash
rm -rf ~/Library/Application\ Support/RootCauseInspector
```

## Coste de una captura

| Superficie | Coste aproximado | Notas |
|---|---|---|
| Procesos | bajo | `sysinfo` + una llamada a `ps` |
| Firmas | medio | Un proceso `codesign` por binario, con presupuesto de 12 y caché por ruta |
| Conexiones | medio | Una llamada a `lsof` |
| Seguridad | bajo | Cinco comandos cortos |
| TCC | bajo | Dos consultas SQLite en solo lectura |
| Persistencia | bajo | Lectura de plists de tres carpetas |
| Cachés | medio | Recorrido acotado a 40 000 entradas por raíz |
| Log unificado | **alto** | Por eso solo se consulta bajo demanda, nunca en la captura periódica |

El intervalo por defecto (5 s) es cómodo para observación activa. Para dejarlo abierto en segundo
plano, súbelo a 30-60 s en Configuración.
