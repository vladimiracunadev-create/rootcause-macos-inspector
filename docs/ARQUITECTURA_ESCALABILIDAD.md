# Arquitectura y escalabilidad

Qué aguanta el diseño actual, dónde está el techo y qué habría que cambiar para pasarlo.

## Dónde se va el tiempo de una captura

Medido sobre un MacBook con Apple Silicon, ~600 procesos:

| Fase | Coste | Escala con |
|---|---|---|
| `sysinfo` (procesos y memoria) | ~50 ms | Número de procesos |
| `ps` (usuario y línea de comandos) | ~30 ms | Número de procesos |
| `codesign` × presupuesto | ~40 ms cada uno | **Presupuesto**, no procesos |
| `lsof -i -F` | 100-400 ms | Sockets abiertos |
| Controles de seguridad (5 comandos) | ~150 ms | Constante |
| TCC (2 consultas SQLite) | ~10 ms | Filas en `TCC.db` |
| Persistencia (plists de 3 carpetas) | ~30 ms | Entradas instaladas |
| Cachés | 200 ms - 2 s | **Archivos en disco** (con tope) |
| Vecinos ARP | ~20 ms | Equipos en el segmento |

Dos dominan: `lsof` y el escaneo de cachés. Ambos están acotados.

## Los tres topes deliberados

### Presupuesto de firmas

`codesign` cuesta un proceso por binario. Verificar 600 procesos por captura sería inaceptable, y
eliminar la verificación perdería la señal más valiosa en macOS.

**Solución:** presupuesto configurable (12 por defecto), priorizando lo que ya destaca por
severidad y lo que vive fuera de rutas del sistema, más una **caché por ruta** — un binario no
cambia de firma mientras corre. En estado estacionario, el coste tiende a cero.

### Tope de entradas al medir cachés

40 000 entradas por raíz. Al alcanzarlo, la medición se declara aproximada en la propia sección.

Un monitor que tarda un minuto en refrescar deja de usarse, y una herramienta que no se usa no
protege nada. Se prefiere un número aproximado y honesto a uno exacto e inútil.

### El log unificado no entra en la captura periódica

`log show` tarda segundos. Se consulta **solo bajo demanda**, desde un botón o desde
`rootcause events`. Meterlo en el ciclo de 5 s convertiría la herramienta en el problema que
diagnostica.

## Crecimiento del almacenamiento

| Tabla | Retención | Tamaño estimado |
|---|---|---|
| `snapshots` | 1000 filas | ~1,5 MB |
| `incidents` | 300 filas | ~2 MB |
| `audit_log` | Sin límite | ~100 bytes por acción |
| Baselines | Estado actual | ~200 KB |

El recorte se aplica en cada inserción, así que la base no crece sin control. `audit_log` no se
recorta a propósito: es el registro de lo que la herramienta hizo, y truncarlo destruiría
precisamente la evidencia que justifica tenerlo.

## El techo del diseño actual

| Escenario | ¿Aguanta? | Por qué |
|---|---|---|
| Mac de escritorio, 600 procesos | ✅ | Diseñado para esto |
| Servidor con 5000 procesos | ⚠️ | `sysinfo` y `ps` escalan linealmente; el intervalo habría que subirlo |
| Refresco cada segundo | ❌ | `lsof` no da abasto; el mínimo razonable es 2 s |
| Flota de 500 equipos | ❌ | No hay agente, ni servidor, ni consola. **Fuera del alcance del producto.** |
| Historial de años | ⚠️ | El recorte lo impide por diseño; habría que exportar periódicamente |

## Qué habría que cambiar para pasar cada techo

### Para muchos más procesos

Sustituir la llamada a `ps` por lecturas directas de `libproc`, y paralelizar la verificación de
firmas con un pool de hilos. La arquitectura lo permite: el motor ya está en un hilo separado y no
comparte estado mutable.

### Para intervalos más cortos

Separar las superficies por frecuencia: procesos cada 2 s, red cada 10 s, persistencia y seguridad
cada 60 s. Los datos ya están desacoplados por módulo, así que es un cambio en el orquestador, no
en las superficies.

### Para una flota

Sería **otro producto**. RootCause está pensado para un equipo concreto y para una persona que
mira. Convertirlo en agente de flota implicaría servidor, autenticación, transporte y
almacenamiento central — es decir, romper el principio de análisis local que lo define.

Quien necesite eso, necesita Velociraptor o Wazuh, y este documento lo dice antes de que pierda una
semana intentándolo.

## Lo que el diseño sí facilita

- **Añadir una superficie** es escribir un módulo en `services/` y registrarlo en el orquestador.
  No toca ni la interfaz ni el motor de baseline.
- **Vigilar una superficie nueva contra baseline** es implementar `WatchedItem` y declarar un
  `SurfaceSpec`. El motor genérico hace el resto.
- **Añadir una salida** (CSV, syslog, webhook) es un consumidor más del mismo `SystemSnapshot`.
