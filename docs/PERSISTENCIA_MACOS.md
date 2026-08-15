# Persistencia en macOS

## Por qué esta sección existe

En Windows, la persistencia vive sobre todo en el registro. En macOS vive en **archivos**: un
puñado de carpetas con `.plist` que `launchd` lee al arrancar el equipo o al iniciar sesión. Un
implante que quiere sobrevivir a un reinicio casi siempre deja rastro en una de ellas.

## Las cinco carpetas

| Carpeta | Quién la ejecuta | Cuándo | ¿Se vigila por defecto? |
|---|---|---|---|
| `~/Library/LaunchAgents` | El usuario | Al iniciar sesión | ✅ |
| `/Library/LaunchAgents` | Cualquier usuario | Al iniciar sesión | ✅ |
| `/Library/LaunchDaemons` | **root** | Al arrancar el equipo | ✅ |
| `/System/Library/LaunchAgents` | Apple | Al iniciar sesión | Solo con `--all` |
| `/System/Library/LaunchDaemons` | Apple | Al arrancar | Solo con `--all` |

Las de Apple están protegidas por SIP y son cientos de entradas inmutables: incluirlas por defecto
llenaría la baseline de ruido sin aportar señal.

Además se vigilan:

- **`crontab -l`** del usuario, que sigue vivo en macOS aunque Apple empuje launchd.
- **Login items**, bajo demanda (requieren permiso de Automatización).

## Qué se lee de cada `.plist`

| Clave | Para qué |
|---|---|
| `Label` | Identidad del job; si imita a `com.apple.*` fuera de las carpetas de Apple, es señal |
| `Program` / `ProgramArguments` | El comando efectivo y el binario destino |
| `RunAtLoad` | Se ejecuta al cargar |
| `KeepAlive` | Se relanza si muere — típico de implantes y de agentes legítimos por igual |
| `StartInterval` | Cada cuánto se repite; intervalos muy cortos llaman la atención |

## Cómo se calcula el riesgo

El puntaje parte del **ámbito** (un daemon de root pesa más que un agente de usuario) y suma
señales acumulativas. Ninguna señal por sí sola declara un problema:

| Señal | Puntos | Por qué |
|---|---|---|
| Ámbito `LaunchDaemon` (root) | 26 | Se ejecuta como root al arrancar |
| Ámbito `LaunchAgent` global | 20 | Afecta a todos los usuarios |
| Ámbito `LaunchAgent` de usuario | 16 | Solo a tu sesión |
| `Label` imita a `com.apple.*` fuera del sistema | 35 | Suplantación deliberada de identidad |
| Binario en ruta temporal o compartida | 30 | Nadie instala software en `/tmp` |
| Binario con nombre oculto (punto inicial) | 25 | Ocultarse no es un requisito funcional |
| Binario sin firma de código | 30 | No se puede atribuir a ningún desarrollador |
| Firma ad-hoc | 20 | Firma sin autoridad verificable |
| Ejecuta un intérprete o `curl` | 18 | El comando real está en otro sitio |
| `KeepAlive` activo | 10 | Resiste a que lo cierres |
| `StartInterval` ≤ 60 s | 12 | Frecuencia inusual para una tarea de mantenimiento |
| Apunta a un binario inexistente | 12 | Resto de una desinstalación… o de una limpieza |

Resultado: `Bajo` (0-24), `Medio` (25-54), `Alto` (55-84), `Crítico` (85+).

## Detección de cambios

La primera captura siembra la baseline en silencio. Después, cada entrada se clasifica:

- **NUEVA** — no estaba en la baseline.
- **MODIFICADA** — misma ubicación y `Label`, distinto comando. (La clave estable ignora el
  comando a propósito: cambiarlo es una modificación, no una entrada nueva más una eliminada.)
- **ELIMINADA** — estaba y ya no aparece.

Una entrada **nueva** que además es sospechosa por sí misma escala a **crítica**: dos señales
independientes apuntando al mismo sitio.

## Qué hace RootCause con un hallazgo

**Lo muestra. No lo borra.** La única acción que ofrece sobre una persistencia sospechosa es
revelarla en el Finder. Descargar un plist de launchd o eliminar un binario es una decisión con
consecuencias que corresponde tomar a una persona, con contexto, no a un heurístico.

Para revisarla manualmente:

```bash
# Ver el plist completo
plutil -p /Library/LaunchDaemons/sospechoso.plist

# Ver la firma del binario destino
codesign -dvv /ruta/al/binario

# Ver si está cargado ahora mismo
launchctl list | grep sospechoso
```
