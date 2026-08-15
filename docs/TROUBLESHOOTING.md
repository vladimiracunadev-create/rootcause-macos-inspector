# Solución de problemas

## «Permisos TCC no legibles»

**Causa:** RootCause no tiene Acceso total al disco.

**Solución:** Ajustes del Sistema → Privacidad y seguridad → Acceso total al disco → **+** → añade
`RootCause.app` (o el Terminal, si usas la CLI). Reinicia RootCause.

Comprobar:

```bash
rootcause tcc
```

Detalle → [`PERMISOS_MACOS.md`](PERMISOS_MACOS.md).

## La sección Conexiones aparece casi vacía

**Causa:** sin privilegios de root, `lsof` solo ve los sockets de tu propio usuario.

**Solución:** es el comportamiento esperado y la app lo declara. Si necesitas la vista completa:

```bash
sudo rootcause connections
```

## «Modo encubierto del firewall: Desconocido»

**Causa:** la redacción de `socketfilterfw` cambia entre versiones de macOS y una nueva variante
puede no reconocerse.

**Solución:** comprueba manualmente y, si el texto no coincide con ninguna variante conocida, abre
un issue con la salida exacta:

```bash
/usr/libexec/ApplicationFirewall/socketfilterfw --getstealthmode
```

## Alertas de cambio nada más aceptar la baseline

**Causa esperada:** la baseline se aceptó con un valor y en la siguiente captura el sistema
devolvió otro (por ejemplo, un control que pasó de «Desconocido» a un estado concreto).

**Solución:** acepta la baseline una vez más. Si se repite en cada captura, el valor está
oscilando: abre un issue con la salida de `rootcause security --json`.

## El escaneo profundo de red tarda muchísimo

**Causa esperada:** hace ping a las 254 direcciones del `/24`, en serie y con timeout de 1 s.

**Solución:** úsalo solo cuando lo necesites. El escaneo pasivo (el que corre en cada captura) es
instantáneo y no envía ningún paquete.

## `rootcause events` tarda varios segundos

**Causa esperada:** `log show` consulta el log unificado del sistema, y eso es lento por diseño.

**Solución:** reduce la ventana con `--minutes 10`. Esta consulta nunca forma parte de la captura
periódica precisamente por su coste.

## Gatekeeper bloquea el `.app` descargado

**Causa:** RootCause no está firmado ni notarizado.

**Solución:** Ajustes del Sistema → Privacidad y seguridad → «Abrir de todos modos». O quita el
atributo de cuarentena:

```bash
xattr -d com.apple.quarantine /Applications/RootCause.app
```

O compílalo tú, que es la vía recomendada.

## La app tarda en mostrar la primera captura

**Causa esperada:** la primera captura consulta siete superficies y verifica firmas de código.

**Solución:** ninguna. La ventana no se bloquea mientras tanto: el motor corre en un hilo aparte.
Las capturas siguientes son más rápidas porque las firmas se cachean por ruta.

## Un proceso legítimo aparece como crítico

**Causa:** los umbrales por defecto no encajan con tu equipo o tu carga de trabajo.

**Solución:** súbelos en Configuración o en `rootcause-config.json`. Prefiere ajustar un umbral a
desactivar la detección entera: una detección apagada no avisa de nada.

## «El proceso ya no existe» al finalizar

**Causa esperada:** el proceso terminó entre la captura y el clic.

**Solución:** actualiza con `F5`.

## «Proceso protegido por política local»

**Causa esperada:** intentaste finalizar un proceso que sostiene la sesión gráfica o el arranque
(`kernel_task`, `launchd`, `WindowServer`…).

**Solución:** ninguna, y es deliberado. Matarlos dejaría el equipo inutilizable.

## Empezar de cero

```bash
rm -rf ~/Library/Application\ Support/RootCauseInspector
```

Borra configuración, historial, incidentes, auditoría y todas las baselines. La próxima captura
sembrará una baseline nueva.

## Reportar un problema

Adjunta:

```bash
rootcause --version
sw_vers
rootcause status --json > estado.json   # revisa el contenido antes de compartirlo
```

Para fallos de seguridad, no abras un issue público: ver [`../SECURITY.md`](../SECURITY.md).
