# 19 · Matriz de trazabilidad

> Cada funcionalidad seguida desde la interfaz hasta el dato persistido y su prueba. Sirve
> para responder dos preguntas: *«si cambio esto, ¿qué se rompe?»* y *«¿esta funcionalidad
> está realmente cubierta?»*.

---

## 1. Cómo leer la matriz

| Columna | Qué contiene |
|---|---|
| **Funcionalidad** | Lo que el usuario percibe |
| **Requisito** | Documento de requisitos o principio del producto que la justifica |
| **Interfaz** | Sección de la GUI y comando del CLI |
| **Módulo** | Archivo principal |
| **Función** | Símbolo real, sin traducir |
| **Persistencia** | Tabla o archivo donde queda el dato |
| **Prueba** | Test que la cubre, o el hueco |
| **Documento** | Dónde se explica |
| **Estado** | ✅ verificado · ⚠️ parcial · ❓ requiere validación |

Los requisitos citados son los del propio repositorio:
[`REQ-SEC-001`](../requirements/REQ-SEC-001-deteccion-comportamiento-anomalo.md),
[`REQ-SEC-002`](../requirements/REQ-SEC-002-autoproteccion-y-resiliencia.md) y
[`REQ-SEC-003`](../requirements/REQ-SEC-003-superficies-nativas-macos.md).

---

## 2. Superficie 1 · Persistencia

| Elemento | Detalle |
|---|---|
| **Funcionalidad** | Inventariar LaunchAgents, LaunchDaemons, login items y `cron`, y detectar cambios |
| **Requisito** | REQ-SEC-003 · «lo que sobrevive a un reinicio» |
| **Interfaz** | Sección Persistencia · `rootcause persistence [--all\|--login-items\|--accept]` |
| **Módulo** | `src/services/launchd.rs` |
| **Funciones** | `scan_persistence`, `parse_launch_plist`, `scan_cron`, `login_items`, `classify_entry` |
| **Clasificación** | `classify_entry` (7 señales, umbral crítico 85) |
| **Comparación** | `InspectorService::diff_persistence_baseline` + `persistence_entry_key` |
| **Evento** | `anomaly::persistence_change_event` → `kind = "persistence-change"` |
| **Persistencia** | Tabla `persistence_baseline`; el hallazgo, en `incidents.payload_json` |
| **Pruebas** | `agente_normal_de_aplicacion_es_riesgo_bajo_o_medio`, `daemon_sin_firmar_en_tmp_es_critico`, `label_que_imita_a_apple_fuera_del_sistema_sube_el_riesgo`, `intervalo_muy_corto_se_reporta`, `persistencia_nueva_y_sospechosa_escala_a_critica`, `la_clave_ignora_el_comando` |
| **Hueco** | El parseo de plists reales no tiene test (⚠️) |
| **Documento** | [06 §5](06-deep-code-explanation.md), [05 §4.6](05-technical-reference.md) |
| **Estado** | ✅ |

## 3. Superficie 2 · Procesos

| Elemento | Detalle |
|---|---|
| **Funcionalidad** | Listar procesos con consumo, ruta, usuario, línea de comandos y firma |
| **Requisito** | REQ-SEC-001 · «distorsión anómala de recursos» |
| **Interfaz** | Sección Procesos · `rootcause status` |
| **Módulo** | `src/services/inspector.rs` + `src/services/macos.rs` |
| **Funciones** | `collect_processes`, `macos::process_details`, `apply_signatures`, `reclassify_with_signatures`, `macos::code_signature` |
| **Clasificación** | `rules::classify_process` (7 señales, umbral crítico 55) |
| **Persistencia** | Solo el dominante, en `snapshots.dominant_process` |
| **Pruebas** | `proceso_tranquilo_es_saludable`, `binario_sin_firmar_en_tmp_con_escritura_intensa_es_critico`, `binario_oculto_suma_puntaje`, `una_cpu_alta_por_si_sola_no_es_critica`, `clasifica_*` (5 de firma) |
| **Hueco** | `collect_processes` no tiene test directo (⚠️) |
| **Documento** | [06 §3.3](06-deep-code-explanation.md) |
| **Estado** | ✅ |

