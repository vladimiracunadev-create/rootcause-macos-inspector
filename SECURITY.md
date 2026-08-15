# Política de seguridad

## Versiones soportadas

| Versión | Soportada |
|---|---|
| 0.1.x | ✅ |

## Reportar una vulnerabilidad

Si encuentras un fallo de seguridad en RootCause macOS Inspector, **no abras un issue público**.

Escribe a [@vladimiracunadev-create](https://github.com/vladimiracunadev-create) mediante un
[security advisory privado](https://github.com/vladimiracunadev-create/rootcause-macos-inspector/security/advisories/new)
con:

- descripción del problema y su impacto,
- pasos para reproducirlo,
- versión de RootCause y de macOS,
- cualquier registro o captura que ayude.

Compromiso de respuesta: acuse de recibo en 72 horas y una evaluación inicial en 7 días.

## Modelo de amenaza de la propia herramienta

RootCause corre en el espacio de usuario y es honesto sobre lo que eso implica:

- **No resiste a un atacante con root.** Un proceso privilegiado puede finalizar el agente,
  alterar el SQLite del historial o falsear la salida de los comandos que RootCause consulta.
- **Su baseline es un archivo local.** Quien pueda escribir en
  `~/Library/Application Support/RootCauseInspector/` puede manipular el "estado bueno conocido".
- **No verifica criptográficamente su configuración.** La huella de integridad detecta cambios
  accidentales entre sesiones, no falsificaciones deliberadas.

Lo que sí hace: dejar constancia. Cierres abruptos, cambios de configuración y cada acción
ejecutada quedan en la auditoría local, de modo que una manipulación deja rastro aunque no se pueda
impedir.

## Superficie de red

RootCause **no abre puertos ni acepta conexiones entrantes**. La única salida a la red posible es
el adaptador de IA opcional, apagado por defecto. El escaneo profundo de red envía pings al
segmento local y solo se ejecuta cuando el usuario lo pide.

## Dependencias

Las dependencias se mantienen deliberadamente pocas y auditables. Revisa `Cargo.toml` y
`Cargo.lock`; la CI compila con `clippy -D warnings` en cada cambio.
