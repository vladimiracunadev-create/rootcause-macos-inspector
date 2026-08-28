# 14 · Solución de problemas

> Guía práctica: síntoma → causa posible → cómo diagnosticarlo → cómo resolverlo, con los
> archivos implicados y el riesgo de cada solución. Ordenada por área.

---

## 1. Cómo usar esta guía

Cada entrada sigue la misma estructura. Antes de nada, tres comandos que resuelven la mitad
de los diagnósticos:

```bash
./scripts/verify-environment.sh          # ¿tengo lo necesario?
rootcause config show                    # ¿qué configuración está en uso y dónde?
rootcause audit 25                       # ¿qué acciones se ejecutaron y con qué resultado?
```

## 2. Compilación

### 2.1 `error: linker 'cc' not found`

| Aspecto | Detalle |
|---|---|
| **Causa** | Faltan las herramientas de línea de comandos de Xcode |
| **Diagnóstico** | `xcode-select -p` no devuelve una ruta |
| **Solución** | `xcode-select --install` |
| **Archivos** | Ninguno del proyecto |
| **Riesgo** | Ninguno |

### 2.2 `failed to run custom build command for libsqlite3-sys`

| Aspecto | Detalle |
|---|---|
| **Causa** | `rusqlite` con `bundled` compila SQLite desde C y no encuentra compilador o SDK |
| **Diagnóstico** | `cc --version`; `xcode-select -p` |
| **Solución** | Reinstalar las herramientas de Xcode; `cargo clean && cargo build` |
| **Archivos** | `Cargo.toml` (característica `bundled`) |
| **Riesgo** | `cargo clean` obliga a recompilar todo (varios minutos) |

### 2.3 `package requires rustc 1.82 or newer`

| Aspecto | Detalle |
|---|---|
| **Causa** | Toolchain anterior a la mínima declarada |
| **Diagnóstico** | `rustc --version` |
| **Solución** | `rustup update stable` |
| **Archivos** | `Cargo.toml` (`rust-version`), `rust-toolchain.toml` |
| **Riesgo** | Ninguno |

### 2.4 `lipo: can't open input file … x86_64-apple-darwin/release/rootcause`

| Aspecto | Detalle |
|---|---|
| **Causa** | Rust instalado por Homebrew: solo trae el target del host |
| **Diagnóstico** | `command -v rustup` no devuelve nada |
| **Solución** | Instalar Rust con `rustup`, o construir sin `--universal` |
| **Archivos** | `scripts/package-app.sh` |
| **Riesgo** | Ninguno: el script ya se degrada solo con un aviso |

### 2.5 La CI falla con un lint que en local no aparece

| Aspecto | Detalle |
|---|---|
| **Causa** | El runner usa una versión de `rustc` más reciente con lints nuevos |
| **Diagnóstico** | Comparar `rustc --version` local con el paso «Versiones» del log de CI |
| **Solución** | `rustup update stable` y volver a ejecutar `./scripts/ci-local.sh` |
| **Archivos** | Los que señale `clippy` |
| **Riesgo** | Ninguno |
| **Precedente** | Ya ocurrió: commit *«Corregir dos lints que solo aparecen en rustc 1.97 (el del runner)»* |

## 3. Arranque y ejecución

### 3.1 `No se pudo inicializar RootCause: …`

| Aspecto | Detalle |
|---|---|
| **Causa** | `InspectorService::new` falló: normalmente no se puede crear o escribir la carpeta de datos |
| **Diagnóstico** | `ls -ld ~/Library/Application\ Support/RootCauseInspector` |
| **Solución** | Corregir permisos del directorio, o borrarlo para que se recree |
| **Archivos** | `src/services/persistence.rs`, `src/services/resilience.rs` |
| **Riesgo** | Borrar el directorio **elimina historial y baselines** |

### 3.2 La ventana no abre o falla con un error de `glow`

| Aspecto | Detalle |
|---|---|
| **Causa** | No hay contexto gráfico: sesión SSH, sin pantalla, o entorno sin OpenGL |
| **Diagnóstico** | Ejecutar `rootcause status`: si funciona, el motor está bien y el problema es gráfico |
| **Solución** | Usar el CLI, o la edición CLI-only (`--no-default-features`) |
| **Archivos** | `src/main.rs::launch_gui` |
| **Riesgo** | Ninguno |

