# Catálogo del producto

Documento fuente de verdad para distinguir cuatro cosas que se confunden con facilidad:

- qué es una **edición** del producto,
- qué es un **artefacto** de distribución,
- qué es un **adaptador** sobre el binario principal,
- y qué sigue siendo **futuro** declarado.

## 1 · Definiciones

**Edición.** Forma funcional en que RootCause se ejecuta o se consume.

**Artefacto.** Archivo concreto publicado en una release (`.dmg`, `.zip`, `SHA256SUMS.txt`).

**Adaptador.** Integración que reutiliza el binario `rootcause` como motor. No reemplaza el núcleo.

**Futuro.** Algo escrito en el roadmap que todavía no existe. Se nombra para no prometerlo como si
existiera.

## 2 · Ediciones

| Edición | Estado | Cómo se compila | ¿Sale en la release? |
|---|---|---|---|
| **GUI Desktop** | Producción | `cargo build --release` | Sí, dentro del `.app` |
| **CLI-only** | Producción | `cargo build --release --no-default-features` | No como artefacto propio |
| **App bundle `.app`** | Producción | `./scripts/package-app.sh --universal` | Sí, comprimido en `.zip` |

La edición CLI-only no es un producto distinto: es el mismo binario sin `egui`. Existe para
sesiones sin pantalla, servidores y automatización.

## 3 · Artefactos oficiales

| Archivo | Contenido | Lo genera |
|---|---|---|
| `RootCause-<versión>.dmg` | Imagen con la app y enlace a Aplicaciones | `package-dmg.sh` |
| `RootCause-app.zip` | Bundle `.app` comprimido | `release-macos.yml` |
| `SHA256SUMS.txt` | Hashes de integridad de lo anterior | `package-dmg.sh` + workflow |

El binario dentro del `.app` es **universal** (arm64 + x86_64) cuando lo construye la CI.

## 4 · Adaptadores

| Adaptador | Estado | Qué es |
|---|---|---|
| **Cask de Homebrew** | Plantilla | Manifiesto en `packaging/homebrew/`. No es un canal oficial: necesita un tap propio y un `.dmg` publicado con su SHA-256. |

## 5 · Lo que NO es una edición

Cosas que a veces se confunden con productos separados:

- **El reporte forense en Markdown** es una salida, no una edición.
- **El adaptador de IA** es una función opcional apagada por defecto, no un producto.
- **La landing** es documentación, no software distribuible.

## 6 · Futuro declarado

Nada de esto existe hoy. Está en [`ROADMAP.md`](ROADMAP.md) con su justificación:

| Idea | Por qué no está |
|---|---|
| Extensión de Endpoint Security | Exige un *entitlement* concedido por Apple y cambia la naturaleza del producto |
| Firma y notarización | Requiere cuenta de Apple Developer |
| Modo desatendido con reporte diario | Falta decidir cómo se entrega sin convertirse en un LaunchAgent más |
| Motor de reglas declarativas | Falta resolver de dónde vienen las reglas y cómo se verifican |

## 7 · La familia RootCause

Este repositorio es la edición macOS. Hay tres hermanas, con el mismo posicionamiento y superficies
distintas: ver [`FAMILIA_ROOTCAUSE.md`](FAMILIA_ROOTCAUSE.md).
