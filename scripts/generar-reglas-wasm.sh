#!/usr/bin/env bash
# Genera el paquete WebAssembly de las reglas compartidas (`reglas/wasm`) y lo
# deja donde lo usan:
#   - web/src/reglas/wasm/                      panel web (Vite lo empaqueta)
#   - supabase/functions/_shared/reglas/        Edge Functions (Deno), con el
#                                               .wasm comprimido (gzip) y en
#                                               base64
#
# Hay que correrlo cada vez que cambia algo en `reglas/` y commitear lo
# generado. Si no, `web/src/reglas/reglas.test.ts` falla: compara la huella
# de las fuentes con la del paquete commiteado (ver reglas/wasm/build.rs).
#
# Requiere: rustup target add wasm32-unknown-unknown, y wasm-pack 0.13.
set -euo pipefail

RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
WEB="$RAIZ/web/src/reglas/wasm"
DENO="$RAIZ/supabase/functions/_shared/reglas"

wasm-pack build "$RAIZ/reglas/wasm" --release --target web \
  --out-dir "$WEB" --out-name reglas --no-pack
# wasm-pack deja un .gitignore con "*" en la carpeta de salida: se commitea.
rm -f "$WEB/.gitignore" "$WEB/package.json" "$WEB/README.md"

mkdir -p "$DENO"
cp "$WEB/reglas.js" "$WEB/reglas.d.ts" "$DENO/"
# gzip -n: sin nombre ni fecha dentro, así el resultado no cambia entre corridas.
{
  echo "// GENERADO por scripts/generar-reglas-wasm.sh -- no editar a mano."
  echo "// El módulo WebAssembly de las reglas, comprimido con gzip y en base64: el"
  echo "// despliegue de Edge Functions sólo admite archivos de texto, y comprimido"
  echo "// pesa un tercio (_shared/reglas.ts lo descomprime al cargar)."
  printf 'export const REGLAS_WASM_GZIP_BASE64 =\n  "%s";\n' "$(gzip -n -9 -c "$WEB/reglas_bg.wasm" | base64 -w0)"
} > "$DENO/reglas_wasm_base64.ts"

echo "Paquete de reglas generado:"
ls -la "$WEB" "$DENO"
