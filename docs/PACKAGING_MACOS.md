# Empaquetado para macOS

## Artefactos

| Artefacto | Contenido | Script |
|---|---|---|
| `RootCause.app` | Bundle de aplicación con `Info.plist` e icono | `./scripts/package-app.sh` |
| `RootCause-<versión>.dmg` | Imagen de disco con el `.app` y enlace a Aplicaciones | `./scripts/package-dmg.sh` |
| `rootcause` | Binario CLI suelto | `cargo build --release` |
| `SHA256SUMS.txt` | Hashes de integridad de todo lo anterior | Generado por `package-dmg.sh` |

Todo se deja en `dist/`.

## Todo de una vez

```bash
./scripts/release-product.sh                       # construye y verifica dist/
./scripts/release-product.sh --verify-environment  # comprueba el entorno primero
./scripts/release-product.sh --publish             # además etiqueta y publica en GitHub
./scripts/release-product.sh --publish --watch     # y espera a que el workflow acabe
./scripts/release-product.sh --publish --tag-only  # empuja el tag; los artefactos los hace la CI
```

El orquestador encadena todo lo demás en orden: entorno → validación → `.app` universal → `.dmg` →
`.zip` → hashes → verificación de artefactos → notas de la release → publicación.

La fase de publicación comprueba seis condiciones antes de tocar el remoto: `gh` autenticado, rama
`main`, árbol de trabajo limpio, etiqueta libre, release inexistente y binario universal. Y verifica
que el binario y el `Info.plist` declaren la misma versión que `Cargo.toml`, porque un artefacto con
la versión equivocada es de los errores que nadie detecta hasta que ya está publicado.

> Si compilas con un Rust instalado por Homebrew (sin `rustup`), el binario universal no se puede
> construir en local: el script lo avisa, degrada a la arquitectura nativa y te ofrece `--tag-only`
> para que el workflow `release-macos` genere los artefactos universales.

## Generar el `.app`

```bash
./scripts/package-app.sh              # arquitectura nativa
./scripts/package-app.sh --universal  # arm64 + x86_64
```

Estructura resultante:

```text
dist/RootCause.app/
└── Contents/
    ├── Info.plist          ← identidad, versión, categoría, política de red
    ├── MacOS/rootcause     ← el binario
    └── Resources/
        └── AppIcon.icns    ← icono generado desde el SVG de marca
```

El `Info.plist` vive en [`packaging/macos/Info.plist`](../packaging/macos/Info.plist) y el script
sustituye la versión desde `Cargo.toml`, de modo que no hay dos sitios donde mantenerla.

## Generar el `.dmg`

```bash
./scripts/package-dmg.sh
```

Construye el `.app` si falta, arma una imagen comprimida con un enlace a `/Applications` y escribe
`dist/SHA256SUMS.txt`.

## Firma y notarización

RootCause **no se distribuye firmado ni notarizado**. Consecuencia práctica: al abrir el `.app`
descargado, Gatekeeper mostrará un aviso, y hay que autorizarlo explícitamente en
**Ajustes del Sistema → Privacidad y seguridad**.

Es una decisión consciente y consistente con el resto del producto: se entrega el código para que
puedas compilarlo tú, en vez de un binario opaco.

Si tienes una cuenta de Apple Developer y quieres firmar tu propia compilación:

```bash
# Firmar
codesign --deep --force --options runtime \
  --sign "Developer ID Application: TU NOMBRE (TEAMID)" \
  --entitlements packaging/macos/entitlements.plist \
  dist/RootCause.app

# Verificar
codesign -dvv dist/RootCause.app
spctl --assess --type execute -vv dist/RootCause.app

# Notarizar
xcrun notarytool submit dist/RootCause-0.1.0.dmg \
  --apple-id TU_APPLE_ID --team-id TEAMID --wait
xcrun stapler staple dist/RootCause-0.1.0.dmg
```

## Homebrew

En [`packaging/homebrew/rootcause.rb`](../packaging/homebrew/rootcause.rb) hay una plantilla de
cask. Para usarla necesitas un tap propio y un `.dmg` publicado con su SHA-256:

```bash
brew tap tu-usuario/tap
brew install --cask rootcause
```

La plantilla queda como punto de partida documentado, no como canal oficial de distribución.

## Instalación manual del binario CLI

```bash
cargo build --release
sudo cp target/release/rootcause /usr/local/bin/
rootcause --help
```

Recuerda que el permiso de Acceso total al disco se aplica al proceso que lanza el comando: si usas
la CLI desde el Terminal, hay que concedérselo al Terminal.