## 4. Superficie 3 · Controles de seguridad

| Control | Comando | Función | Severidad si apagado | Test |
|---|---|---|---|---|
| Gatekeeper | `spctl --status` | `security::gatekeeper` | `Critical` | `control_apagado_toma_la_severidad_declarada` |
| SIP | `csrutil status` | `security::system_integrity_protection` | `Critical` | Idem |
| FileVault | `fdesetup status` | `security::filevault` | `Warning` | Idem |
| Firewall | `socketfilterfw --getglobalstate` | `security::application_firewall` | `Warning` | Idem |
| Modo encubierto | `socketfilterfw --getstealthmode` | `security::firewall_stealth_mode` | Nunca sube | — |
| SSH | `launchctl list` | `security::remote_login` | `Warning` si activo | — |

| Elemento | Detalle |
|---|---|
| **Requisito** | REQ-SEC-003 |
| **Interfaz** | Sección Seguridad · `rootcause security [--accept]` |
| **Comparación** | `security::control_watch_items` → `baseline::diff_surface(SECURITY_SURFACE)` |
| **Evento** | `baseline::surface_change_event` → `kind = "security-control-change"` |
| **Persistencia** | Tabla `baseline`, `surface = 'security-control'` |
| **Pruebas** | `control_desconocido_no_se_pinta_de_verde`, `los_controles_se_convierten_en_items_vigilables`, `control_de_seguridad_modificado_genera_evento_de_riesgo_alto`, `primera_linea_util_o_respaldo` |
| **Verificación real** | ✅ Ejecutado en este análisis: los seis controles responden con evidencia |
| **Documento** | [06 §6](06-deep-code-explanation.md) |
| **Estado** | ✅ |

## 5. Superficie 4 · Antimalware de Apple

| Elemento | Detalle |
|---|---|
| **Funcionalidad** | Versión y antigüedad de XProtect, XProtect Remediator y MRT |
| **Requisito** | REQ-SEC-003 |
| **Interfaz** | Sección Seguridad · `rootcause xprotect` |
| **Módulo** | `src/services/security.rs` |
| **Funciones** | `scan_xprotect`, `read_definition`, `age_severity`, `age_note` |
| **Umbrales** | `thresholds.xprotect.warning_days` (30) y `critical_days` (90) |
| **Persistencia** | Ninguna propia; se refleja en `alerts_json` si genera alerta |
| **Prueba** | `antiguedad_de_firmas_escala_con_los_umbrales` |
| **Hueco** | La lectura de los `Info.plist` reales no tiene test (⚠️) |
| **Documento** | [06 §6.3](06-deep-code-explanation.md) |
| **Estado** | ✅ |

## 6. Superficie 5 · Privacidad (TCC)

| Elemento | Detalle |
|---|---|
| **Funcionalidad** | Inventariar permisos concedidos y detectar los nuevos sobre servicios sensibles |
| **Requisito** | REQ-SEC-003 |
| **Interfaz** | Sección Privacidad · `rootcause tcc [--sensitive\|--accept]` |
| **Módulo** | `src/services/tcc.rs` |
| **Funciones** | `scan`, `read_database`, `table_columns`, `decode_decision`, `service_label`, `severity_for`, `permission_watch_items` |
| **Comparación** | `baseline::diff_surface(TCC_SURFACE)`, solo sensibles y concedidos |
| **Evento** | `kind = "tcc-permission-change"` |
| **Persistencia** | Tabla `baseline`, `surface = 'tcc-permission'` |
| **Pruebas** | `decodifica_el_esquema_moderno`, `decodifica_el_esquema_heredado`, `traduce_los_servicios_conocidos`, `permiso_denegado_nunca_es_advertencia`, `solo_los_permisos_sensibles_concedidos_entran_en_la_baseline`, `epoch_cero_no_produce_fecha` |
| **Hueco** | La lectura de una `TCC.db` real no está probada ni se pudo verificar (❓) |
| **Verificación real** | ⚠️ Ejecutado: declara correctamente la falta de Acceso total al disco |
| **Documento** | [06 §7](06-deep-code-explanation.md) |
| **Estado** | ⚠️ |