### 3.3 macOS impide abrir el `.app` descargado

| Aspecto | Detalle |
|---|---|
| **Causa** | El binario **no está firmado ni notarizado**; Gatekeeper lo bloquea |
| **Diagnóstico** | El diálogo lo dice explícitamente |
| **Solución** | Ajustes del Sistema → Privacidad y seguridad → «Abrir igualmente»; o compilar desde el código, que es la vía recomendada |
| **Archivos** | Notas de la release, `LÉEME.txt` del `.dmg` |
| **Riesgo** | Autorizar software sin firmar es una decisión consciente: hazlo solo con artefactos cuyo hash coincida con `SHA256SUMS.txt` |

### 3.4 La aplicación consume CPU de forma perceptible

| Aspecto | Detalle |
|---|---|
| **Causa** | Intervalo de refresco muy bajo, `signature_budget` alto, o cachés enormes que medir |
| **Diagnóstico** | `rootcause config show`; observar el tiempo entre capturas en la barra de estado |
| **Solución** | Subir `refresh_interval_secs` a 15–30; bajar `signature_budget` |
| **Archivos** | `rootcause-config.json` |
| **Riesgo** | Ventana ciega mayor entre capturas |

## 4. Datos que faltan

### 4.1 La sección Privacidad está vacía y avisa

| Aspecto | Detalle |
|---|---|
| **Causa** | Falta Acceso total al disco para el binario que se ejecuta |
| **Diagnóstico** | `rootcause tcc` imprime el mensaje y las dos rutas que no pudo abrir |
| **Solución** | Ajustes del Sistema → Privacidad y seguridad → Acceso total al disco → añadir el binario o el `.app` |
| **Archivos** | `src/services/tcc.rs` |
| **Riesgo** | Ese permiso es amplio: concédelo solo a un binario que hayas compilado o verificado |

> **Detalle que confunde a menudo:** el permiso se concede al **ejecutable concreto**.
> Dárselo a `Terminal.app` cubre lo que se lance desde ese terminal, pero no al `.app`, y
> viceversa.

### 4.2 `No se observaron conexiones`

| Aspecto | Detalle |
|---|---|
| **Causa** | Sin root, `lsof` solo ve los sockets del propio usuario |
| **Diagnóstico** | `rootcause status` imprime el contexto: usuario, UID y aviso de privilegios |
| **Solución** | Es el comportamiento esperado. Para la vista completa: `sudo rootcause connections` |
| **Archivos** | `src/services/macos.rs::lsof_connections` |
| **Riesgo** | Ejecutar como root amplía lo que la herramienta puede hacer; el producto no lo necesita para su función principal |

### 4.3 La lista de login items está vacía

| Aspecto | Detalle |
|---|---|
| **Causa** | No se ha concedido el permiso de Automatización, o no se pulsó el botón |
| **Diagnóstico** | `rootcause persistence --login-items`; debería aparecer el diálogo del sistema |
| **Solución** | Aceptar el diálogo; si se rechazó antes: Ajustes → Privacidad y seguridad → Automatización |
| **Archivos** | `src/services/launchd.rs::login_items` |
| **Riesgo** | Ninguno |

### 4.4 Un control de seguridad aparece como «Desconocido»

| Aspecto | Detalle |
|---|---|
| **Causa** | El comando no respondió, o su texto cambió en una versión nueva de macOS |
| **Diagnóstico** | Ejecutar a mano el comando de la columna Evidencia (`spctl --status`, `csrutil status`, `fdesetup status`…) |
| **Solución** | Si el comando responde algo no contemplado, es un fallo del parser: abrir un issue con la salida literal |
| **Archivos** | `src/services/security.rs` |
| **Riesgo** | Ninguno: «desconocido» se muestra en amarillo, nunca en verde |

### 4.5 La sección Red no muestra equipos

