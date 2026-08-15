#!/usr/bin/env bash
# Orquestador de release de RootCause macOS Inspector.
#
# Encadena en un solo comando todo lo que hay que hacer para publicar una
# versión: validar, compilar universal, empaquetar .app y .dmg, calcular
# hashes, verificar que los artefactos existen de verdad y —si se pide—
# etiquetar y publicar la release en GitHub.
#
#   ./scripts/release-product.sh                        # construye dist/ y para ahí
#   ./scripts/release-product.sh --verify-environment   # verifica el entorno primero
#   ./scripts/release-product.sh --publish              # además etiqueta y publica
#   ./scripts/release-product.sh --publish --watch      # y espera al workflow
#   ./scripts/release-product.sh --publish --tag-only   # empuja el tag; construye la CI
#   ./scripts/release-product.sh --skip-checks          # salta fmt/clippy/tests
#
# Nada de lo que hace es irreversible hasta `--publish`, y esa fase comprueba
# seis condiciones antes de tocar el remoto.
set -euo pipefail

cd "$(dirname "$0")/.."

# macOS siembra archivos AppleDouble `._*` en volúmenes no nativos; ensucian el
# .app y el .dmg. Desactivarlos aquí evita tener que limpiarlos después.
export COPYFILE_DISABLE=1

VERIFICAR_ENTORNO=0
PUBLICAR=0
VIGILAR=0
SALTAR_CHECKS=0
SOLO_ETIQUETA=0

for argumento in "$@"; do
  case "$argumento" in
    --verify-environment) VERIFICAR_ENTORNO=1 ;;
    --publish)            PUBLICAR=1 ;;
    --watch)              VIGILAR=1 ;;
    --skip-checks)        SALTAR_CHECKS=1 ;;
    --tag-only)           SOLO_ETIQUETA=1 ;;
    -h|--help)
      sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Opción desconocida: $argumento" >&2
      echo "Usa --help para ver las opciones." >&2
      exit 2
      ;;
  esac
done

DIST="dist"
REPO="vladimiracunadev-create/rootcause-macos-inspector"
VERSION=$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)
TAG="v$VERSION"

APP="$DIST/RootCause.app"
DMG="$DIST/RootCause-$VERSION.dmg"
ZIP="$DIST/RootCause-app.zip"
SUMS="$DIST/SHA256SUMS.txt"
NOTAS="$DIST/RELEASE_NOTES.md"

paso()  { printf '\n\033[0;34m▶ %s\033[0m\n' "$1"; }
ok()    { printf '\033[0;32m  ✓ %s\033[0m\n' "$1"; }
fatal() { printf '\033[0;31m  ✗ %s\033[0m\n' "$1" >&2; exit 1; }

printf '\033[1mRootCause macOS Inspector · release %s\033[0m\n' "$TAG"

# ── 1 · Entorno ─────────────────────────────────────────────────────────────
if [[ "$VERIFICAR_ENTORNO" -eq 1 ]]; then
  paso "Verificando el entorno"
  ./scripts/verify-environment.sh || fatal "El entorno no cumple los requisitos"
fi

# ── 2 · Validación ──────────────────────────────────────────────────────────
if [[ "$SALTAR_CHECKS" -eq 1 ]]; then
  paso "Validación saltada por --skip-checks"
  printf '\033[0;33m  ! Publicar sin validar es tu decisión; la CI la hará igualmente.\033[0m\n'
else
  paso "Validando (formato, clippy, tests, ambas ediciones)"
  REGISTRO=$(mktemp "${TMPDIR:-/tmp}/rootcause-ci.XXXXXX.log")
  ./scripts/ci-local.sh > "$REGISTRO" 2>&1 || {
    tail -30 "$REGISTRO" >&2
    fatal "La validación falló. Registro completo en $REGISTRO"
  }
  rm -f "$REGISTRO"
  ok "Formato, clippy, tests y ambas ediciones en verde"
fi

# ── 3 · Empaquetado ─────────────────────────────────────────────────────────
paso "Construyendo el .app universal (arm64 + x86_64)"
./scripts/package-app.sh --universal
ok "$APP"

