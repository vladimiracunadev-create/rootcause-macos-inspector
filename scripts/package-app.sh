#!/usr/bin/env bash
# Construye dist/RootCause.app a partir del binario de release.
#
#   ./scripts/package-app.sh              # arquitectura nativa
#   ./scripts/package-app.sh --universal  # arm64 + x86_64 unidos con lipo
set -euo pipefail

cd "$(dirname "$0")/.."

UNIVERSAL=0
[[ "${1:-}" == "--universal" ]] && UNIVERSAL=1

APP_NOMBRE="RootCause"
DIST="dist"
APP="$DIST/$APP_NOMBRE.app"
VERSION=$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)

echo "RootCause macOS Inspector v$VERSION"
echo "Construyendo $APP …"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# ── Binario ──────────────────────────────────────────────────────────────────
if [[ "$UNIVERSAL" -eq 1 ]]; then
  echo "▶ Compilando para aarch64-apple-darwin y x86_64-apple-darwin"

  # El binario universal necesita la biblioteca estándar de ambos targets, y eso
  # solo lo aporta rustup. Con un Rust instalado por Homebrew (que trae solo el
  # target del host) se degrada a la arquitectura nativa con un aviso claro, en
  # vez de fallar: la CI sí construye el universal en cada release.
  UNIVERSAL_POSIBLE=1
  if command -v rustup >/dev/null 2>&1; then
    rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null 2>&1 || true
    for objetivo in aarch64-apple-darwin x86_64-apple-darwin; do
      if ! cargo build --release --target "$objetivo"; then
        UNIVERSAL_POSIBLE=0
        break
      fi
    done
  else
    UNIVERSAL_POSIBLE=0
    echo "  ! rustup no está instalado (¿Rust vía Homebrew?)"
  fi

  if [[ "$UNIVERSAL_POSIBLE" -eq 1 ]]; then
    lipo -create -output "$APP/Contents/MacOS/rootcause" \
      target/aarch64-apple-darwin/release/rootcause \
      target/x86_64-apple-darwin/release/rootcause
  else
    echo "  ! No se pudo construir el universal; se usa la arquitectura nativa."
    echo "    Para el universal en local: instala rustup desde https://rustup.rs"
    echo "    El workflow release-macos sí publica artefactos universales."
    cargo build --release
    cp target/release/rootcause "$APP/Contents/MacOS/rootcause"
    UNIVERSAL=0
  fi
else
  echo "▶ Compilando para la arquitectura nativa"
  cargo build --release
  cp target/release/rootcause "$APP/Contents/MacOS/rootcause"
fi
chmod +x "$APP/Contents/MacOS/rootcause"

# ── Info.plist ───────────────────────────────────────────────────────────────
# La versión sale de Cargo.toml: no hay dos sitios donde mantenerla.
sed "s/__VERSION__/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"

# ── Icono ────────────────────────────────────────────────────────────────────
if command -v iconutil >/dev/null 2>&1 && command -v python3 >/dev/null 2>&1; then
  echo "▶ Generando el icono"
  ICONSET="$DIST/AppIcon.iconset"
  rm -rf "$ICONSET"
  python3 scripts/make-icon.py "$ICONSET"
  iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
  rm -rf "$ICONSET"
else
  echo "▶ Sin iconutil o python3: la app usará el icono genérico del sistema"
fi

# ── Firma ad-hoc ─────────────────────────────────────────────────────────────
# No sustituye a una firma con Developer ID, pero evita que macOS rechace el
# bundle por no tener firma alguna.
if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "$APP" 2>/dev/null \
    && echo "▶ Firma ad-hoc aplicada" \
    || echo "▶ No se pudo aplicar la firma ad-hoc (no es bloqueante)"
fi

echo
echo "✓ $APP"
du -sh "$APP" | awk '{print "  tamaño: " $1}'
[[ "$UNIVERSAL" -eq 1 ]] && lipo -info "$APP/Contents/MacOS/rootcause"
echo
echo "Probar:  open $APP"
echo "CLI:     $APP/Contents/MacOS/rootcause status"