| Aspecto | Detalle |
|---|---|
| **Causa** | El escaneo pasivo solo ve equipos con los que este Mac ya habló |
| **Diagnóstico** | `arp -a -n` a mano; comparar con lo que muestra el producto |
| **Solución** | Usar el escaneo profundo (`rootcause network --deep` o el botón de la sección) |
| **Archivos** | `src/services/netscan.rs` |
| **Riesgo** | El barrido envía 254 pings al segmento: es ruidoso y puede activar alertas en redes vigiladas. **No lo ejecutes en una red ajena sin autorización** |

### 4.6 Las definiciones de XProtect no se leen

| Aspecto | Detalle |
|---|---|
| **Causa** | Las rutas conocidas no existen en esa versión de macOS |
| **Diagnóstico** | `ls -l /Library/Apple/System/Library/CoreServices/XProtect.bundle/Contents/Info.plist` |
| **Solución** | Si la ruta es otra, hay que añadirla a `DEFINITION_PATHS` |
| **Archivos** | `src/services/security.rs` |
| **Riesgo** | Ninguno; se reporta en amarillo con titular explícito |

## 5. Baselines y falsos positivos

### 5.1 Todo aparece como NUEVO tras una actualización

| Aspecto | Detalle |
|---|---|
| **Causa** | La baseline se sembró antes; una actualización de macOS o de aplicaciones cambió muchas entradas a la vez |
| **Diagnóstico** | `rootcause persistence` y revisar si los cambios corresponden a software conocido |
| **Solución** | Revisar uno a uno y, si todo es reconocible, `rootcause persistence --accept` |
| **Archivos** | `src/services/baseline.rs`, tabla `persistence_baseline` |
| **Riesgo** | **Alto si se acepta sin revisar**: aceptar convierte un hallazgo real en estado normal |

### 5.2 Un cambio se sigue reportando después de revisarlo

| Aspecto | Detalle |
|---|---|
| **Causa** | Comportamiento por diseño: los cambios son «pegajosos» hasta que se aceptan explícitamente |
| **Diagnóstico** | El elemento sigue marcado NUEVA/MODIFICADA en cada captura |
| **Solución** | Aceptar la baseline de esa superficie |
| **Archivos** | `src/services/baseline.rs` |
| **Riesgo** | Ninguno; es la garantía de que una alerta no se auto-silencia |

### 5.3 Se aceptó una baseline por error

| Aspecto | Detalle |
|---|---|
| **Causa** | Acción irreversible |
| **Diagnóstico** | `rootcause audit` muestra la acción `accept-*-baseline` con su fecha y recuento |
| **Solución** | No hay deshacer. Opciones: borrar el SQLite (se pierde también el historial) o revisar manualmente la superficie con las herramientas del sistema |
| **Archivos** | `rootcause-history.db`, tablas `baseline` y `persistence_baseline` |
| **Riesgo** | Borrar el SQLite elimina toda la evidencia acumulada |

### 5.4 Un proceso legítimo dispara anomalías continuamente

| Aspecto | Detalle |
|---|---|
| **Causa** | El proceso supera umbrales de forma sostenida (copias de seguridad, compiladores, sincronización) |
| **Diagnóstico** | Ver el `kind` del evento en `rootcause incidents --json` |
| **Solución** | Subir el umbral correspondiente, o añadir el nombre a `anomaly.trusted_process_names` |
| **Archivos** | `rootcause-config.json` |
| **Riesgo** | Añadir a la lista de confianza excluye ese nombre de **todas** las heurísticas, no solo de la que molesta |

### 5.5 «La puerta de enlace cambió de MAC»

| Aspecto | Detalle |
|---|---|
| **Causa** | Cambió el router, se cambió de red, o hay una suplantación ARP |
| **Diagnóstico** | Comparar la MAC mostrada con la del router físico; `arp -a -n \| grep <ip_gateway>` |
| **Solución** | Si el cambio es legítimo (router nuevo, otra red), aceptar la red conocida. Si no, **desconectar de esa red antes de seguir** |
| **Archivos** | `src/services/netscan.rs` |
| **Riesgo** | Aceptar sin verificar da por buena una posible suplantación |

## 6. Configuración

### 6.1 «Configuración con respaldo» aparece en cada captura

