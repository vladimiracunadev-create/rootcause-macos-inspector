# Manual de usuario

## Qué es RootCause y qué no es

RootCause es un **sensor forense**: observa tu Mac, detecta distorsiones y cambios respecto a un
estado bueno conocido, y te explica dónde mirar con evidencia.

**No es un antivirus.** No elimina malware, no bloquea por firma y no interviene solo. Si esperas
un botón de «limpiar», este no es el producto. Si quieres entender qué cambió en tu equipo y por
qué, sí lo es.

## La primera vez que lo abres

1. La primera captura tarda unos segundos: se están consultando siete superficies del sistema.
2. **Esa primera captura se guarda como tu «estado bueno conocido»**. No verás cien alertas por
   software que ya tenías instalado.
3. A partir de ahí, cada cambio se reporta hasta que lo aceptes explícitamente.

Si acabas de instalar el Mac o vienes de una limpieza, es el momento ideal para empezar: la
baseline será realmente confiable.

## Cómo leer el semáforo

| Color | Significa |
|---|---|
| 🟢 Verde | No hay señales relevantes en esta captura |
| 🟡 Amarillo | Hay algo que merece tu atención, no necesariamente un problema |
| 🔴 Rojo | Señal crítica: un cambio en una superficie vigilada o una anomalía fuerte |

**Rojo no significa «estás infectado».** Significa «esto no encaja con el estado que aceptaste como
bueno». La causa más frecuente de un rojo legítimo es que instalaste algo nuevo.

## Sección por sección

### Resumen

El veredicto de la captura. Arriba, el semáforo con su motivo. Debajo, tres tendencias (CPU,
memoria, escritura) y cuatro contadores de superficie. Al final, las alertas ordenadas por
severidad y el incidente dominante si lo hay.

Cada alerta trae tres cosas: **qué pasa**, **el detalle** y **qué hacer** (la línea azul con la
flecha).

### Procesos

Qué se está ejecutando. Lo relevante no es el consumo, es la combinación: un proceso al 70 % de CPU
es un compilador; el mismo proceso al 70 %, sin firmar y ejecutándose desde `/tmp`, es otra
conversación.

La etiqueta de **firma** es la información más valiosa de esta tabla:

- `Apple` — software del sistema.
- `Developer ID` — firmado por un desarrollador identificado ante Apple.
- `Ad-hoc` — firmado sin autoridad verificable.
- `Sin firmar` — no se puede atribuir a nadie. En macOS esto es inusual.
- `Desconocida` — no se pudo verificar (no entra en el presupuesto de esta captura).

El botón **Finalizar** envía `SIGTERM`. No escala a `SIGKILL`: si el proceso ignora la señal, eso
también es información. Los procesos que sostienen la sesión gráfica o el arranque están protegidos
y no se pueden finalizar desde aquí.

### Conexiones

Qué proceso habla con el exterior. Dos etiquetas importan:

- **IP PÚBLICA** — el destino está en Internet, no en tu red local.
- **ESCUCHA** — el proceso tiene un puerto abierto. Si aparece en amarillo, está escuchando en
  *todas* las interfaces (`*:puerto`), es decir, es alcanzable desde tu red.

El botón «Regla de bloqueo» **no bloquea nada**: te entrega el comando `pfctl` exacto para que lo
ejecutes tú conscientemente, porque modificar el firewall del equipo no debería ser un efecto
secundario de pulsar un botón en una app.

> Sin privilegios de administrador, `lsof` solo ve los sockets de tu propio usuario. La app te lo
> recuerda en la sección.

### Red

Los equipos vecinos de tu segmento. El escaneo normal es **pasivo**: lee la tabla que tu Mac ya
conoce, es instantáneo y no envía un solo paquete. El **escaneo profundo** hace ping a todo el
segmento para despertar a los que no aparecen: es ruidoso y tarda, por eso nunca se ejecuta solo.

El caso que más importa: si la **puerta de enlace cambia de MAC**, se reporta como crítico. Esa es
la firma clásica de una suplantación ARP.

### Persistencia

**La sección más importante del producto.** Todo lo que se ejecuta al arrancar tu Mac o al iniciar
sesión: LaunchAgents, LaunchDaemons, login items y `cron`.

Cada entrada trae su ámbito, su comando, su firma y su nota explicativa. Las etiquetas de cambio
(NUEVA, MODIFICADA, ELIMINADA) son lo que hay que mirar primero.

El botón **Revelar** abre el Finder con el `.plist` seleccionado. RootCause muestra dónde está;
borrarlo es tu decisión.

Detalle completo → [`PERSISTENCIA_MACOS.md`](PERSISTENCIA_MACOS.md).

### Seguridad

Los controles nativos de macOS, cada uno con **la salida cruda del comando** que lo respondió. Si
dice «Gatekeeper: Activado» es porque `spctl --status` devolvió `assessments enabled`, y ahí lo
tienes escrito.

Un control apagado no prueba una intrusión: hay motivos legítimos para desactivar SIP en una
máquina de desarrollo. Pero es una superficie abierta que merece una decisión consciente.

Debajo, la antigüedad de las definiciones de XProtect. Si tienen meses, lo más probable es que las
actualizaciones automáticas de macOS estén rotas.

### Privacidad

Qué aplicación tiene qué permiso: grabar pantalla, leer el teclado, usar el micrófono, controlar
otras apps o **acceder a todo el disco**.

Para leer esto, RootCause necesita a su vez Acceso total al disco. Si no lo tiene, **te lo dice** —
no te muestra una lista vacía fingiendo que no hay permisos concedidos.

Cómo concederlo → [`PERMISOS_MACOS.md`](PERMISOS_MACOS.md).

### Almacenamiento

Qué ocupa espacio en cachés y temporales. La limpieza tiene tres salvaguardas y se pide dos veces
a propósito:

1. Solo toca `~/Library/Caches` — nunca cachés del sistema ni tu papelera.
2. Solo borra lo que no se ha tocado en 24 horas.
3. Salta lo que esté en uso en vez de forzarlo.

### Historial

Las capturas guardadas, los incidentes persistidos y la **auditoría**: cada acción ejecutada desde
la app o la CLI, con su resultado. Si algo cambió en tu equipo desde RootCause, está ahí.

### Configuración

Tema (oscuro, claro o seguir al sistema), idioma, intervalo de refresco, umbrales de proceso y qué
superficies se vigilan. Los cambios se guardan al momento.

## Rutinas recomendadas

| Cuándo | Qué hacer |
|---|---|
| Al instalar | Abrir, dejar que siembre la baseline, revisar Seguridad y Privacidad |
| Semanal | Abrir, revisar Persistencia y aceptar la baseline si los cambios son tuyos |
| Tras instalar software nuevo | Revisar Persistencia: casi todo instalador deja un LaunchAgent |
| Si el Mac va lento | Resumen → Procesos → Almacenamiento, en ese orden |
| Si sospechas algo | `rootcause report` y guardar el Markdown antes de tocar nada |

## Qué hacer con un hallazgo

1. **No borres nada todavía.** Genera un reporte (`⌘R`): captura la evidencia antes de alterarla.
2. **Busca el nombre.** Un `Label` de LaunchAgent suele identificar al software que lo puso.
3. **Mira la firma.** Si el binario está firmado con Developer ID, hay un desarrollador
   identificable detrás.
4. **Comprueba si lo instalaste tú.** La causa más frecuente de un cambio es la más aburrida.
5. **Si sigue sin cuadrar**, ese es el momento de escalar a una herramienta de eliminación o a
   alguien con más contexto. RootCause te dijo dónde mirar; ese era su trabajo.
