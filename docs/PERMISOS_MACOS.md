# Permisos de macOS

RootCause no pide permisos en silencio. Este documento explica cuáles puede necesitar, por qué, y
qué pierdes si no los concedes.

## Resumen

| Permiso | ¿Obligatorio? | Para qué | Si no lo concedes |
|---|---|---|---|
| **Acceso total al disco** | No | Leer `TCC.db` y auditar los permisos de privacidad | La sección Privacidad lo declara; el resto funciona igual |
| **Automatización** | No | Consultar los login items vía System Events | No se listan los login items; LaunchAgents y Daemons sí |
| **Administrador (root)** | No | `lsof` vería los sockets de todos los usuarios | Solo ves los sockets de tu usuario |

**Ninguno es obligatorio.** RootCause arranca y funciona sin conceder nada; simplemente declara qué
no puede ver.

## Acceso total al disco

### Por qué lo necesita

La base de datos de permisos de macOS (`TCC.db`) contiene información sensible: qué app puede
grabar tu pantalla, leer tus pulsaciones de teclado o acceder a todos tus archivos. Apple la
protege exigiendo Acceso total al disco para leerla.

Hay una ironía inevitable aquí: **para auditar quién tiene Acceso total al disco, hay que tener
Acceso total al disco**. RootCause no lo disimula: si no lo tiene, la sección lo dice y explica
cómo concederlo.

### Cómo concederlo

1. Abre **Ajustes del Sistema → Privacidad y seguridad → Acceso total al disco**.
2. Pulsa **+**.
3. Selecciona el binario o la app:
   - si usas el `.app`: `/Applications/RootCause.app`;
   - si usas la CLI desde el Terminal: concédeselo a **Terminal** (o a iTerm), porque el permiso
     se aplica al proceso que lanza el comando.
4. Reinicia RootCause.

### Cómo comprobarlo

```bash
rootcause tcc
```

Si responde con la lista de permisos, lo tiene. Si responde «Permisos TCC no legibles», no.

### Cómo revocarlo

Mismo panel, selecciona la entrada y pulsa **−**. RootCause seguirá funcionando sin ella.

## Automatización

### Para qué se usa

Los login items modernos se consultan a través de System Events, y macOS pide permiso de
Automatización la primera vez.

### Por qué no se pide al arrancar

Una herramienta de seguridad no debería provocar diálogos de permisos que el usuario no pidió. El
escaneo automático **omite** los login items; solo se consultan cuando pulsas «Consultar login
items» en la sección Persistencia, o con:

```bash
rootcause persistence --login-items
```

Es la única acción de todo el producto que dispara un diálogo del sistema, y siempre parte de un
clic tuyo.

## Privilegios de administrador

RootCause **nunca escala privilegios por su cuenta** y no usa `sudo` implícito en ninguna función.

Si lo ejecutas como root (`sudo rootcause status`), `lsof` verá los sockets de todos los usuarios
en vez de solo los tuyos. Nada más cambia. La sección «Acerca» y la salida de `rootcause status`
declaran con qué privilegios se está ejecutando, para que sepas cómo interpretar lo que ves.

## Lo que RootCause nunca pide

- **Extensión de sistema o kernel.** No hay driver ni `.kext`.
- **Acceso a la red entrante.** No abre puertos ni acepta conexiones.
- **Cuenta ni inicio de sesión.** No hay servidor ni servicio asociado.
- **Acceso a contactos, calendario, fotos o ubicación.** No los toca.

## Dónde guarda sus datos

```text
~/Library/Application Support/RootCauseInspector/
├── rootcause-config.json          ← configuración
├── rootcause-history.db           ← historial, incidentes, auditoría y baselines
└── rootcause-agent-state.json     ← estado de salud del agente

~/Documents/RootCause/reports/     ← reportes forenses en Markdown
~/Downloads/                       ← capturas exportadas en JSON
```

Para desinstalar por completo, borra el `.app`, esas carpetas y la entrada de Acceso total al disco.
