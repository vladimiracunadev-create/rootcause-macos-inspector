# Lista de verificación de release

## 1 · Código

- [ ] `cargo fmt --all -- --check` sin cambios pendientes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` limpio
- [ ] `cargo test --all-features` en verde
- [ ] `cargo build --release` sin advertencias
- [ ] `cargo build --release --no-default-features` (edición CLI-only) compila

O de una vez:

```bash
./scripts/ci-local.sh
```

## 2 · Versión

- [ ] `version` actualizada en `Cargo.toml`
- [ ] `Cargo.lock` regenerado y commiteado
- [ ] Insignia de versión actualizada en `README.md`
- [ ] Entrada de la versión en `docs/ROADMAP.md`

La versión del `.app` y del `.dmg` se toma de `Cargo.toml` automáticamente: no hay que tocarla en
dos sitios.

## 3 · Documentación

- [ ] `README.md` refleja lo que hace realmente esta versión
- [ ] `docs/COMMANDS.md` incluye cualquier comando nuevo
- [ ] `docs/LIMITACIONES.md` recoge cualquier límite nuevo
- [ ] Los enlaces internos funcionan

## 4 · Prueba manual

- [ ] La GUI abre y muestra las 12 secciones
- [ ] La primera captura siembra la baseline sin generar alertas de cambio
- [ ] Persistencia detecta una entrada de prueba como NUEVA y la olvida al aceptar
- [ ] `rootcause status`, `security`, `persistence`, `tcc`, `network` responden
- [ ] `rootcause report` genera un Markdown legible
- [ ] La ventana no se congela durante un escaneo profundo de red
- [ ] Probado en Apple Silicon **y**, si es posible, en Intel

Guion completo → [`TESTING_MACOS.md`](TESTING_MACOS.md).

## 5 · Empaquetado

- [ ] `./scripts/package-app.sh --universal` genera el `.app`
- [ ] `lipo -info dist/RootCause.app/Contents/MacOS/rootcause` lista ambas arquitecturas
- [ ] `./scripts/package-dmg.sh` genera el `.dmg`
- [ ] El `.dmg` monta y el `.app` arranca desde él
- [ ] `dist/SHA256SUMS.txt` generado

## 6 · Publicación

```bash
# Todo en un comando: valida, empaqueta, verifica, etiqueta y publica
./scripts/release-product.sh --publish --watch
```

O a mano:

```bash
git tag v0.1.0
git push origin v0.1.0
```

- [ ] El workflow `release-macos` termina en verde
- [ ] La release de GitHub contiene `.dmg`, `.app` comprimido y `SHA256SUMS.txt`
- [ ] Las notas de la release explican los cambios en lenguaje claro

## 7 · Después

- [ ] `README.md` de la landing revisado si cambió algo visible
- [ ] Insignia de CI en verde en la portada del repositorio
- [ ] Abrir los issues de seguimiento de lo que quedó fuera
