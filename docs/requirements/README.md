# Registro permanente de requerimientos

Los requerimientos que definen la evolución del producto más allá de una versión concreta. Cada uno
tiene estado, prioridad y trazabilidad con el roadmap.

| ID | Título | Prioridad | Estado |
|---|---|---|---|
| [REQ-SEC-001](REQ-SEC-001-deteccion-comportamiento-anomalo.md) | Detección de comportamiento anómalo | Alta | V1 implementada |
| [REQ-SEC-002](REQ-SEC-002-autoproteccion-y-resiliencia.md) | Autoprotección y resiliencia del agente | Media | Base inicial implementada |
| [REQ-SEC-003](REQ-SEC-003-superficies-nativas-macos.md) | Cobertura de superficies nativas de macOS | Alta | V1 implementada |

## Estados

| Estado | Significa |
|---|---|
| Propuesto | Escrito, sin implementación |
| V1 implementada | Hay una primera versión funcional y honesta sobre sus límites |
| Base inicial implementada | Existe el andamiaje; falta profundidad |
| Completo | Cumple el requerimiento tal como está escrito |

## Cómo se escribe uno nuevo

1. Un identificador estable (`REQ-<área>-<número>`).
2. **Qué problema resuelve**, antes que qué se va a construir.
3. Criterios de aceptación verificables.
4. **Qué queda explícitamente fuera de alcance.** Esta sección es obligatoria: un requerimiento sin
   límites escritos acaba prometiendo lo que no puede cumplir.
