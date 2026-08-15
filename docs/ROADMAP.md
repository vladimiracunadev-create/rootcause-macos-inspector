# Roadmap

## v0.1.0 — actual

Base completa del producto en macOS.

- [x] Inventario de persistencia (LaunchAgents, LaunchDaemons, login items, `cron`)
- [x] Motor genérico de baseline sobre cuatro superficies
- [x] Controles de seguridad nativos con evidencia del comando
- [x] Estado y antigüedad de las definiciones de XProtect
- [x] Auditoría de permisos TCC (usuario y sistema)
- [x] Conexiones por proceso con verificación de firma de código
- [x] Vecinos de red con detección de suplantación ARP de la puerta de enlace
- [x] Heurísticas de comportamiento con memoria entre capturas
- [x] Interfaz de 12 secciones con motor en hilo aparte
- [x] CLI completa con `--json`
- [x] Historial, incidentes y auditoría en SQLite
- [x] Reporte forense en Markdown
- [x] Empaquetado `.app` y `.dmg`
- [x] CI en `macos-latest` con formato, clippy, tests y build

## v0.2.0 — próxima

- [ ] **Sección de eventos del log unificado** como tab propio, con filtros por subsistema
- [ ] **Diff visual de capturas** (A vs B) en la sección Historial
- [ ] **Detalle de un incidente** en ventana propia, con toda su evidencia
- [ ] **Exportación a CSV** además de JSON
- [ ] **Inventario de extensiones de sistema** (`systemextensionsctl list`)
- [ ] **Verificación de integridad de aplicaciones** en `/Applications` contra su firma

## v0.3.0

- [ ] **Ampliar la tabla OUI** de fabricantes de red
- [ ] **Vigilancia de perfiles de configuración** (MDM) instalados
- [ ] **Detección de cambios en `/etc/hosts`** y en la configuración DNS
- [ ] **Modo desatendido** que genere un reporte diario sin abrir la interfaz
- [ ] **Notificaciones agrupadas** con ventana de silencio configurable

## Explorado, aún sin compromiso

- **Endpoint Security Framework.** Daría intercepción en tiempo real en vez de muestreo, pero exige
  un `entitlement` concedido por Apple y una extensión de sistema. Cambiaría la naturaleza del
  producto: de sensor de espacio de usuario a componente privilegiado.
- **Firma y notarización.** Requiere cuenta de Apple Developer. Hoy la vía es compilar desde el
  código, lo que además permite auditarlo.
- **Reglas tipo Sigma.** Un motor de reglas declarativas permitiría añadir detecciones sin
  recompilar. Habría que resolver primero de dónde vienen esas reglas y cómo se verifican.
- **Paridad de superficies con la edición Windows.** Los dos productos comparten arquitectura pero
  no superficies; conviene mantener cada uno idiomático a su plataforma en vez de forzar simetría.

## Principios que no van a cambiar

1. **Diagnóstico primero, intervención después.** No habrá eliminación automática de nada.
2. **Análisis local.** No habrá telemetría ni servidor.
3. **Honestidad sobre los límites.** Antes de añadir una detección, se documenta qué no cubre.
4. **Sin permisos silenciosos.** Ningún permiso del sistema se pedirá sin una acción explícita.
