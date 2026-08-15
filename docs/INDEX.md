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

## Evolución

| Documento | Para qué |
|---|---|
| [`REQUIREMENTS.md`](REQUIREMENTS.md) | Requisitos funcionales y de entorno |
| [`ROADMAP.md`](ROADMAP.md) | Qué viene después y en qué orden |
| [`requirements/README.md`](requirements/README.md) | Registro permanente de requerimientos de seguridad |
| [`requirements/REQ-SEC-001-deteccion-comportamiento-anomalo.md`](requirements/REQ-SEC-001-deteccion-comportamiento-anomalo.md) | Detección de comportamiento anómalo |
| [`requirements/REQ-SEC-002-autoproteccion-y-resiliencia.md`](requirements/REQ-SEC-002-autoproteccion-y-resiliencia.md) | Autoprotección y resiliencia del agente |
| [`requirements/REQ-SEC-003-superficies-nativas-macos.md`](requirements/REQ-SEC-003-superficies-nativas-macos.md) | Cobertura de superficies nativas de macOS |
