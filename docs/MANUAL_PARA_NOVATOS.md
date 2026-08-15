# Manual para novatos

Sin jerga. Si algo aquí no se entiende, es un fallo del texto, no tuyo.

## ¿Qué hace este programa?

Mira tu Mac y te avisa cuando **algo cambia sin que tú lo hayas cambiado**.

La primera vez que lo abres, saca una foto de cómo está tu equipo. A partir de ahí compara: si
mañana aparece un programa nuevo que arranca solo, o si una protección de macOS se apaga, te lo
dice.

## ¿Es un antivirus?

No. Un antivirus intenta **borrar** cosas malas. Esto solo **mira y avisa**. Puedes usar los dos a
la vez; no se estorban.

## ¿Es peligroso instalarlo?

No borra ni cambia nada de tu Mac por su cuenta. Las dos únicas cosas que puede modificar son:

- cerrar un programa, si tú pulsas el botón «Finalizar»;
- vaciar la carpeta de archivos temporales, si tú lo pides **dos veces**.

Todo lo demás es mirar.

## ¿Qué significan los colores?

- 🟢 **Verde**: todo normal.
- 🟡 **Amarillo**: algo que conviene que mires, sin urgencia.
- 🔴 **Rojo**: algo cambió respecto a la foto inicial.

**Rojo no significa «tienes un virus».** Casi siempre significa «instalaste algo nuevo». Si
instalaste una app ayer y hoy sale rojo, es normal: pulsa «Aceptar baseline» y esa será la nueva
foto de referencia.

## Las palabras raras, traducidas

| Palabra | Qué significa de verdad |
|---|---|
| **LaunchAgent / LaunchDaemon** | Un programa que arranca solo cuando enciendes el Mac o entras a tu cuenta |
| **Persistencia** | Que un programa sobreviva a apagar y encender el equipo |
| **Baseline** | La foto de referencia: cómo estaba tu Mac cuando dijiste «así está bien» |
| **Gatekeeper** | El portero de macOS: comprueba que un programa descargado sea de fiar |
| **SIP** | Un candado que impide tocar los archivos del sistema, incluso con contraseña de administrador |
| **FileVault** | El cifrado del disco: sin tu contraseña, nadie puede leerlo |
| **XProtect** | El antivirus que macOS trae de serie, invisible |
| **TCC / Permisos** | Quién tiene permiso para usar tu cámara, micrófono, pantalla o archivos |
| **Firma de código** | La etiqueta que dice quién hizo un programa. Sin firma = nadie se hace responsable |
| **Puerto a la escucha** | Una puerta abierta en tu Mac por la que otro ordenador podría entrar |
| **IP pública** | Una dirección de Internet, fuera de tu casa u oficina |

## ¿Qué miro primero?

1. **Resumen.** Si está verde, ya está: cierra tranquilo.
2. Si hay algo en rojo o amarillo, lee la línea con la flecha azul: ahí dice qué hacer.
3. Si la alerta habla de **Persistencia**, ve a esa sección y mira el nombre. ¿Reconoces el
   programa? ¿Lo instalaste tú la semana pasada?

## ¿Me va a pedir permisos raros?

Dos, y solo si tú los concedes:

- **Acceso total al disco**: para poder leer la lista de permisos de tus aplicaciones. Sin esto,
  la sección de Privacidad te dirá que le falta el permiso, y el resto seguirá funcionando.
- **Automatización**: solo si pulsas el botón «Consultar login items».

Nunca los pide en segundo plano ni sin avisar.

## ¿Manda mis datos a algún sitio?

No. Todo se queda en tu Mac, en una carpeta tuya. No hay servidor, no hay cuenta, no hay
estadísticas. La única excepción es una función de inteligencia artificial que viene **apagada** y
que tendrías que configurar tú a propósito.

## ¿Y si veo algo que no entiendo?

Genera un reporte (menú de arriba, botón «Reporte»). Se guarda un archivo de texto en tu carpeta
de Documentos con todo lo que se vio. Ese archivo es lo que le puedes enseñar a alguien que sepa
más, sin tener que explicarle nada.
