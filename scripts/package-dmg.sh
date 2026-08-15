#!/usr/bin/env bash
# Construye dist/RootCause-<versión>.dmg y dist/SHA256SUMS.txt.
# Si el .app no existe todavía, lo construye antes.
set -euo pipefail

cd "$(dirname "$0")/.."

DIST="dist"
APP="$DIST/RootCause.app"
VERSION=$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)
DMG="$DIST/RootCause-$VERSION.dmg"
ESCENARIO="$DIST/dmg-stage"

if [[ ! -d "$APP" ]]; then
  echo "▶ El .app no existe todavía; construyéndolo"
  ./scripts/package-app.sh "$@"
fi

echo "Construyendo $DMG …"

rm -rf "$ESCENARIO" "$DMG"
mkdir -p "$ESCENARIO"
cp -R "$APP" "$ESCENARIO/"
ln -s /Applications "$ESCENARIO/Aplicaciones"

# Un README dentro de la imagen: quien la abre no tiene por qué haber leído el
# repositorio, y el aviso de Gatekeeper conviene que lo lea antes de chocarse.
cat > "$ESCENARIO/LÉEME.txt" <<TXT
RootCause macOS Inspector v$VERSION
==================================

1. Arrastra RootCause.app a la carpeta Aplicaciones.

2. La primera vez, macOS avisará de que la app no está firmada ni notarizada.
   Es esperado: no se distribuye firmada. Autorízala en
   Ajustes del Sistema → Privacidad y seguridad → «Abrir de todos modos».

   La alternativa recomendada es compilarla tú desde el código fuente:
   https://github.com/vladimiracunadev-create/rootcause-macos-inspector

3. Para auditar los permisos de privacidad (TCC), concédele Acceso total al
   disco en Ajustes del Sistema → Privacidad y seguridad.
   Sin ese permiso el resto de la app funciona igual; solo esa sección lo dirá.

Todo el análisis es local. Ningún dato sale de tu equipo.

Licencia Apache 2.0 · Vladimir Acuña
TXT

hdiutil create \
  -volname "RootCause $VERSION" \
  -srcfolder "$ESCENARIO" \
  -ov -format UDZO \
  "$DMG" >/dev/null

rm -rf "$ESCENARIO"

# ── Hashes de integridad ─────────────────────────────────────────────────────
(
  cd "$DIST"
  : > SHA256SUMS.txt
  for archivo in *.dmg; do
    [[ -e "$archivo" ]] && shasum -a 256 "$archivo" >> SHA256SUMS.txt
  done
)

echo
echo "✓ $DMG"
du -sh "$DMG" | awk '{print "  tamaño: " $1}'
echo
cat "$DIST/SHA256SUMS.txt"
