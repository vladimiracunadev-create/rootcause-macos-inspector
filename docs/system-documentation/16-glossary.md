# 16 · Glosario

> Términos técnicos, siglas y palabras propias del producto, explicados para que los entienda
> alguien que no trabaja en seguridad informática. Ordenados alfabéticamente dentro de cada
> bloque.

---

## 1. Términos del producto

**Agente**
: Aquí no significa «programa que se instala como servicio». En RootCause designa al propio
  ejecutable mientras está corriendo. Su «salud» es lo que vigila el módulo de resiliencia:
  si la sesión anterior cerró bien, si la configuración cambió, si hubo reinicios repetidos.

**Alerta**
: Un hallazgo concreto de una captura, con severidad, título, detalle y una pista de qué
  hacer. Las alertas se ordenan por severidad y se recortan a las más importantes (ocho por
  defecto).

**Baseline** *(estado bueno conocido)*
: La foto guardada de cómo estaba una superficie en un momento que se dio por bueno. Todo lo
  que se compara, se compara contra ella. Si aceptas una baseline, le estás diciendo al
  producto «esto que ves ahora es lo normal».

**Cambio pegajoso**
: Un cambio detectado se sigue reportando en todas las capturas siguientes hasta que alguien
  lo acepta explícitamente. Es deliberado: una alerta que se apaga sola tras reiniciar es
  peor que no tenerla.

**Captura** *(snapshot)*
: Todo lo que el producto observa en un instante: procesos, conexiones, controles, permisos,
  persistencia, red y almacenamiento. Es la unidad de trabajo: se muestra, se exporta, se
  guarda y se compara.

**Evidencia**
: El dato crudo que respalda una afirmación. Si el producto dice «Gatekeeper está
  desactivado», la evidencia es el texto exacto que respondió el comando `spctl --status`.
  El principio del producto es no mostrar nunca un veredicto sin su evidencia.

**Heurística**
: Una regla práctica que sugiere que algo merece atención, sin demostrarlo. «Este proceso
  lleva tres muestras seguidas por encima del 55 % de CPU» es una heurística: puede ser un
  compilador o puede ser un minero de criptomonedas.

**Incidente**
: El resumen correlacionado de lo que pasa en una captura, con hipótesis de causa raíz,
  causas probables, acciones sugeridas y evidencia. No toda captura genera uno: si el equipo
  está sano, no hay incidente.

**Puntaje acumulativo**
: Forma de decidir del producto: ninguna señal por sí sola declara un problema; cada una suma
  puntos y el veredicto sale de la suma. Un proceso al 70 % de CPU es un compilador; el mismo
  proceso al 70 %, sin firmar y ejecutándose desde una carpeta temporal, ya es otra cosa.

**Semáforo**
: Los tres estados visuales: verde (normal), amarillo (atención), rojo (crítico). El veredicto
  global toma el color de la señal más grave presente.

**Sensor forense**
: Cómo se define el producto a sí mismo: observa, registra y explica, pero no interviene por
  su cuenta. No es un antivirus (no elimina) ni un EDR (no bloquea).

**Superficie vigilada**
: Cada una de las siete áreas que el producto observa: persistencia, procesos, controles de
  seguridad, antimalware, privacidad, red y almacenamiento.

**Veredicto**
: La conclusión de una captura en una frase y un color. Se calcula a partir de la alerta más
  grave.

## 2. Conceptos de macOS

**Acceso total al disco** *(Full Disk Access)*
: Permiso que concede el usuario en Ajustes del Sistema y que permite a una aplicación leer
  archivos protegidos, incluidos correo, mensajes y las bases de datos de permisos. RootCause
  lo necesita para auditar la privacidad, y sin él lo dice en vez de mostrar una lista vacía.

**Automatización** *(Apple Events)*
: Permiso que permite a una aplicación controlar otra. RootCause solo lo usa para preguntar a
  System Events qué elementos de inicio de sesión hay, y solo cuando el usuario lo pide.

**Bundle**
: Una carpeta que macOS trata como si fuera un archivo único. Las aplicaciones (`.app`) son
  bundles: dentro tienen el binario, el icono y un archivo de identidad (`Info.plist`).

**codesign**
: Herramienta de macOS que dice quién firmó un programa. En macOS casi todo el software viene
  firmado; un binario sin firma ejecutándose desde una carpeta de usuario es una de las
  señales más fuertes que existen.

**cron**
: Sistema clásico de tareas programadas, heredado de Unix. Apple empuja hacia launchd, pero
  `cron` sigue funcionando, y por eso sigue siendo un sitio cómodo donde esconder una tarea
  recurrente.

