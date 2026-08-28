# Índice de documentación

Toda la documentación de RootCause macOS Inspector, agrupada por para qué sirve.

## Empezar

| Documento | Para qué |
|---|---|
| [`GUIA_DE_USO_PREVIA.md`](GUIA_DE_USO_PREVIA.md) | Léelo **antes** de la primera ejecución |
| [`MANUAL_USUARIO.md`](MANUAL_USUARIO.md) | Manual completo: qué hace cada sección y cómo leer los resultados |
| [`MANUAL_PARA_NOVATOS.md`](MANUAL_PARA_NOVATOS.md) | Versión sin jerga, para quien no es técnico |
| [`COMMANDS.md`](COMMANDS.md) | Referencia completa de la CLI |
| [`PERMISOS_MACOS.md`](PERMISOS_MACOS.md) | Qué permisos pide, por qué y cómo concederlos |

## Entender el producto

| Documento | Para qué |
|---|---|
| [`PLAN_MAESTRO.md`](PLAN_MAESTRO.md) | La tesis, los principios y por qué el producto es como es |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Cómo está construido y por qué así |
| [`MODULO_DETECCION_ANOMALIAS.md`](MODULO_DETECCION_ANOMALIAS.md) | El detector de comportamiento, tal como está implementado |
| [`HEURISTICAS.md`](HEURISTICAS.md) | Cada heurística, su umbral y su justificación |
| [`PERSISTENCIA_MACOS.md`](PERSISTENCIA_MACOS.md) | Cómo funciona la persistencia en macOS y qué se vigila |
| [`DETECCION_AMENAZAS.md`](DETECCION_AMENAZAS.md) | Mapa honesto: qué detecta y qué no, amenaza por amenaza |
| [`LIMITACIONES.md`](LIMITACIONES.md) | Los límites del producto, escritos sin adornos |

## Arquitectura en profundidad

| Documento | Para qué |
|---|---|
| [`ARQUITECTURA_ESCALABILIDAD.md`](ARQUITECTURA_ESCALABILIDAD.md) | Qué aguanta el diseño, dónde está el techo y qué habría que cambiar |
| [`ARQUITECTURA_EVOLUTIVA.md`](ARQUITECTURA_EVOLUTIVA.md) | Cómo crecer sin reescribir: los tres puntos de extensión |
| [`RUST_PARA_ROOTCAUSE.md`](RUST_PARA_ROOTCAUSE.md) | Por qué Rust y cómo se usa el lenguaje aquí |

## Construir y distribuir

| Documento | Para qué |
|---|---|
| [`BUILD_MACOS.md`](BUILD_MACOS.md) | Compilar desde el código fuente |
| [`PACKAGING_MACOS.md`](PACKAGING_MACOS.md) | Generar el `.app`, el `.dmg` y los hashes |
| [`CI_GITHUB.md`](CI_GITHUB.md) | Qué hace cada workflow de GitHub Actions |
| [`TESTING_MACOS.md`](TESTING_MACOS.md) | Cómo se prueba, automática y manualmente |
| [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md) | Lista de verificación antes de publicar |

## Operar

| Documento | Para qué |
|---|---|
| [`OPERACION.md`](OPERACION.md) | Uso diario, baselines y rutinas recomendadas |
| [`TROUBLESHOOTING.md`](TROUBLESHOOTING.md) | Problemas frecuentes y su solución |
| [`POLITICA_DE_PRIVACIDAD_LOCAL.md`](POLITICA_DE_PRIVACIDAD_LOCAL.md) | Qué datos se guardan, dónde y qué nunca sale del equipo |

## Producto y marca

| Documento | Para qué |
|---|---|
| [`CATALOGO_PRODUCTO.md`](CATALOGO_PRODUCTO.md) | Qué es edición, artefacto, adaptador y futuro declarado |
| [`FAMILIA_ROOTCAUSE.md`](FAMILIA_ROOTCAUSE.md) | Las cuatro ediciones: Windows, macOS, Web y Mobile |
| [`MARCA_Y_BRANDING_ROOTCAUSE.md`](MARCA_Y_BRANDING_ROOTCAUSE.md) | Identidad visual y verbal, y las reglas del semáforo |
| [`NOMBRES_PRODUCTO.md`](NOMBRES_PRODUCTO.md) | La decisión sobre el nombre, con sus reservas |
| [`LICENCIA_Y_DECISION.md`](LICENCIA_Y_DECISION.md) | Por qué Apache 2.0 y por qué no se distribuyen binarios firmados |
| [`COMPARATIVA_OSS.md`](COMPARATIVA_OSS.md) | Frente a KnockKnock, osquery, Santa, LuLu, Velociraptor… |
| [`GUIA_COMPETENCIA.md`](GUIA_COMPETENCIA.md) | Cuándo se puede tomar código de otro proyecto y cuándo solo la idea |
| [`RECLUTADORES.md`](RECLUTADORES.md) | Qué demuestra el proyecto, para evaluación técnica |
| [`REPOSITORIO_ANALISIS.md`](REPOSITORIO_ANALISIS.md) | Qué hay dentro, con su deuda técnica reconocida |

