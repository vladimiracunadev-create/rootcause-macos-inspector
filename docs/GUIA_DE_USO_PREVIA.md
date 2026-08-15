# Guía de uso previa

Lee esto **antes** de la primera ejecución. Son cinco minutos que evitan las tres confusiones más
frecuentes.

## 1 · Qué vas a ver, y qué no

RootCause no te va a decir «tienes un virus». Te va a decir **qué cambió en tu Mac** desde que lo
miraste por primera vez, y te va a dar la evidencia para que tú decidas.

Si buscas un botón de «limpiar y arreglar», este producto no lo tiene y no lo va a tener.

## 2 · La primera ejecución define tu referencia

La primera captura se guarda **en silencio** como tu «estado bueno conocido». Todo lo que ya
tuvieras instalado —Docker, Chrome, actualizadores— pasa a considerarse normal.

Consecuencia práctica:

| Situación | Qué hacer |
|---|---|
| Mac recién instalado o recién limpiado | **Ejecuta ahora.** La baseline será realmente confiable |
| Mac que llevas años usando | Ejecuta igual. Detectará todo lo que aparezca **de aquí en adelante**, que es donde está el valor |
| Sospechas que ya hay algo raro | Ejecuta y revisa Persistencia y Privacidad **antes** de aceptar la baseline |

## 3 · Los permisos, antes de que te sorprendan

| Permiso | ¿Cuándo se pide? | Si no lo das |
|---|---|---|
| **Acceso total al disco** | Nunca automáticamente. Lo concedes tú en Ajustes | La sección Privacidad lo dirá; todo lo demás funciona |
| **Automatización** | Solo si pulsas «Consultar login items» | No se listan los login items |

Ninguno es obligatorio. Y ninguno se pide en segundo plano.

Cómo concederlos → [`PERMISOS_MACOS.md`](PERMISOS_MACOS.md).

## 4 · Lo que la app puede modificar en tu equipo

Exactamente dos cosas, y las dos parten de un clic tuyo:

1. **Finalizar un proceso** — envía `SIGTERM`, no fuerza. Los procesos que sostienen el sistema
   están protegidos y no se pueden finalizar desde aquí.
2. **Vaciar cachés** — solo `~/Library/Caches`, solo lo no usado en 24 h, saltando lo que esté en
   uso, y **se pide dos veces**.

Todo lo demás es mirar. Cada acción queda registrada en la auditoría.

## 5 · Lo primero que deberías revisar

En este orden:

1. **Seguridad** — ¿Gatekeeper, SIP y FileVault están como esperas? Si algo está apagado, decídelo
   ahora conscientemente.
2. **Privacidad** — ¿Reconoces todas las apps con permisos sensibles? Es la sección que más
   sorpresas da la primera vez.
3. **Persistencia** — ¿Reconoces lo que arranca con tu Mac? Casi todo tendrá una explicación
   aburrida (un instalador), y eso está bien.

## 6 · Qué hacer cuando algo salga en rojo

1. **No borres nada todavía.** Pulsa «Reporte» (`⌘R`): guarda la evidencia antes de alterarla.
2. **Pregúntate qué instalaste esta semana.** Es la respuesta el 90 % de las veces.
3. **Mira la firma del binario.** Developer ID significa que hay alguien identificable detrás.
4. **Si sigue sin cuadrar**, ese es el momento de llevarlo a alguien con más contexto — con el
   reporte en la mano, que es exactamente para eso.

## 7 · Expectativas realistas

| Esperas | Realidad |
|---|---|
| Que detecte cualquier malware | Detecta **cambios y comportamiento**, no familias de malware |
| Que proteja en tiempo real | Muestrea cada pocos segundos; no intercepta |
| Que funcione sin permisos | Funciona, pero declara lo que no puede ver |
| Que sustituya a un antivirus | Lo **complementa**. Son cosas distintas |
| Que el rojo signifique infección | Significa «esto cambió respecto a lo que aceptaste» |

## 8 · Antes de compartir un reporte

Un reporte contiene el nombre de tu equipo, rutas con tu nombre de usuario, aplicaciones instaladas
y direcciones IP con las que hablaste. **Revísalo antes de adjuntarlo** a un ticket o a un issue
público.
