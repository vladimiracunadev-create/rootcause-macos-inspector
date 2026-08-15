# Guía sencilla: cómo tomar cosas de otros proyectos

La versión corta y clara de [`COMPARATIVA_OSS.md`](COMPARATIVA_OSS.md): cuándo se puede tomar
código de otro proyecto y cuándo solo la idea.

## El semáforo de licencias

RootCause es **Apache 2.0**. Eso determina qué se puede integrar:

| Licencia del proyecto ajeno | ¿Se puede tomar el código? | Qué implica |
|---|---|---|
| **MIT / BSD / ISC** | 🟢 Sí | Conservar el aviso de copyright original |
| **Apache 2.0** | 🟢 Sí | Conservar el aviso y el `NOTICE` si lo trae |
| **MPL 2.0** | 🟡 Con cuidado | Los archivos que vengan de ahí siguen bajo MPL |
| **LGPL** | 🟡 Solo enlazado dinámico | En un binario estático de Rust, en la práctica no |
| **GPL 2.0 / 3.0** | 🔴 No | Obligaría a relicenciar RootCause entero |
| **AGPL** | 🔴 No | Aún más restrictiva |
| **Licencia propia** | 🔴 No sin permiso | Hay que leerla y, casi siempre, pedir permiso |

**Una idea nunca tiene licencia.** Leer cómo otro resolvió un problema y resolverlo tú a tu manera
es legítimo siempre. Copiar y pegar su código, no.

## Los cinco pasos, caso a caso

1. **Identifica la licencia real.** No la del README: la del archivo `LICENSE` y las cabeceras de
   los archivos concretos. Un repositorio puede tener partes con licencias distintas.
2. **Decide qué necesitas de verdad.** La mayoría de las veces necesitas *el conocimiento* (qué
   rutas vigilar, qué campo del plist importa), no el código.
3. **Si es conocimiento, escríbelo tú.** Con tu estructura, tus nombres y tus tests. Y cita la
   fuente en un comentario: es honesto y ayuda a quien venga después.
4. **Si es código y la licencia lo permite**, cópialo con su aviso de copyright, en un archivo
   identificado, y anótalo en la documentación.
5. **Si es código y la licencia no lo permite**, no lo mires mientras escribes el tuyo. Lee la
   documentación del proyecto, no su implementación.

## Casos concretos de este proyecto

| Fuente | Qué se tomó | Cómo |
|---|---|---|
| osquery (Apache-2.0) | Qué rutas de persistencia existen en macOS | Conocimiento; el código de escaneo es propio |
| Objective-See (licencia propia) | La taxonomía de superficies de persistencia | **Solo la idea**; ni una línea de código |
| Documentación de Apple | Formato de los `.plist` de launchd, esquema de `TCC.db` | Documentación pública |
| LuLu (GPL-3.0) | Cómo explicar una conexión a un usuario no técnico | **Solo la idea**; GPL es incompatible |

## La pregunta que resuelve el 90 % de las dudas

> ¿Necesito **su código**, o necesito **saber lo que ellos ya averiguaron**?

Casi siempre es lo segundo. Y lo segundo siempre se puede.
