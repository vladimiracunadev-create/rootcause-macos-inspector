# Marca y branding

Identidad visual y verbal de RootCause macOS Inspector, y por qué es la que es.

## 1 · La promesa

> **Diagnóstico primero. Intervención después.**

Todo el branding sale de ahí. RootCause no promete limpiar, acelerar ni proteger: promete
**explicar**. Cualquier pieza de comunicación que sugiera lo contrario está mal.

## 2 · El nombre

**RootCause** — «causa raíz». Comunica tres cosas simultáneamente:

- que busca **el origen**, no el síntoma;
- que es una herramienta de **diagnóstico**, vocabulario de ingeniería, no de marketing;
- que **explica**, porque una causa raíz solo sirve si se entiende.

Cada edición añade su plataforma: *RootCause macOS Inspector*, *RootCause Windows Inspector*,
*RootCause Web Inspector*, *RootCause Mobile*.

Las reservas sobre el nombre como marca registrable están en
[`NOMBRES_PRODUCTO.md`](NOMBRES_PRODUCTO.md).

## 3 · El símbolo

Un **radar**: dos círculos concéntricos y un punto central.

```text
    ╭─────────╮
   ╱    ╭─╮    ╲
  │    │ ● │    │
   ╲    ╰─╯    ╱
    ╰─────────╯
```

Por qué un radar y no un escudo, un candado o una lupa:

| Símbolo | Por qué se descartó |
|---|---|
| Escudo | Promete protección. RootCause observa, no protege |
| Candado | Promete bloqueo. No bloquea nada |
| Lupa | Sugiere buscar algo concreto que ya sabes que existe |
| **Radar** | **Detecta lo que aparece sin saber de antemano qué es.** Exactamente la tesis del producto |

El icono se **dibuja por código**, no se carga de un archivo: el mismo radar aparece en la barra
lateral, en el icono del `.app`, en el favicon de la landing y en la esquina de la sección Acerca.
Una sola definición, sin activos que se desincronicen.

## 4 · Paleta

| Color | Hex | Uso |
|---|---|---|
| Azul de marca | `#1f6feb` | Acento, radar, enlaces, elemento activo |
| Fondo oscuro | `#0d1117` | Fondo principal del tema oscuro |
| Panel | `#12171f` | Barra lateral y barras superior/inferior |
| Tarjeta | `#181e28` | Contenedores de contenido |
| Verde | `#3fb950` | Estado sano |
| Ámbar | `#e2a82c` | Atención |
| Rojo | `#e95454` | Crítico |

Reglas de uso del semáforo, que valen más que la paleta:

1. **Verde solo cuando se sabe.** Un estado indeterminado va en ámbar, nunca en verde. «No lo sé»
   no es «está bien».
2. **El rojo se gana.** Se reserva para cambios en superficies vigiladas y anomalías de riesgo
   alto. Si todo es rojo, nada lo es.
3. **El color nunca va solo.** Cada punto de color lleva texto al lado y muestra su etiqueta al
   pasar el ratón. Nadie debería depender de distinguir un ámbar de un verde.

## 5 · Tono de voz

**Preciso, directo y honesto sobre los límites.** Sin alarmismo y sin marketing.

| Se dice | No se dice |
|---|---|
| «Gatekeeper está desactivado: macOS ejecutará binarios descargados sin comprobar firma» | «¡Tu Mac está en peligro!» |
| «Sin Acceso total al disco no se pueden leer los permisos TCC» | *(mostrar una lista vacía)* |
| «El escaneo mide raíces conocidas, no indexa el disco completo» | «Análisis completo del sistema» |
| «Complementa a tu antivirus; no lo sustituye» | «Protección total» |

La regla operativa: **antes de añadir una afirmación, escribir qué no cubre.** Si eso no se puede
escribir con honestidad, la afirmación sobra.

## 6 · Nomenclatura

| Elemento | Convención |
|---|---|
| Nombre completo | RootCause macOS Inspector |
| Nombre corto | RootCause |
| Binario | `rootcause` (minúsculas) |
| Bundle | `RootCause.app` |
| Bundle ID | `dev.vladimiracuna.rootcause` |
| Carpeta de datos | `RootCauseInspector` |
| Etiquetas de versión | `v0.1.0` |

## 7 · Coherencia con las ediciones hermanas

Las cuatro comparten radar, paleta, promesa y tono. Cambia la plataforma y, con ella, las
superficies. Un usuario que conoce una reconoce las demás sin manual: ver
[`FAMILIA_ROOTCAUSE.md`](FAMILIA_ROOTCAUSE.md).