**FileVault**
: Cifrado del disco de arranque. Con FileVault activo, alguien con acceso físico al equipo no
  puede leer el disco sin la contraseña.

**Firewall de aplicaciones**
: El cortafuegos de macOS, que filtra conexiones **entrantes** por aplicación. No filtra las
  salientes.

**Gatekeeper**
: El control que verifica la firma y la notarización del software descargado antes de
  ejecutarlo por primera vez. Con Gatekeeper apagado, macOS ejecuta lo que sea sin comprobar
  su origen.

**launchd**
: El proceso que arranca y mantiene vivo todo lo demás en macOS. Es el equivalente a los
  servicios de Windows. Cualquier programa que quiera ejecutarse al arrancar suele registrarse
  aquí.

**LaunchAgent**
: Un archivo que le dice a launchd «ejecuta esto cuando el usuario inicie sesión». Vive en
  `~/Library/LaunchAgents` (solo ese usuario) o en `/Library/LaunchAgents` (todos).

**LaunchDaemon**
: Como un LaunchAgent, pero se ejecuta **como root al arrancar el equipo**, antes de que nadie
  inicie sesión. Por eso pesa más en el cálculo de riesgo.

**Login item**
: Elemento de inicio de sesión: una aplicación que se abre sola al entrar en la cuenta. Se ven
  en Ajustes del Sistema.

**MRT** *(Malware Removal Tool)*
: Herramienta de Apple que elimina malware conocido. RootCause solo comprueba su versión y su
  antigüedad.

**Notarización**
: Proceso por el que Apple revisa un programa y le da un sello. Los binarios de RootCause
  **no están notarizados**, y por eso macOS pide confirmación la primera vez.

**plist** *(property list)*
: Archivo de configuración de macOS, normalmente en XML o binario. Los LaunchAgents y
  LaunchDaemons son plists.

**SIP** *(System Integrity Protection)*
: Protección que impide modificar archivos y procesos del sistema **incluso siendo root**.
  Solo se desactiva arrancando en modo recuperación, así que si está apagado fue una acción
  deliberada de alguien.

**spctl, csrutil, fdesetup, socketfilterfw**
: Los comandos con los que se consultan, respectivamente, Gatekeeper, SIP, FileVault y el
  firewall. RootCause muestra su salida literal como evidencia.

**TCC** *(Transparency, Consent and Control)*
: El sistema de permisos de privacidad de macOS y la base de datos donde se anotan. Es lo que
  responde a «¿esta app puede usar el micrófono?». Su archivo se llama `TCC.db` y hace falta
  Acceso total al disco para leerlo.

**XProtect**
: El antimalware básico que macOS trae de serie, basado en firmas que Apple actualiza en
  silencio. Si sus definiciones tienen meses, casi siempre significa que las actualizaciones
  automáticas están rotas, y eso ya es un hallazgo.

## 3. Conceptos de red

**ARP** *(Address Resolution Protocol)*
: El mecanismo que traduce una dirección IP en la dirección física (MAC) del equipo que la
  tiene. Cada equipo mantiene una tabla con los vecinos que ha visto, y de ahí saca RootCause
  la lista de dispositivos cercanos.

**Barrido de descubrimiento**
: Enviar un ping a todas las direcciones de la red local para que los equipos respondan y
  aparezcan en la tabla ARP. Es ruidoso, por eso RootCause solo lo hace si se lo pides.

**CGNAT** *(Carrier-Grade NAT)*
: Rango de direcciones (`100.64.0.0/10`) que usan los operadores de telecomunicaciones. No es
  Internet abierta, y RootCause lo excluye para no marcar como «pública» una conexión que no
  lo es.

**IP pública / privada**
: Una IP privada (`192.168.x.x`, `10.x.x.x`…) solo existe dentro de una red local. Una pública
  es alcanzable desde Internet. Que un programa hable con muchas IP públicas distintas es
  normal en un navegador y llamativo en un binario suelto.

**lsof**
: Herramienta que lista los archivos y sockets abiertos por cada proceso. Es la que responde
  «¿qué programa tiene una conexión abierta y hacia dónde?».

**MAC**
: Identificador físico de una tarjeta de red. Se supone único por dispositivo, aunque se puede
  falsificar.

**Puerta de enlace** *(gateway)*
: El equipo por el que sale todo el tráfico hacia fuera: normalmente, el router. Si su MAC
  cambia de un día para otro sin que hayas cambiado de router, es la señal clásica de una
  suplantación.

