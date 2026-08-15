# Referencia de la CLI

Todo lo que hace la interfaz gráfica se puede hacer desde consola. Casi todos los comandos aceptan
`--json` para encadenarlos con otras herramientas.

```bash
rootcause --help       # ayuda completa
rootcause --version    # versión
```

## Diagnóstico

### `rootcause status [--json]`

Captura completa y veredicto del equipo: semáforo, métricas, controles de seguridad, XProtect,
persistencia, privacidad, contexto de ejecución, alertas e incidente dominante.

```bash
rootcause status
rootcause status --json | jq '.overview.primary_severity'
```

### `rootcause snapshot [--output RUTA]`

La captura completa en JSON. Sin `--output`, la escribe por salida estándar.

```bash
rootcause snapshot --output ~/Desktop/captura.json
```

### `rootcause report`

Genera un reporte forense en Markdown en `~/Documents/RootCause/reports/`. Es la salida pensada
para que la lea una persona o para adjuntarla a un ticket.

### `rootcause export`

Exporta la captura actual a JSON en Descargas.

## Persistencia

```bash
rootcause persistence               # entradas con su estado vs baseline
rootcause persistence --json        # detalle completo
rootcause persistence --all         # incluye las carpetas de Apple (cientos de entradas)
rootcause persistence --login-items # consulta login items (pide permiso de Automatización)
rootcause persistence --accept      # fija el estado actual como baseline
```

## Seguridad

```bash
rootcause security            # Gatekeeper, SIP, FileVault, firewall, modo encubierto, SSH
rootcause security --json
rootcause security --accept   # fija el estado actual como baseline
rootcause xprotect            # versión y antigüedad de las definiciones de Apple
```

## Privacidad

```bash
rootcause tcc              # todos los permisos concedidos
rootcause tcc --sensitive  # solo los que dan control real sobre el equipo
rootcause tcc --accept     # fija los permisos actuales como baseline
```

Requiere Acceso total al disco. Sin él, devuelve código de salida 1 y explica qué falta.

## Red

```bash
rootcause connections          # conexiones activas por proceso
rootcause network              # vecinos del segmento local (pasivo, instantáneo)
rootcause network --deep       # barrido activo del /24 + resolución de nombres
rootcause network --accept     # fija los equipos actuales como red conocida
rootcause block-ip 203.0.113.5 # imprime la regla pf; NO la aplica
```

## Historial y auditoría

```bash
rootcause history          # últimas 20 capturas
rootcause history 60       # últimas 60
rootcause history --json
rootcause history --backup # copia el historial a JSON junto al SQLite
rootcause incidents        # incidentes persistidos
rootcause audit            # acciones ejecutadas, con su resultado
```

## Mantenimiento

```bash
rootcause events --minutes 60  # eventos de seguridad del log unificado (lento)
rootcause clean-caches         # SIMULACIÓN: calcula sin borrar
rootcause clean-caches --yes   # limpia de verdad ~/Library/Caches (>24 h, no en uso)
```

## Configuración e IA opcional

```bash
rootcause config show          # ruta y configuración efectiva
rootcause config show --json
rootcause config init          # crea el JSON de configuración si no existe
rootcause ai explain-latest    # enriquece el último incidente con IA (si está activada)
```

## Intervención

```bash
rootcause kill 1234   # envía SIGTERM a un proceso no protegido
```

Los procesos que sostienen la sesión gráfica o el arranque están protegidos y no se pueden
finalizar desde aquí.

## Códigos de salida

| Código | Significa |
|---|---|
| `0` | Todo bien |
| `1` | Error de ejecución (permiso, recurso no disponible, comando falló) |
| `2` | Uso incorrecto (falta un argumento obligatorio) |

## Ejemplos de automatización

```bash
# Alerta si el veredicto no es verde
if [ "$(rootcause status --json | jq -r '.overview.primary_severity')" != "Healthy" ]; then
  rootcause report
fi

# Contar cambios de persistencia sin aceptar la baseline
rootcause persistence --json | jq '[.[] | select(.change_status != "unchanged")] | length'

# Listar apps con Acceso total al disco
rootcause tcc --json | jq -r '.permissions[] | select(.service == "kTCCServiceSystemPolicyAllFiles" and .allowed) | .client'
```