| Aspecto | Detalle |
|---|---|
| **Causa** | El JSON tiene un error de sintaxis; se están usando los valores por defecto |
| **Diagnóstico** | El detalle de la alerta incluye la ruta y el error de `serde` |
| **Solución** | Corregir el JSON, o renombrarlo y ejecutar `rootcause config init` |
| **Archivos** | `rootcause-config.json` |
| **Riesgo** | Ninguno: el historial y las baselines viven en otro archivo |

### 6.2 «La configuración cambió respecto a la sesión anterior»

| Aspecto | Detalle |
|---|---|
| **Causa** | La huella (tamaño + fecha) del archivo cambió entre dos arranques |
| **Diagnóstico** | `rootcause audit` muestra `agent-config-changed` con ambas huellas |
| **Solución** | Si el cambio fue tuyo, ignorar: desaparece en el siguiente arranque. Si no lo fue, **investigar quién editó el archivo** |
| **Archivos** | `src/services/resilience.rs` |
| **Riesgo** | Ninguno |

### 6.3 Un cambio de umbral no tiene efecto

| Aspecto | Detalle |
|---|---|
| **Causa** | Se editó el archivo con la aplicación abierta, o el campo no se lee (`suspicious_parent_names`, `shell_interpreters`, `daily_report`, `notification_cooldown_secs`, `stale_after_secs`) |
| **Diagnóstico** | Reiniciar y comprobar con `rootcause config show`; consultar la lista de campos sin efecto en [10 · Configuración](10-configuration.md) |
| **Solución** | Reiniciar la aplicación; si el campo es de los que no se leen, no hay solución sin tocar el código |
| **Archivos** | `src/config.rs` |
| **Riesgo** | Ninguno |

## 7. Base de datos e historial

### 7.1 «Persistencia con advertencia: no se pudo guardar el historial SQLite»

| Aspecto | Detalle |
|---|---|
| **Causa** | Disco lleno, permisos, o el archivo bloqueado por otro proceso |
| **Diagnóstico** | `df -h ~`; `ls -l ~/Library/Application\ Support/RootCauseInspector/` |
| **Solución** | Liberar espacio o corregir permisos |
| **Archivos** | `rootcause-history.db` |
| **Riesgo** | Se pierde ese punto del historial, no la captura en pantalla |

### 7.2 El archivo SQLite crece más de lo esperado

| Aspecto | Detalle |
|---|---|
| **Causa** | `audit_log` **no se recorta**; `history_limit` e `incident_limit` altos |
| **Diagnóstico** | `sqlite3 <ruta> "SELECT COUNT(*) FROM audit_log;"` |
| **Solución** | Bajar los límites en la configuración; borrar filas antiguas de `audit_log` a mano |
| **Archivos** | `src/services/persistence.rs` |
| **Riesgo** | Borrar auditoría elimina evidencia |

### 7.3 Se quiere empezar de cero

```bash
rootcause history --backup    # copia previa por si acaso
rm ~/Library/Application\ Support/RootCauseInspector/rootcause-history.db
```

| Aspecto | Detalle |
|---|---|
| **Consecuencia** | Se pierden historial, incidentes, auditoría **y las cuatro baselines** |
| **Riesgo** | Alto: tras el borrado, cualquier persistencia sospechosa ya presente se siembra como «conocida» |

## 8. IA opcional

### 8.1 `La integración IA está desactivada en la configuración`

Es el estado de fábrica. Para activarla: [10 · Configuración §12](10-configuration.md).

### 8.2 `No existe la variable de entorno … con la API key`

| Aspecto | Detalle |
|---|---|
| **Causa** | La variable no está exportada en la sesión que ejecuta el binario |
| **Diagnóstico** | `echo ${ROOTCAUSE_AI_API_KEY:+definida}` |
| **Solución** | Exportarla antes de ejecutar. Para la GUI lanzada desde el Finder, la variable **no** se hereda del terminal: hay que lanzarla desde una sesión que la tenga |
| **Archivos** | `src/services/ai.rs` |
| **Riesgo** | No guardes la clave en el archivo de configuración: el producto no la lee de ahí |

### 8.3 `La respuesta IA no trae choices[0].message.content`