## 7. Superficie 6a · Conexiones por proceso

| Elemento | Detalle |
|---|---|
| **Funcionalidad** | Qué proceso habla con el exterior, con qué destino y qué puertos expone |
| **Requisito** | REQ-SEC-001 |
| **Interfaz** | Sección Conexiones · `rootcause connections` |
| **Módulo** | `src/services/network.rs` (parseo) + `macos::lsof_connections` (origen) |
| **Funciones** | `parse_lsof_field_output`, `classify_connection`, `extract_ip`, `is_public_ip`, `unique_public_remotes_by_pid` |
| **Persistencia** | Recuento en `incidents.payload_json` (evidencia «Conexiones a IP pública») |
| **Pruebas** | `parsea_nombres_de_proceso_con_espacios`, `detecta_destino_publico_y_puerto_expuesto`, `ips_privadas_y_loopback_no_son_publicas`, `ips_publicas_se_reconocen`, `extrae_ip_de_extremos_v4_y_v6`, `agrupa_destinos_publicos_unicos_por_pid` |
| **Limitación** | Sin root solo se ven los sockets del propio usuario (❓ no verificado como root) |
| **Documento** | [06 §8](06-deep-code-explanation.md) |
| **Estado** | ✅ |

## 8. Superficie 6b · Vecinos de red

| Elemento | Detalle |
|---|---|
| **Funcionalidad** | Equipos del segmento local y detección de equipos nuevos o de suplantación de la puerta de enlace |
| **Requisito** | REQ-SEC-001, REQ-SEC-003 |
| **Interfaz** | Sección Red · `rootcause network [--deep\|--accept]` |
| **Módulo** | `src/services/netscan.rs` |
| **Funciones** | `scan`, `parse_arp_table`, `normalize_mac`, `vendor_from_mac`, `device_key`, `classify_device`, `device_watch_items`, `device_from_watch_item`, `new_device_event` |
| **Comparación** | `baseline::diff_surface(NETWORK_SURFACE_ID)` + `annotate_network_changes` |
| **Evento** | `kind = "unknown-device"`, puntaje 92 si es la puerta de enlace |
| **Persistencia** | Tabla `baseline`, `surface = 'network-device'` |
| **Pruebas** | `parsea_vecinos_y_descarta_ruido`, `normaliza_octetos_sin_cero_a_la_izquierda`, `reconoce_fabricante_por_oui`, `prefijo_de_subred_solo_para_ipv4`, `gateway_nuevo_es_incidente_critico`, `dispositivo_sin_cambios_no_genera_evento`, `el_propio_equipo_no_entra_en_la_baseline`, `ordena_ips_numericamente` |
| **Hueco** | El barrido activo no se ejecutó en este análisis (❓) |
| **Documento** | [06 §9](06-deep-code-explanation.md) |
| **Estado** | ✅ |

## 9. Superficie 7 · Almacenamiento

| Elemento | Detalle |
|---|---|
| **Funcionalidad** | Medir cachés y temporales; limpieza segura de `~/Library/Caches` |
| **Requisito** | Diagnóstico de recursos (REQ-SEC-001, uso no estrictamente de seguridad) |
| **Interfaz** | Sección Almacenamiento · `rootcause clean-caches [--yes]` |
| **Módulo** | `src/services/temp_scan.rs` |
| **Funciones** | `scan`, `measure_directory`, `severity_for_size`, `clean_user_caches` |
| **Salvaguardas** | Solo `~/Library/Caches`, 24 h de antigüedad, salta lo que está en uso, `dry_run` por defecto |
| **Persistencia** | `snapshots.cache_total_mb`; la limpieza real, en `audit_log` |
| **Pruebas** | `severidad_por_tamano_respeta_umbrales`, `expand_home_resuelve_rutas_de_usuario_y_absolutas`, `medir_directorio_inexistente_devuelve_cero`, `el_simulacro_nunca_marca_borrado_real`, `conversion_de_bytes_a_mb` |
| **Hueco** | El borrado real no está probado (⚠️, deliberado) |
| **Documento** | [06 §10](06-deep-code-explanation.md) |
| **Estado** | ✅ |

