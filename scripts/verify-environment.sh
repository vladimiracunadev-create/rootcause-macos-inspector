#!/usr/bin/env bash
# Comprueba que este Mac tiene todo lo necesario para compilar y ejecutar
# RootCause. No instala nada: informa y devuelve un código de salida.
set -uo pipefail

fallos=0
avisos=0

verde()  { printf '\033[0;32m[ OK ]\033[0m %s\n' "$1"; }
rojo()   { printf '\033[0;31m[FALTA]\033[0m %s\n' "$1"; fallos=$((fallos + 1)); }
ambar()  { printf '\033[0;33m[AVISO]\033[0m %s\n' "$1"; avisos=$((avisos + 1)); }

echo "RootCause macOS Inspector · verificación de entorno"
echo "=================================================="
echo

echo "── Sistema ──"
if [[ "$(uname -s)" != "Darwin" ]]; then
  rojo "Este script requiere macOS (detectado: $(uname -s))"
else
  verde "macOS $(sw_vers -productVersion) ($(sw_vers -buildVersion)) · $(uname -m)"
  mayor=$(sw_vers -productVersion | cut -d. -f1)
  if [[ "$mayor" -lt 13 ]]; then
    ambar "Se recomienda macOS 13 (Ventura) o superior"
  fi
fi
echo

echo "── Cadena de compilación ──"
if command -v cargo >/dev/null 2>&1; then
  verde "cargo $(cargo --version | awk '{print $2}')"
else
  rojo "cargo no encontrado — instala Rust desde https://rustup.rs"
fi

if command -v rustc >/dev/null 2>&1; then
  verde "rustc $(rustc --version | awk '{print $2}')"
else
  rojo "rustc no encontrado"
fi

if xcode-select -p >/dev/null 2>&1; then
  verde "herramientas de Xcode en $(xcode-select -p)"
else
  rojo "herramientas de línea de comandos de Xcode — ejecuta: xcode-select --install"
fi
echo

echo "── Utilidades del sistema que usa RootCause ──"
for herramienta in \
  /usr/sbin/lsof \
  /usr/sbin/spctl \
  /usr/bin/csrutil \
  /usr/bin/fdesetup \
  /usr/bin/codesign \
  /usr/sbin/arp \
  /sbin/route \
  /sbin/ifconfig \
  /bin/launchctl \
  /bin/ps \
  /usr/bin/log; do
  if [[ -x "$herramienta" ]]; then
    verde "$herramienta"
  else
    ambar "$herramienta no disponible — esa superficie quedará vacía"
  fi
done
echo

echo "── Empaquetado (opcional) ──"
for herramienta in iconutil hdiutil python3; do
  if command -v "$herramienta" >/dev/null 2>&1; then
    verde "$herramienta"
  else
    ambar "$herramienta no disponible — package-app.sh o package-dmg.sh fallarán"
  fi
done
echo

echo "── Permisos ──"
tcc="$HOME/Library/Application Support/com.apple.TCC/TCC.db"
if [[ -r "$tcc" ]]; then
  verde "Acceso total al disco concedido a este terminal (TCC.db legible)"
else
  ambar "Sin Acceso total al disco: la sección Privacidad no podrá leer TCC.db"
fi

if [[ "$(id -u)" -eq 0 ]]; then
  ambar "Ejecutándose como root: lsof verá los sockets de todos los usuarios"
else
  verde "Usuario normal (uid $(id -u)) — lsof solo verá tus sockets"
fi
echo

echo "=================================================="
if [[ "$fallos" -gt 0 ]]; then
  echo "Resultado: $fallos requisito(s) sin cumplir, $avisos aviso(s)."
  exit 1
fi
echo "Resultado: entorno listo. $avisos aviso(s)."
