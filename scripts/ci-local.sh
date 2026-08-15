#!/usr/bin/env bash
# Réplica local exacta de .github/workflows/ci.yml, en el mismo orden.
# Si esto pasa, la CI pasa.
set -euo pipefail

cd "$(dirname "$0")/.."

paso() { printf '\n\033[0;34m▶ %s\033[0m\n' "$1"; }

paso "Versiones"
cargo --version
rustc --version

paso "Formato"
cargo fmt --all -- --check

paso "Análisis estático (clippy, sin tolerancia a advertencias)"
cargo clippy --all-targets --all-features -- -D warnings

paso "Tests"
cargo test --all-features

paso "Build edición CLI-only"
cargo build --release --no-default-features

paso "Build edición completa"
cargo build --release

paso "Humo — la CLI responde"
./target/release/rootcause --version
./target/release/rootcause --help > /dev/null

printf '\n\033[0;32m✓ Todo en verde. Listo para empujar.\033[0m\n'