**Puerto a la escucha**
: Un programa esperando conexiones entrantes. Si escucha en `127.0.0.1`, solo lo alcanza el
  propio equipo; si escucha en `0.0.0.0` o `*`, lo alcanza cualquiera de la red.

**Suplantación ARP** *(ARP spoofing)*
: Ataque en el que alguien se hace pasar por el router para que el tráfico pase por su equipo.
  Por eso un cambio de MAC de la puerta de enlace se marca como crítico.

## 4. Conceptos de programación

**`async` / hilo de trabajo**
: RootCause no usa programación asíncrona: usa un **hilo** aparte para el motor. Un hilo es
  una línea de ejecución que corre en paralelo; así, mientras el motor recoge datos, la
  ventana sigue respondiendo.

**Canal** *(channel)*
: Un tubo por el que dos hilos se pasan mensajes. La interfaz manda órdenes por un canal y
  recibe resultados por otro.

**Cargo**
: El gestor de paquetes y de compilación de Rust. `cargo build`, `cargo test`, `cargo clippy`.

**Clippy**
: El analizador de código de Rust. En este proyecto se ejecuta con `-D warnings`, lo que
  significa que **cualquier advertencia hace fallar la compilación**.

**Feature** *(característica de compilación)*
: Una parte del código que se incluye o no al compilar. Aquí, `gui` decide si el binario lleva
  la interfaz gráfica.

**Función pura**
: Una función que, con las mismas entradas, siempre devuelve lo mismo y no toca nada externo.
  Casi todo lo que decide en RootCause es puro, y por eso se puede probar sin un Mac real
  detrás.

**Rust**
: El lenguaje en el que está escrito el producto. Compila a código nativo y garantiza en
  tiempo de compilación que no hay errores de memoria, lo que importa en una herramienta de
  seguridad.

**Serde**
: La biblioteca que convierte estructuras de Rust en JSON y viceversa. Es lo que hace que la
  captura se pueda exportar tal cual.

**SQLite**
: Base de datos que vive en un solo archivo, sin servidor. RootCause la usa para su historial
  y macOS la usa para sus permisos.

**Test unitario**
: Un pequeño programa que comprueba que una función concreta hace lo que debe. Este proyecto
  tiene 112.

## 5. Estados y valores del producto

**`Severity`** — el semáforo: `Healthy` (verde), `Warning` (amarillo), `Critical` (rojo).

**`RiskLevel`** — la escala del motor de anomalías: `Low`, `Medium`, `High`, `Critical`. Se
traduce al semáforo: alto y crítico comparten el rojo.

**`PersistenceChange`** — el estado de un elemento respecto a la baseline:

| Valor | Etiqueta | Significa |
|---|---|---|
| `Unchanged` | *(vacío)* | Igual que en la baseline |
| `Added` | NUEVA | No estaba antes |
| `Modified` | MODIFICADA | Estaba, pero cambió |
| `Removed` | ELIMINADA | Estaba y ya no aparece |

**`CodeSignature`** — quién firmó un binario: `Apple`, `DeveloperId`, `AdHoc` (firma sin
autoridad verificable), `Unsigned` (sin firma) y `Unknown` (no se pudo determinar; **nunca se
trata como confiable**).

**`AgentStatus`** — la salud del propio producto: `Healthy`, `Recovered` (arrancó tras un
cierre abrupto) y `Degraded` (reinicios repetidos o configuración cambiada).

**`PersistenceScope`** — de dónde sale una entrada de arranque automático: agente de usuario,
agente global, daemon de root, sistema de Apple, login item, `cron` u otro.

## 6. Siglas rápidas

| Sigla | Significado |
|---|---|
| ARP | Address Resolution Protocol |
| CGNAT | Carrier-Grade NAT |
| CI | Integración continua |
| CLI | Interfaz de línea de comandos |
| DNS | Sistema de nombres de dominio |
| EDR | Endpoint Detection and Response |
| GUI | Interfaz gráfica de usuario |
| IDS | Sistema de detección de intrusiones |
| MAC | Media Access Control (dirección física) |
| MRT | Malware Removal Tool |
| OUI | Organizationally Unique Identifier (prefijo de fabricante en una MAC) |
| PID | Identificador de proceso |
| RFC 3339 | Formato estándar de fecha y hora |
| SIEM | Gestión de eventos e información de seguridad |
| SIP | System Integrity Protection |
| SSH | Secure Shell (acceso remoto) |
| TCC | Transparency, Consent and Control |
| UID | Identificador de usuario |

---

**Siguiente lectura recomendada:** [17 · Resumen ejecutivo](17-executive-summary.md).
