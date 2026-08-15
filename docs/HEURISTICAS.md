# Heurísticas

Ninguna heurística de RootCause sabe qué es un malware. Todas responden a la misma idea: **una
distorsión sostenida es el primer indicio de que algo está pasando.**

Dos principios gobiernan el módulo:

- **Sostenido, no instantáneo.** Un pico de CPU no dispara nada; un pico que dura N muestras
  seguidas, sí. Por eso el detector guarda historial por PID entre capturas.
- **Correlación antes que sospecha.** Cada señal suma puntaje; el veredicto sale de la suma, no de
  una sola condición. Así un compilador legítimo no acaba en rojo por consumir CPU.

## Clasificación de procesos

| Señal | Puntos | Umbral por defecto |
|---|---|---|
| CPU crítica | 35 | ≥ 65 % |
| CPU de aviso | 18 | ≥ 30 % |
| Memoria crítica | 28 | ≥ 2500 MB |
| Memoria de aviso | 14 | ≥ 1000 MB |
| Escritura intensa | 40 | ≥ 200 MB en el intervalo |
| Escritura perceptible | 20 | ≥ 40 MB |
| Ruta temporal o compartida | 24 | `/tmp`, `/private/tmp`, `/var/tmp`, `/Users/Shared`, `~/Downloads` |
| Binario oculto | 20 | nombre con punto inicial |
| Sin firma de código | 26 | `codesign` no encuentra firma |
| Firma ad-hoc | 14 | firma sin autoridad verificable |
| Patrón de instalador | 10 | nombre contiene `install`, `update`, `pkgutil`… |

Resultado: `Healthy` (0-24), `Warning` (25-54), `Critical` (55+).

Los umbrales son configurables en `rootcause-config.json` y en la sección Configuración.

## Heurísticas con memoria

### CPU sostenido

Dispara cuando un proceso supera el umbral (55 % por defecto) durante **3 muestras consecutivas**.
La racha se rompe en cuanto baja: un pico aislado nunca genera evento.

### Crecimiento de memoria

Compara contra la línea base del proceso. Si crece más de 250 MB sobre esa base durante 2 muestras,
reporta. Al reportar, reajusta la base para no repetir el mismo evento indefinidamente. Si la
memoria baja, la base se reajusta hacia abajo.

### Escritura agresiva

120 MB por intervalo durante 2 muestras consecutivas. Es la heurística más cercana a un patrón de
cifrado masivo, pero también la que más disparan los compiladores y los gestores de copias: por eso
el evento apunta a «identifica qué carpeta está creciendo» antes que a «detén el proceso».

### Tráfico saliente inusual

4 o más destinos públicos distintos desde el mismo PID.

**Excepción deliberada:** un binario instalado con normalidad (`/Applications`, `/usr`, `/System`)
y con firma válida está exento. Un navegador hablando con veinte destinos es lo que hace un
navegador. La señal está en que lo haga algo que vive fuera de esas rutas.

### Barrido de la red local

8 o más equipos distintos del segmento contactados desde el mismo PID. Misma excepción que la
anterior.

### Ruta de ejecución sospechosa

Se reporta de inmediato, sin necesidad de racha: en macOS el software se instala en `/Applications`
o `/usr`. Ejecutar desde `/tmp` o desde una carpeta compartida no es normal.

### Binario sin firma fuera del sistema

Un binario sin firma que además no vive en una ruta del sistema. En macOS casi todo viene firmado;
la ausencia de firma significa que nadie se hace responsable de ese código.

### Reapariciones rápidas

Un proceso que muere y renace con PID nuevo 2 veces en menos de 180 segundos.

**Solo se vigilan los nombres con una única instancia viva.** Los nombres multi-instancia
(ayudantes de navegador, workers de Spotlight) van y vienen constantemente por diseño; vigilarlos
convertiría la heurística en una fábrica de ruido.

## Lista de confianza

Un proceso queda exento de las heurísticas si:

- su nombre está en `trusted_process_names` (`kernel_task`, `launchd`, `WindowServer`, `mds`…), **o**
- está **firmado por Apple** y vive en una ruta del sistema (`/System`, `/usr`, `/bin`, `/sbin`,
  `/Applications`, `/Library/Apple`).

Si eso está comprometido, el problema es de otro orden y ninguna heurística de espacio de usuario
lo va a resolver.

## Cambios contra baseline

Estos no son heurísticas: son hechos. Cualquier cambio en una superficie vigilada se reporta,
independientemente de si parece sospechoso, porque la decisión sobre si es legítimo corresponde al
usuario.

| Superficie | Riesgo al cambiar |
|---|---|
| Persistencia | Alto (crítico si la entrada además es sospechosa por sí misma) |
| Controles de seguridad | Alto |
| Permisos TCC sensibles | Alto |
| Equipos de la red | Medio (crítico si cambia la MAC de la puerta de enlace) |

## Cómo ajustar los umbrales

```bash
rootcause config init
open ~/Library/Application\ Support/RootCauseInspector/rootcause-config.json
```

O desde la sección **Configuración** de la interfaz, que los guarda al momento.

Si una heurística te genera ruido en tu equipo concreto, súbele el umbral en vez de desactivar la
detección entera: una detección apagada no avisa de nada.