## 10. Motor de detección de anomalías

| `kind` | Función | Configuración | Prueba | Estado |
|---|---|---|---|---|
| `sustained-cpu` | `AnomalyTracker::analyze` | `cpu_sustained_percent`, `cpu_sustained_samples` | `cpu_sostenida_dispara_tras_las_muestras_configuradas`, `un_pico_de_cpu_no_dispara_nada`, `la_racha_se_rompe_si_la_cpu_baja` | ✅ |
| `memory-growth` | Idem | `memory_growth_mb`, `memory_growth_samples` | — | ⚠️ |
| `aggressive-write` | Idem | `aggressive_write_mb`, `aggressive_write_samples` | — | ⚠️ |
| `unusual-outbound` | Idem | `public_destination_count` | `muchos_destinos_publicos_disparan_trafico_inusual`, `un_navegador_instalado_con_normalidad_no_dispara_trafico_inusual`, `un_binario_sin_firmar_en_applications_si_dispara_trafico_inusual` | ✅ |
| `local-scan` | Idem | `local_scan_destination_count` | `destinos_privados_se_agrupan_sin_loopback` (parcial) | ⚠️ |
| `suspicious-path` | Idem | `suspicious_path_keywords` | `ruta_sospechosa_se_reporta_de_inmediato` | ✅ |
| `unsigned-binary` | Idem | `watch_unsigned_binaries` | Cubierto indirectamente | ⚠️ |
| `fast-respawn` | `track_respawns` | `respawn_window_secs`, `respawn_count` | `varias_instancias_del_mismo_nombre_no_son_una_reaparicion`, `un_proceso_unico_que_cambia_de_pid_si_es_una_reaparicion` | ✅ |

Guarda transversal: `is_trusted` + `binario_de_apple_en_ruta_del_sistema_se_ignora` ✅.
Interruptor general: `la_deteccion_desactivada_no_produce_eventos` ✅.

## 11. Motor de baseline

| Elemento | Detalle |
|---|---|
| **Funcionalidad** | Detectar NUEVA / MODIFICADA / ELIMINADA en cuatro superficies |
| **Requisito** | Principio central del producto |
| **Interfaz** | Columna «Cambio» en cuatro secciones · `--accept` en cuatro comandos |
| **Módulo** | `src/services/baseline.rs` |
| **Funciones** | `diff_surface`, `surface_change_event`, `SurfaceSpec` |
| **Persistencia** | Tablas `baseline` y `persistence_baseline` |
| **Pruebas** | `un_item_sin_cambios_no_genera_evento`, `control_de_seguridad_modificado_genera_evento_de_riesgo_alto`, `un_item_eliminado_baja_a_riesgo_medio`, `la_evidencia_incluye_el_tipo_de_cambio` |
| **Hueco crítico** | **La propiedad «pegajosa» y la siembra silenciosa no tienen ningún test** (⚠️ → propuesta P1 en [12 §12](12-testing-and-quality.md)) |
| **Documento** | [06 §13](06-deep-code-explanation.md) |
| **Estado** | ⚠️ |

## 12. Alertas, veredicto e incidentes

