# Integración continua

Tres workflows en [`.github/workflows/`](../.github/workflows/).

## `ci.yml` — validación en cada cambio

**Se dispara con:** push y pull request a `main`, y manualmente.

**Corre en:** `macos-latest`.

| Paso | Comando | Por qué |
|---|---|---|
| Formato | `cargo fmt --all -- --check` | El formato no se discute en las revisiones |
| Análisis estático | `cargo clippy --all-targets --all-features -- -D warnings` | Ninguna advertencia llega a `main` |
| Tests | `cargo test --all-features` | La lógica de clasificación y parseo está cubierta |
| Build CLI-only | `cargo build --release --no-default-features` | La edición sin GUI no puede romperse en silencio |
| Build completo | `cargo build --release` | El artefacto real |

Los tests corren en un runner de macOS de verdad, así que las funciones que tocan el sistema
(`environment()`, rutas de launchd) se ejercitan en su plataforma real.

## `release-macos.yml` — publicación

**Se dispara con:** una etiqueta `v*` (por ejemplo `v0.1.0`), o manualmente.

Pasos:

1. Instala los targets `aarch64-apple-darwin` y `x86_64-apple-darwin`.
2. Compila release para ambos.
3. Une los binarios con `lipo` en uno universal.
4. Construye `RootCause.app` y `RootCause-<versión>.dmg`.
5. Calcula `SHA256SUMS.txt`.
6. Publica una release de GitHub con todos los artefactos.

```bash
# Publicar una versión
git tag v0.1.0
git push origin v0.1.0
```

## `deploy-landing.yml` — página del producto

**Se dispara con:** push a `main` que toque `landing/**`, o manualmente.

Publica el contenido de `landing/` en GitHub Pages. Requiere que Pages esté configurado en el
repositorio con origen **GitHub Actions**.

## Réplica local

```bash
./scripts/ci-local.sh
```

Ejecuta exactamente los mismos pasos que `ci.yml`, en el mismo orden. Si pasa en local, pasa en la
CI — salvo por diferencias del entorno del runner.

Para el ciclo completo de release (validar, empaquetar, verificar y publicar) hay un orquestador
que encadena todo:

```bash
./scripts/release-product.sh --publish --watch
```

## Qué NO hace la CI

- No firma ni notariza: eso requiere credenciales de Apple Developer.
- No prueba la interfaz gráfica: un runner sin sesión gráfica no puede abrir una ventana.
- No sustituye la prueba manual en un Mac real con los permisos concedidos.