| Aspecto | Detalle |
|---|---|
| **Causa** | El proveedor no es compatible con la forma de la API de chat de OpenAI |
| **Diagnóstico** | Probar el endpoint con `curl` a mano y observar la forma de la respuesta |
| **Solución** | Usar un proveedor compatible; adaptar el parser exigiría tocar el código |
| **Archivos** | `src/services/ai.rs::parse_response` |
| **Riesgo** | Ninguno: el incidente local no se ve afectado |

## 9. Empaquetado y release

### 9.1 `hdiutil: create failed - Operation not permitted`

| Aspecto | Detalle |
|---|---|
| **Causa** | El repositorio está en exFAT o en un volumen de red; `hdiutil` solo trabaja sobre APFS o HFS+ |
| **Diagnóstico** | `df -T` o `diskutil info $(df . \| tail -1 \| awk '{print $1}')` |
| **Solución** | Ya lo evita `package-dmg.sh` creando la imagen en un temporal del disco interno. Si aún falla, ejecutar desde una copia en el disco interno |
| **Archivos** | `scripts/package-dmg.sh` |
| **Riesgo** | Ninguno |

### 9.2 Aparecen archivos `._*` en `dist/`

| Aspecto | Detalle |
|---|---|
| **Causa** | Bifurcaciones AppleDouble que macOS crea en volúmenes no nativos |
| **Diagnóstico** | `ls -la dist/` |
| **Solución** | `export COPYFILE_DISABLE=1` antes de empaquetar (`release-product.sh` ya lo hace) |
| **Archivos** | `.gitignore` ya los excluye; `.markdownlint-cli2.jsonc` también |
| **Riesgo** | Ninguno |

### 9.3 `El binario reporta vX pero Cargo.toml dice vY`

| Aspecto | Detalle |
|---|---|
| **Causa** | El `.app` se construyó con un binario de una versión anterior |
| **Diagnóstico** | El propio script lo detecta y aborta |
| **Solución** | `rm -rf dist target/release/rootcause` y volver a empaquetar |
| **Archivos** | `scripts/release-product.sh`, fase 4 |
| **Riesgo** | Ninguno |

### 9.4 `No se publican artefactos de una sola arquitectura`

| Aspecto | Detalle |
|---|---|
| **Causa** | El `.app` no es universal y se pidió `--publish` |
| **Solución** | Instalar `rustup`, o publicar con `--tag-only` para que los construya la CI |
| **Archivos** | `scripts/release-product.sh`, fase 7 |
| **Riesgo** | Ninguno |

## 10. Documentación

### 10.1 La CI falla en el job `docs`

| Aspecto | Detalle |
|---|---|
| **Causa** | Un Markdown incumple `.markdownlint-cli2.jsonc` |
| **Diagnóstico** | `npx --yes markdownlint-cli2` en local; el log de CI señala archivo, línea y regla |
| **Solución** | Corregir la regla indicada. Las habituales: MD013 (línea > 100), MD032 (lista sin línea en blanco), MD040 (bloque de código sin lenguaje), MD024 (encabezado duplicado) |
| **Archivos** | El que señale el log |
| **Riesgo** | Ninguno |

### 10.2 `Faltan dependencias: markdown, xhtml2pdf`

| Aspecto | Detalle |
|---|---|
| **Causa** | El generador de PDF necesita esas dos bibliotecas de Python |
| **Solución** | `python3 -m pip install markdown xhtml2pdf`, o un entorno virtual |
| **Archivos** | `scripts/build-docs-pdf.py` |
| **Riesgo** | Ninguno: son dependencias solo de documentación |

### 10.3 Los diagramas del PDF salen como texto

Es el comportamiento documentado: el generador no ejecuta JavaScript, así que los diagramas
Mermaid viajan como código fuente legible. Ver
[13 · Despliegue y operación §13.4](13-deployment-and-operations.md).

## 11. Tabla resumen

| Síntoma | Documento con el detalle |
|---|---|
| No compila | §2 |
| No arranca | §3 |
| Falta una superficie de datos | §4 |
| Demasiadas alertas o ninguna | §5 |
| La configuración no hace efecto | §6 |
| Historial o base de datos | §7 |
| IA opcional | §8 |
| Empaquetado o release | §9 |
| Documentación y PDF | §10 |

---

**Siguiente lectura recomendada:** [15 · Riesgos y deuda técnica](15-risks-and-technical-debt.md).