| Elemento | Detalle |
|---|---|
| **Funcionalidad** | Priorizar hallazgos, fijar el semáforo y derivar un incidente con evidencia |
| **Interfaz** | Sección Resumen · `rootcause status`, `rootcause incidents` |
| **Módulo** | `src/services/rules.rs` |
| **Funciones** | `build_alerts`, `derive_incident`, `incident_evidence`, `probable_causes`, `recommended_actions` |
| **Persistencia** | `snapshots.alerts_json`, tabla `incidents` |
| **Pruebas** | `las_alertas_se_ordenan_por_severidad_y_se_recortan`, `tcc_ilegible_produce_alerta_explicativa`, `una_captura_sana_no_genera_incidente`, `una_anomalia_baja_no_genera_incidente`, `una_anomalia_alta_genera_incidente_con_causa_y_accion`, `la_huella_agrupa_incidentes_equivalentes`, `dedupe_conserva_el_orden_de_aparicion` |
| **Documento** | [06 §11](06-deep-code-explanation.md) |
| **Estado** | ✅ |

## 13. Salud del propio agente

| Elemento | Detalle |
|---|---|
| **Funcionalidad** | Detectar cierre abrupto, cambio de configuración y reinicios repetidos |
| **Requisito** | REQ-SEC-002 |
| **Interfaz** | Tarjeta en Resumen · sección 10 del reporte |
| **Módulo** | `src/services/resilience.rs` + `inspector::apply_agent_health` |
| **Funciones** | `ResilienceMonitor::new`, `heartbeat`, `shutdown`, `config_fingerprint` |
| **Persistencia** | `rootcause-agent-state.json` y tabla `audit_log` |
| **Pruebas** | `la_huella_de_una_config_inexistente_es_estable`, `el_estado_viaja_a_json_y_vuelve`, `un_estado_vacio_no_rompe_la_deserializacion`, `un_agente_degradado_eleva_el_veredicto`, `un_agente_sano_no_agrega_ruido` |
| **Documento** | [06 §15](06-deep-code-explanation.md) |
| **Estado** | ✅ |

## 14. Acciones del usuario

| Acción | Interfaz | Función | Guarda | Auditoría | Prueba |
|---|---|---|---|---|---|
| Finalizar proceso | Procesos · `kill <PID>` | `terminate_process` | `manual_actions_enabled`, PID propio, 13 protegidos | `terminate-process` | `kill_sin_pid_pide_uso_correcto` (parcial) |
| Revelar en Finder | Persistencia | `reveal_in_finder` | — | `reveal-in-finder` | — |
| Sugerir bloqueo de IP | Conexiones · `block-ip <IP>` | `suggest_block_ip` | Extracción de IP válida | `suggest-block-ip` | — |
| Limpiar cachés | Almacenamiento · `clean-caches --yes` | `clean_caches` | Dos pasos + `dry_run` | `clean-caches` | `el_simulacro_nunca_marca_borrado_real` |
| Aceptar baseline (×4) | Cuatro secciones · `--accept` | `accept_*_baseline` | TCC exige legibilidad | `accept-*-baseline` | — |
| Exportar captura | `⌘E` · `export`, `snapshot` | `export_snapshot` | — | — | — |
| Generar reporte | `⌘R` · `report` | `generate_report` | — | `generate-report` | `el_reporte_incluye_veredicto_y_secciones` |
| Copia del historial | Historial · `history --backup` | `export_history_backup` | — | — | — |
| Consultar login items | Persistencia · `--login-items` | `login_items` | Permiso de Automatización | — | ❓ |
| Consultar la IA | `ai explain-latest` | `explain_latest_incident_with_ai` | 3 guardas | `ai-explain-latest` | `la_ia_desactivada_falla_sin_tocar_la_red` |

## 15. Salidas del producto

| Salida | Función | Destino | Prueba |
|---|---|---|---|
| Interfaz gráfica | `app.rs::update` y 12 `draw_*` | Pantalla | `hay_doce_secciones_y_ninguna_repetida`, `todas_las_secciones_tienen_subtitulo` |
| Consola | `cli.rs::cmd_*` | `stdout` | `la_ayuda_y_la_version_siempre_salen_bien`, `un_comando_desconocido_devuelve_error` |
| JSON | `serde_json` sobre los modelos | Archivo o `stdout` | `la_config_viaja_a_json_y_vuelve` |
| Reporte Markdown | `report::build_report`, `save_report` | `~/Documents/RootCause/reports/` | 4 tests de `report.rs` |
| Historial | `persistence::persist_snapshot` | Tabla `snapshots` | ⚠️ sin test |
| Notificación | `macos::notify` | Centro de notificaciones | ⚠️ sin test |
| Petición IA | `ai::post_json` | Proveedor configurado | Guardas probadas; la petición no (deliberado) |

