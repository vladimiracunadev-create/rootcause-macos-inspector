# Limitaciones

Escritas sin adornos. Un producto de seguridad que exagera lo que cubre es peor que uno que no
existe, porque genera confianza donde no la hay.

## De diseño

### No elimina nada

RootCause no borra malware, no pone archivos en cuarentena y no revierte cambios. Muestra dónde
mirar y deja evidencia. Si buscas una herramienta de limpieza, esta no lo es.

### Muestrea, no intercepta

El modelo es de captura periódica (5 s por defecto). Algo que ocurre y desaparece entre dos
capturas no se ve. Interceptar en tiempo real exigiría el Endpoint Security Framework de Apple, una
extensión de sistema y un `entitlement` concedido por Apple.

### No reconoce familias de malware

No hay base de firmas ni descarga de definiciones. RootCause detecta **comportamiento y cambios**;
nunca dirá «esto es la variante X del troyano Y».

### No resiste a un atacante con root

Un proceso privilegiado puede finalizar el agente, alterar el SQLite del historial o falsear la
salida de los comandos consultados. Lo que sí queda es rastro: cierres abruptos y cambios de
configuración se auditan. Ver [`../SECURITY.md`](../SECURITY.md).

## De permisos

### Sin Acceso total al disco no hay auditoría de privacidad

`TCC.db` está protegida por macOS. Sin ese permiso, la sección Privacidad no muestra nada — y lo
declara en vez de fingir que no hay permisos concedidos.

### Sin root, `lsof` solo ve tus sockets

Las conexiones de procesos de otros usuarios (incluidos daemons del sistema) quedan fuera.

### Los login items no se consultan solos

Requieren permiso de Automatización, y pedirlo sin que el usuario lo haya solicitado sería
exactamente el comportamiento que una herramienta de seguridad no debe tener.

## De cobertura

### Las carpetas de Apple se omiten por defecto

`/System/Library/Launch*` está protegido por SIP y son cientos de entradas inmutables. Se incluyen
solo con `--all`. Si algo escribe ahí, SIP ya está comprometido y el problema es de otro orden.

### El escaneo de cachés es aproximado

Tope de 40 000 entradas por raíz. En carpetas enormes, la medición es una estimación — y se dice
en la propia sección. Un monitor que tarda un minuto en refrescar deja de usarse.

### Solo se verifica la firma de un puñado de binarios por captura

`codesign` cuesta un proceso por binario. El presupuesto por defecto es de 12, priorizando lo que
ya destaca por severidad y lo que vive fuera de las rutas del sistema. El resto queda como firma
`Desconocida`, que no es lo mismo que «sin firmar».

### Los vecinos ARP no son un mapa de red

El escaneo pasivo solo ve equipos con los que este Mac ya habló. El profundo cubre el `/24` asumido,
no subredes distintas. Y una MAC puede falsificarse: la baseline detecta cambios, no garantiza
identidad.

### `block-ip` no bloquea

Entrega el comando `pfctl` exacto. Modificar el firewall del equipo requiere root y toca la
configuración global; no debería ser el efecto secundario de pulsar un botón.

## De distribución

### No está firmado ni notarizado

Gatekeeper avisará al abrir el `.app` descargado. La vía recomendada es compilarlo tú, lo que
además permite auditar el código.

## Falsos positivos conocidos

| Situación | Por qué ocurre | Qué hacer |
|---|---|---|
| Compilador o gestor de copias marcado por escritura agresiva | Escriben cientos de MB de forma legítima | Subir `io_write_critical_mb` |
| Máquina de desarrollo con SIP desactivado | Es un estado inseguro, aunque deliberado | Aceptar la baseline de seguridad |
| Cambios masivos tras actualizar macOS | Apple modifica plists y definiciones | Aceptar la baseline de persistencia |
| Muchas entradas nuevas tras instalar software | Casi todo instalador deja un LaunchAgent | Verificar y aceptar |

## Falsos negativos conocidos

| Situación | Por qué no se detecta |
|---|---|
| Exfiltración lenta y de bajo volumen | Indistinguible del tráfico normal sin inspección de contenido |
| Cifrado por lotes bajo el umbral | Diseñado precisamente para no superarlo |
| Malware que solo vive en memoria | No deja persistencia, que es la superficie principal |
| Persistencia en firmware o EFI | Fuera del alcance del espacio de usuario |
| Amenazas dentro del navegador | Ocurren en el navegador, no en el sistema |
