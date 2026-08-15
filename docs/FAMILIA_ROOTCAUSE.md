# La familia RootCause

Cuatro ediciones, una misma idea, superficies distintas.

## La idea común

> Cualquier distorsión anómala de los recursos o de la configuración puede ser el primer indicio de
> que algo está ocurriendo.

Las cuatro ediciones comparten posicionamiento —**sensor forense y de apoyo a la decisión, no
antivirus**—, arquitectura por capas, análisis local sin telemetría y licencia Apache 2.0.

## Las cuatro ediciones

| Edición | Plataforma | Tecnología | Repositorio |
|---|---|---|---|
| **Windows Inspector** | Windows 10/11 | Rust + egui | [rootcause-windows-inspector](https://github.com/vladimiracunadev-create/rootcause-windows-inspector) |
| **macOS Inspector** | macOS 13+ | Rust + egui | [rootcause-macos-inspector](https://github.com/vladimiracunadev-create/rootcause-macos-inspector) |
| **Web Inspector** | Navegador | Extensión MV3 + Node | [rootcause-web-inspector](https://github.com/vladimiracunadev-create/rootcause-web-inspector) |
| **Mobile Inspector** | Android / iOS | Flutter | [rootcause-mobile-inspector](https://github.com/vladimiracunadev-create/rootcause-mobile-inspector) |

## Qué observa cada una

| Superficie | Windows | macOS | Web | Mobile |
|---|---|---|---|---|
| Persistencia | Registro Run, servicios, tareas | LaunchAgents/Daemons, login items, `cron` | Extensiones del navegador | Apps con inicio automático |
| Defensas del sistema | Defender, firewall | Gatekeeper, SIP, FileVault, firewall, XProtect | Permisos de sitio | Permisos de app |
| Privacidad | — | TCC | Cookies y sesiones | Permisos concedidos vs pedidos |
| Procesos | Consumo + Authenticode | Consumo + `codesign` | — | Consumo por app |
| Red | `netstat` + vecinos ARP | `lsof` + vecinos ARP | Descargas | — |
| Evidencia | JSON, SQLite, ETL | JSON, SQLite, Markdown | Panel local | PDF |

## Por qué no son el mismo código

Traducir el producto entre plataformas sin traducir las superficies produciría una herramienta que
compila en todas partes y no dice nada útil en ninguna. En macOS la persistencia son archivos
`.plist`, no claves de registro; las defensas son Gatekeeper y XProtect, no Defender; los permisos
son TCC, no UAC.

Lo que sí se comparte entre las ediciones nativas:

- El **motor de baseline**: primera foto en silencio, cambios pegajosos hasta aceptarlos.
- El modelo de **incidente** con severidad, evidencia, hipótesis y acción recomendada.
- La estructura de **documentación** y el criterio de honestidad sobre los límites.
- La disciplina de entrega: CI con lint sin tolerancia, tests y release reproducible.

## Cuál usar

- **Un equipo Windows o Mac que va raro o del que sospechas** → la edición nativa correspondiente.
- **Sospechas centradas en el navegador** (extensión rara, sesión ajena, cookies) → Web Inspector,
  que ve una superficie que las ediciones de escritorio no alcanzan.
- **Un teléfono con consumo raro o sospecha de stalkerware** → Mobile Inspector.

Se complementan: usar dos a la vez sobre el mismo problema no duplica trabajo, cubre superficies
distintas.

## Detalle de esta edición

Ver [`CATALOGO_PRODUCTO.md`](CATALOGO_PRODUCTO.md) y [`ARCHITECTURE.md`](ARCHITECTURE.md).