paso "Construyendo el .dmg"
./scripts/package-dmg.sh
ok "$DMG"

paso "Comprimiendo el .app"
rm -f "$ZIP"
(cd "$DIST" && zip -qry "$(basename "$ZIP")" "$(basename "$APP")")
(cd "$DIST" && shasum -a 256 "$(basename "$ZIP")" >> "$(basename "$SUMS")")
ok "$ZIP"

# ── 4 · Verificación de artefactos ──────────────────────────────────────────
# Un artefacto vacío o ausente que llega a una release es peor que un fallo de
# build: nadie lo descubre hasta que alguien intenta usarlo.
paso "Verificando los artefactos"

for artefacto in "$DMG" "$ZIP" "$SUMS"; do
  [[ -f "$artefacto" ]] || fatal "Falta $artefacto"
  [[ -s "$artefacto" ]] || fatal "$artefacto está vacío"
  ok "$(basename "$artefacto") · $(du -h "$artefacto" | cut -f1)"
done

BINARIO="$APP/Contents/MacOS/rootcause"
[[ -x "$BINARIO" ]] || fatal "El binario del bundle no es ejecutable"

ARQUITECTURAS=$(lipo -info "$BINARIO" | sed -E 's/.*(are|is architecture): //')
ES_UNIVERSAL=0
if grep -q "arm64" <<<"$ARQUITECTURAS" && grep -q "x86_64" <<<"$ARQUITECTURAS"; then
  ES_UNIVERSAL=1
  ok "Binario universal: $ARQUITECTURAS"
else
  printf '\033[0;33m  ! Binario NO universal (%s)\033[0m\n' "$ARQUITECTURAS"
  printf '    Instala rustup para construirlo en local, o publica con --tag-only\n'
  printf '    y deja que el workflow release-macos genere el universal.\n'
fi

VERSION_BINARIO=$("$BINARIO" --version | awk '{print $NF}' | tr -d 'v')
[[ "$VERSION_BINARIO" == "$VERSION" ]] \
  || fatal "El binario reporta v$VERSION_BINARIO pero Cargo.toml dice v$VERSION"
ok "El binario reporta la versión esperada (v$VERSION)"

VERSION_PLIST=$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" \
  "$APP/Contents/Info.plist" 2>/dev/null || echo "?")
[[ "$VERSION_PLIST" == "$VERSION" ]] \
  || fatal "El Info.plist dice v$VERSION_PLIST pero Cargo.toml dice v$VERSION"
ok "El Info.plist declara la versión esperada"

# ── 5 · Notas de la release ─────────────────────────────────────────────────
paso "Escribiendo las notas de la release"

if [[ "$ES_UNIVERSAL" -eq 1 ]]; then
  NOTA_ARQUITECTURA="El binario es **universal**: funciona en Apple Silicon y en Intel."
else
  NOTA_ARQUITECTURA="Arquitectura del artefacto local: **$ARQUITECTURAS**."
fi
cat > "$NOTAS" <<NOTAS_EOF
## RootCause macOS Inspector $TAG

Monitor forense ligero para macOS, escrito en Rust. Observa LaunchAgents y
LaunchDaemons, procesos, Gatekeeper, XProtect, permisos TCC, red y persistencia,
y explica la causa raíz con evidencia.

> Diagnóstico primero. Intervención después.

### Artefactos