## 16. Configuración → efecto

| Sección de configuración | Consumida por | Documentada en |
|---|---|---|
| `collection` | `inspector.rs`, `app.rs` | [10 §5](10-configuration.md) |
| `thresholds.process` | `rules::classify_process` | [10 §6](10-configuration.md) |
| `thresholds.cache` | `temp_scan::severity_for_size` | [10 §7](10-configuration.md) |
| `thresholds.xprotect` | `security::age_severity` | [10 §7](10-configuration.md) |
| `anomaly` | `AnomalyTracker::analyze`, `detect_*_changes` | [10 §8](10-configuration.md) |
| `alerting.max_alerts` | `rules::build_alerts`, `apply_agent_health` | [10 §9](10-configuration.md) |
| `alerting.notify_on_critical` | `notify_if_critical` | [10 §9](10-configuration.md) |
| `alerting.notification_cooldown_secs` | **Nadie** | [15 R-04](15-risks-and-technical-debt.md) |
| `remediation.manual_actions_enabled` | `terminate_process` | [10 §10](10-configuration.md) |
| `remediation.automatic_actions_enabled` | **Nadie, por diseño** | [10 §10](10-configuration.md) |
| `resilience` (salvo `stale_after_secs`) | `ResilienceMonitor` | [10 §11](10-configuration.md) |
| `resilience.stale_after_secs` | **Nadie** | [15 R-04](15-risks-and-technical-debt.md) |
| `ai` | `AiAdvisor` | [10 §12](10-configuration.md) |
| `ui.language`, `ui.theme` | `app.rs`, `i18n.rs` | [10 §13](10-configuration.md) |
| `ui.daily_report` | **Nadie** | [15 R-04](15-risks-and-technical-debt.md) |
| `anomaly.suspicious_parent_names` | **Nadie** | [15 R-04](15-risks-and-technical-debt.md) |
| `anomaly.shell_interpreters` | **Nadie** | [15 R-04](15-risks-and-technical-debt.md) |

## 17. Requisitos declarados → implementación

| Requisito | Qué pide | Implementación | Estado |
|---|---|---|---|
| [REQ-SEC-001](../requirements/REQ-SEC-001-deteccion-comportamiento-anomalo.md) | Detección de comportamiento anómalo | `services/anomaly.rs`, 8 heurísticas + `rules.rs` | ✅ |
| [REQ-SEC-002](../requirements/REQ-SEC-002-autoproteccion-y-resiliencia.md) | Autoprotección y resiliencia del agente | `services/resilience.rs` + `apply_agent_health` | ⚠️ Parcial y declarado: no protege contra root |
| [REQ-SEC-003](../requirements/REQ-SEC-003-superficies-nativas-macos.md) | Cobertura de superficies nativas de macOS | `launchd.rs`, `security.rs`, `tcc.rs`, `netscan.rs` | ✅ |

## 18. Resumen de cobertura

| Área | Funcionalidades | Con prueba directa | Estado global |
|---|---:|---:|---|
| Superficies de recolección | 7 | 7 (parciales en el acceso real al sistema) | ✅ |
| Heurísticas | 8 | 5 | ⚠️ |
| Motor de baseline | 1 | 1 (sin la propiedad pegajosa) | ⚠️ |
| Alertas e incidentes | 1 | 1 | ✅ |
| Salud del agente | 1 | 1 | ✅ |
| Acciones del usuario | 10 | 3 | ⚠️ |
| Salidas | 7 | 4 | ⚠️ |
| Configuración | 8 secciones | 1 | ⚠️ |

Los huecos con nombre y apellido están priorizados en
[12 · Pruebas y calidad §12](12-testing-and-quality.md).

---

**Fin del conjunto documental.** Vuelta al [índice](README.md).
