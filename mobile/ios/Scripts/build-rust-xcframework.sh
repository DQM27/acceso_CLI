#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IOS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_DIR="$(cd "$IOS_DIR/../rust-core" && pwd)"

CRATE_NAME="control_acceso_mobile"
LIB_NAME="lib${CRATE_NAME}.a"
FRAMEWORK_DIR="$IOS_DIR/Frameworks"
GENERATED_DIR="$IOS_DIR/Generated/Swift"
HEADERS_DIR="$IOS_DIR/Generated/Headers"
BUILD_DIR="$IOS_DIR/build"

mkdir -p "$FRAMEWORK_DIR" "$GENERATED_DIR" "$HEADERS_DIR" "$BUILD_DIR"

cd "$RUST_DIR"

cargo build --release

HOST_LIB=""
for candidate in \
  "target/release/lib${CRATE_NAME}.dylib" \
  "target/release/${CRATE_NAME}.dll" \
  "target/release/lib${CRATE_NAME}.so"; do
  if [ -f "$candidate" ]; then
    HOST_LIB="$candidate"
    break
  fi
done

if [ -z "$HOST_LIB" ]; then
  echo "No se encontro una libreria dinamica host para generar bindings Swift." >&2
  exit 1
fi

cargo run --features bindgen --bin uniffi-bindgen -- generate \
  --library "$HOST_LIB" \
  --language swift \
  --out-dir "$GENERATED_DIR"

find "$GENERATED_DIR" \( -name "*.h" -o -name "module.modulemap" \) -exec cp {} "$HEADERS_DIR" \;

cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim
cargo build --release --target x86_64-apple-ios

rm -rf "$FRAMEWORK_DIR/ControlAccesoMobile.xcframework"
rm -f "$BUILD_DIR/lib${CRATE_NAME}-simulator.a"

lipo -create \
  "target/aarch64-apple-ios-sim/release/$LIB_NAME" \
  "target/x86_64-apple-ios/release/$LIB_NAME" \
  -output "$BUILD_DIR/lib${CRATE_NAME}-simulator.a"

xcodebuild -create-xcframework \
  -library "target/aarch64-apple-ios/release/$LIB_NAME" -headers "$HEADERS_DIR" \
  -library "$BUILD_DIR/lib${CRATE_NAME}-simulator.a" -headers "$HEADERS_DIR" \
  -output "$FRAMEWORK_DIR/ControlAccesoMobile.xcframework"

echo "Listo: $FRAMEWORK_DIR/ControlAccesoMobile.xcframework"