| Archivo | Contenido |
|---|---|
| \`RootCause-$VERSION.dmg\` | Imagen de disco con la app y enlace a Aplicaciones |
| \`RootCause-app.zip\` | Bundle \`.app\` comprimido |
| \`SHA256SUMS.txt\` | Hashes de integridad |

$NOTA_ARQUITECTURA

### Instalación

1. Abre el \`.dmg\` y arrastra \`RootCause.app\` a Aplicaciones.
2. La primera vez, macOS avisará de que la app no está firmada ni notarizada.
   Autorízala en Ajustes del Sistema → Privacidad y seguridad.
3. Para auditar los permisos de privacidad (TCC), concédele Acceso total al
   disco. Sin ese permiso el resto de la app funciona igual, y esa sección lo dirá.

La alternativa recomendada es compilarla desde el código: \`cargo build --release\`.

### Verificar la integridad

\`\`\`bash
shasum -a 256 -c SHA256SUMS.txt
\`\`\`

### Privacidad

Todo el análisis es local. No hay servidor, ni cuenta, ni telemetría. El
adaptador de IA opcional viene apagado y solo enviaría el incidente ya resumido.

### Documentación

- [Manual de usuario](https://github.com/$REPO/blob/main/docs/MANUAL_USUARIO.md)
- [Qué detecta y qué no](https://github.com/$REPO/blob/main/docs/DETECCION_AMENAZAS.md)
- [Limitaciones honestas](https://github.com/$REPO/blob/main/docs/LIMITACIONES.md)
NOTAS_EOF
ok "$NOTAS"

# ── 6 · Resumen ─────────────────────────────────────────────────────────────
paso "Artefactos listos en $DIST/"
cat "$SUMS"

if [[ "$PUBLICAR" -eq 0 ]]; then
  printf '\n\033[0;32m✓ Release %s construida y verificada.\033[0m\n' "$TAG"
  printf '  Para publicarla:  ./scripts/release-product.sh --publish\n'
  exit 0
fi

# ── 7 · Publicación ─────────────────────────────────────────────────────────
paso "Comprobaciones previas a publicar"

if [[ "$ES_UNIVERSAL" -eq 0 && "$SOLO_ETIQUETA" -eq 0 ]]; then
  fatal "No se publican artefactos de una sola arquitectura. Usa --tag-only para que los construya la CI, o instala rustup."
fi

command -v gh >/dev/null 2>&1 || fatal "gh no está instalado (brew install gh)"
gh auth status >/dev/null 2>&1 || fatal "gh no tiene sesión. Ejecuta: gh auth login"
ok "gh autenticado"

RAMA=$(git rev-parse --abbrev-ref HEAD)
[[ "$RAMA" == "main" ]] || fatal "Estás en la rama '$RAMA'; las releases salen de main"
ok "Rama main"

[[ -z "$(git status --porcelain)" ]] || fatal "Hay cambios sin commitear"
ok "Árbol de trabajo limpio"

git fetch --tags --quiet 2>/dev/null || true
if git rev-parse "$TAG" >/dev/null 2>&1; then
  fatal "La etiqueta $TAG ya existe. Sube la versión en Cargo.toml"
fi
ok "La etiqueta $TAG está libre"

if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
  fatal "La release $TAG ya existe en GitHub"
fi
ok "No hay una release $TAG previa"

paso "Etiquetando y publicando $TAG"
git tag -a "$TAG" -m "RootCause macOS Inspector $TAG"
git push origin "$TAG"
ok "Etiqueta empujada"

if [[ "$SOLO_ETIQUETA" -eq 1 ]]; then
  ok "Solo etiqueta: el workflow release-macos construirá y publicará los artefactos"
else
  gh release create "$TAG" \
    --repo "$REPO" \
    --title "RootCause macOS Inspector $TAG" \
    --notes-file "$NOTAS" \
    "$DMG" "$ZIP" "$SUMS"
  ok "Release publicada con los artefactos locales"
fi

if [[ "$VIGILAR" -eq 1 ]]; then
  paso "Esperando al workflow release-macos"
  sleep 5
  EJECUCION=$(gh run list --repo "$REPO" --workflow release-macos.yml \
    --limit 1 --json databaseId --jq '.[0].databaseId' 2>/dev/null || echo "")
  if [[ -n "$EJECUCION" ]]; then
    gh run watch "$EJECUCION" --repo "$REPO" --exit-status \
      && ok "Workflow en verde" \
      || fatal "El workflow release-macos falló"
  else
    printf '\033[0;33m  ! No se encontró la ejecución del workflow todavía.\033[0m\n'
  fi
fi

printf '\n\033[0;32m✓ Release %s publicada.\033[0m\n' "$TAG"
printf '  https://github.com/%s/releases/tag/%s\n' "$REPO" "$TAG"
