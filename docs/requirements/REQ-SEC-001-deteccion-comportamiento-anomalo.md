# REQ-SEC-001 · Detección de comportamiento anómalo

| Campo | Valor |
|---|---|
| Prioridad | Alta |
| Estado | V1 implementada |
| Módulos | `services/anomaly.rs`, `services/rules.rs`, `services/baseline.rs` |

## Problema

Las defensas por firma solo reconocen lo que ya conocen. Una amenaza nueva, un binario legítimo
secuestrado o un uso indebido de una herramienta de administración no aparecen en ninguna base de
firmas — pero **sí alteran el comportamiento observable del equipo**.

Hace falta una capa que detecte esa alteración sin necesidad de saber qué la causa.

## Requerimiento

RootCause debe detectar, correlacionar y explicar señales de comportamiento anómalo, con evidencia
técnica y una acción recomendada, sin depender de firmas ni de servicios externos.

## Criterios de aceptación

| # | Criterio | Estado |
|---|---|---|
| 1 | Detectar consumo de CPU sostenido (no picos aislados) | ✅ |
| 2 | Detectar crecimiento sostenido de memoria | ✅ |
| 3 | Detectar escritura agresiva sostenida | ✅ |
| 4 | Detectar tráfico saliente a múltiples destinos públicos | ✅ |
| 5 | Detectar barrido de la red local | ✅ |
| 6 | Detectar ejecución desde rutas inusuales | ✅ |
| 7 | Detectar binarios sin firma fuera de las rutas del sistema | ✅ |
| 8 | Detectar procesos que se relanzan repetidamente | ✅ |
| 9 | Cada evento incluye severidad, puntaje, evidencia, hipótesis y acción | ✅ |
| 10 | Los eventos se correlacionan en un incidente resumido y persistido | ✅ |
| 11 | Los umbrales son configurables sin recompilar | ✅ |
| 12 | Existe una lista de confianza que evita el ruido del software del sistema | ✅ |

## Diseño

Dos principios, ambos verificados por tests:

- **Sostenido, no instantáneo.** El detector guarda historial por PID entre capturas. Una racha
  rota reinicia el contador.
- **Correlación antes que sospecha.** Cada señal suma puntaje; el veredicto sale de la suma.

Detalle de umbrales → [`../HEURISTICAS.md`](../HEURISTICAS.md).

## Fuera de alcance

- Identificación de familias de malware.
- Intercepción en tiempo real (requeriría Endpoint Security Framework).
- Detección de exfiltración lenta o por canales encubiertos.
- Análisis del contenido del tráfico de red.

## Evolución prevista

- Correlación entre superficies distintas (un mismo binario con persistencia nueva **y** tráfico
  inusual debería puntuar más que la suma de ambos por separado).
- Ajuste automático de umbrales según el perfil observado del equipo.