## Documentación de sistema

Conjunto completo de documentación técnica verificada contra el código, con versión en PDF.

| Documento | Para qué |
|---|---|
| [`system-documentation/README.md`](system-documentation/README.md) | Portada e índice de los 20 documentos |
| [`01-system-overview.md`](system-documentation/01-system-overview.md) | Qué es el sistema, con explicación para lectores no técnicos |
| [`02-installation-and-execution.md`](system-documentation/02-installation-and-execution.md) | De repositorio clonado a binario funcionando |
| [`03-architecture.md`](system-documentation/03-architecture.md) | Capas, patrones y diagramas |
| [`04-code-map.md`](system-documentation/04-code-map.md) | Inventario archivo por archivo |
| [`05-technical-reference.md`](system-documentation/05-technical-reference.md) | Catálogo de tipos, funciones, comandos y errores |
| [`06-deep-code-explanation.md`](system-documentation/06-deep-code-explanation.md) | Cómo funciona cada módulo por dentro |
| [`07-database.md`](system-documentation/07-database.md) | Esquema SQLite y diccionario de datos |
| [`08-data-flow.md`](system-documentation/08-data-flow.md) | Un dato, de la utilidad de macOS al incidente |
| [`09-apis-and-integrations.md`](system-documentation/09-apis-and-integrations.md) | CLI como API, utilidades nativas e IA opcional |
| [`10-configuration.md`](system-documentation/10-configuration.md) | `rootcause-config.json` campo por campo |
| [`11-security.md`](system-documentation/11-security.md) | Seguridad del propio producto |
| [`12-testing-and-quality.md`](system-documentation/12-testing-and-quality.md) | Los 112 tests y los huecos priorizados |
| [`13-deployment-and-operations.md`](system-documentation/13-deployment-and-operations.md) | Construcción, release y operación diaria |
| [`14-troubleshooting.md`](system-documentation/14-troubleshooting.md) | Síntoma → causa → diagnóstico → solución |
| [`15-risks-and-technical-debt.md`](system-documentation/15-risks-and-technical-debt.md) | 24 hallazgos clasificados |
| [`16-glossary.md`](system-documentation/16-glossary.md) | Términos explicados sin jerga |
| [`17-executive-summary.md`](system-documentation/17-executive-summary.md) | El sistema para tomar decisiones |
| [`18-new-developer-guide.md`](system-documentation/18-new-developer-guide.md) | Itinerario de incorporación |
| [`19-traceability-matrix.md`](system-documentation/19-traceability-matrix.md) | Funcionalidad → función → dato → prueba |
| [`pdf/`](system-documentation/pdf/) | Los 20 documentos en PDF |

## Evolución

| Documento | Para qué |
|---|---|
| [`REQUIREMENTS.md`](REQUIREMENTS.md) | Requisitos funcionales y de entorno |
| [`ROADMAP.md`](ROADMAP.md) | Qué viene después y en qué orden |
| [`requirements/README.md`](requirements/README.md) | Registro permanente de requerimientos de seguridad |
| [`requirements/REQ-SEC-001-deteccion-comportamiento-anomalo.md`](requirements/REQ-SEC-001-deteccion-comportamiento-anomalo.md) | Detección de comportamiento anómalo |
| [`requirements/REQ-SEC-002-autoproteccion-y-resiliencia.md`](requirements/REQ-SEC-002-autoproteccion-y-resiliencia.md) | Autoprotección y resiliencia del agente |
| [`requirements/REQ-SEC-003-superficies-nativas-macos.md`](requirements/REQ-SEC-003-superficies-nativas-macos.md) | Cobertura de superficies nativas de macOS |
