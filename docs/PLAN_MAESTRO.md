# Plan maestro

El documento que explica **por qué el producto es como es**. Si algo en el repositorio contradice
esto, lo que está mal es el repositorio.

## 1 · La tesis

> Cualquier distorsión anómala de los recursos o de la configuración de un equipo puede ser el
> primer indicio de que algo está ocurriendo.

De ahí salen tres consecuencias que gobiernan todas las decisiones:

1. **No hace falta saber qué es una amenaza para notar que algo cambió.** Por eso el motor central
   es una baseline, no una base de firmas.
2. **La correlación vale más que cualquier señal aislada.** Por eso el puntaje es acumulativo y
   ninguna condición sola declara un problema.
3. **Explicar vale más que actuar.** Por eso el producto muestra, documenta y deja evidencia, en
   vez de borrar.

## 2 · Los cinco principios

### 2.1 · Diagnóstico primero, intervención después

RootCause no elimina malware, no pone nada en cuarentena y no revierte cambios. Las dos únicas
acciones que modifican el sistema —finalizar un proceso y vaciar cachés del usuario— parten de un
clic explícito, están limitadas por política y quedan auditadas.

### 2.2 · Honestidad sobre los límites

Cada superficie declara qué no puede ver. Si falta Acceso total al disco, la sección de privacidad
**lo dice** en vez de mostrar una lista vacía. Hay un documento entero,
[`LIMITACIONES.md`](LIMITACIONES.md), dedicado a lo que el producto no hace.

Un producto de seguridad que exagera su cobertura es peor que no tenerlo, porque genera confianza
donde no la hay.

### 2.3 · Análisis local

Sin servidor, sin cuenta, sin telemetría, sin comprobación de actualizaciones. Todo vive en
`~/Library/Application Support/RootCauseInspector/`. La única salida a la red posible es el
adaptador de IA opcional, apagado por defecto y limitado al incidente ya resumido.

### 2.4 · Sin permisos silenciosos

Ningún permiso del sistema se pide sin una acción explícita del usuario. Los login items no se
consultan en el escaneo automático precisamente porque hacerlo dispararía un diálogo que nadie
pidió.

### 2.5 · La interfaz no bloquea

Una captura tarda entre décimas de segundo y varios segundos. El motor vive en un hilo propio y se
comunica por canales. Una herramienta que congela la ventana al refrescar deja de usarse, y una
herramienta que no se usa no protege nada.

## 3 · Fases del producto

### Fase 1 — Superficies nativas *(completada, v0.1.0)*

Traducir el producto a macOS de verdad: launchd en vez del registro, Gatekeeper y XProtect en vez
de Defender, TCC en vez de UAC, `lsof` en vez de `netstat`, `codesign` en vez de Authenticode.

### Fase 2 — Profundidad *(v0.2.0)*

Más superficies (extensiones de sistema, integridad de aplicaciones), mejor presentación de la
evidencia (diff de capturas, detalle de incidente) y exportación a formatos que otras herramientas
lean.

### Fase 3 — Correlación entre superficies *(v0.3.0)*

Hoy cada superficie puntúa por separado. Un mismo binario con persistencia nueva **y** un permiso
TCC nuevo **y** tráfico saliente inusual debería puntuar más que la suma de las tres señales por
separado. Ese es el salto cualitativo que queda pendiente.

### Fase 4 — Decisiones abiertas

Endpoint Security Framework, firma y notarización, reglas declarativas. Cada una cambia la
naturaleza del producto y ninguna se adopta sin decidir explícitamente qué se gana y qué se pierde.

## 4 · Lo que nunca va a cambiar

1. No habrá eliminación automática de nada.
2. No habrá telemetría ni servidor.
3. Antes de añadir una detección, se documenta qué no cubre.
4. Ningún permiso se pedirá sin una acción explícita.
5. La licencia seguirá siendo permisiva.

## 5 · Cómo se decide qué entra

Una funcionalidad entra si responde que sí a las cuatro:

1. ¿Responde una pregunta que el usuario ya se hace?
2. ¿Se puede explicar su resultado con evidencia verificable?
3. ¿Se puede declarar honestamente qué no cubre?
4. ¿Se puede probar sin depender del estado concreto de una máquina?

La cuarta es la que más funcionalidades ha descartado, y es la que mantiene la suite de tests con
sentido.
