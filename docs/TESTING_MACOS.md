# Pruebas

## Automáticas

```bash
cargo test --all-features
```

Más de cien tests que cubren la lógica que se puede probar sin un macOS concreto detrás.

### Qué está cubierto

| Módulo | Qué se prueba |
|---|---|
| `rules` | Clasificación de procesos, construcción de alertas, derivación de incidentes |
| `anomaly` | Que un pico no dispare, que una racha sí, y los falsos positivos ya corregidos |
| `network` | Parseo de `lsof` en modo campo, incluidos nombres con espacios; IPs públicas vs privadas |
| `netscan` | Parseo de `arp`, normalización de MAC, OUI, orden numérico de IPs |
| `launchd` | Clasificación de riesgo de entradas de persistencia sintéticas |
| `security` | Umbrales de antigüedad de firmas, severidad de controles desconocidos |
| `tcc` | Decodificación de ambos esquemas de `TCC.db`, filtro de permisos sensibles |
| `baseline` | Eventos de cambio por superficie |
| `persistence` | Estabilidad de la clave que identifica una entrada |
| `config` | Valores por defecto y tolerancia a JSON incompleto |
| `report` | Que el Markdown no se rompa con caracteres especiales |
| `ai` | Que sin configuración no toque la red; parseo de respuestas |
| `cli` | Parseo de banderas y códigos de salida |
| `app` | Búfer de tendencia, secciones sin duplicados |

### Filosofía de los tests

Los tests describen **el comportamiento que importa**, no la implementación. Varios documentan
falsos positivos ya corregidos, para que no vuelvan:

- `varias_instancias_del_mismo_nombre_no_son_una_reaparicion`
- `un_navegador_instalado_con_normalidad_no_dispara_trafico_inusual`
- `un_pico_de_cpu_no_dispara_nada`
- `control_desconocido_no_se_pinta_de_verde`

## Manuales

Lo que ningún test automático puede cubrir.

### Persistencia

1. Crea un LaunchAgent de prueba:

   ```bash
   cat > ~/Library/LaunchAgents/dev.prueba.rootcause.plist <<'PLIST'
   <?xml version="1.0" encoding="UTF-8"?>
   <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
   <plist version="1.0">
   <dict>
     <key>Label</key><string>dev.prueba.rootcause</string>
     <key>ProgramArguments</key><array><string>/bin/echo</string><string>hola</string></array>
     <key>RunAtLoad</key><true/>
   </dict>
   </plist>
   PLIST
   ```

2. `rootcause persistence` debe marcarla como **NUEVA**.
3. `rootcause persistence --accept` y volver a ejecutar: ya no debe marcarse.
4. Borra el archivo: debe marcarse como **ELIMINADA**.
5. Limpia: `rm ~/Library/LaunchAgents/dev.prueba.rootcause.plist && rootcause persistence --accept`

### Controles de seguridad

Cambia el estado del firewall en Ajustes del Sistema y comprueba que la siguiente captura lo
detecta como cambio vs baseline.

### Permisos TCC

1. Sin Acceso total al disco: debe decir que no puede leer, y el resto debe funcionar.
2. Con Acceso total al disco: debe listar los permisos.
3. Concede un permiso nuevo a cualquier app y comprueba que aparece como cambio.

### Interfaz

| Prueba | Criterio |
|---|---|
| Primera apertura | Muestra el indicador de carga, no una ventana en blanco |
| Durante una captura | La ventana sigue respondiendo (se puede cambiar de sección) |
| Escaneo profundo de red | La ventana no se congela pese a tardar |
| Cambio de tema | Los tres modos se aplican al momento |
| Cambio de idioma | Toda la interfaz cambia sin reiniciar |
| Redimensionar a 880×600 | Nada se solapa ni se corta |
| `F5`, `⌘E`, `⌘R` | Los tres atajos responden |

### Edición CLI-only

```bash
cargo build --release --no-default-features
./target/release/rootcause status
```

Debe funcionar igual, sin dependencias gráficas.

## Antes de una release

Ver [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md).
