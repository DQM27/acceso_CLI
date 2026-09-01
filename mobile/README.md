# App móvil — piloto

Ver `docs/plan-app-movil.md` en la raíz del repo para el plan completo y las
decisiones de diseño.

- `rust-core/` — puente `uniffi` sobre `control_acceso` (reusado sin
  modificar). No se commitea `target/` ni `bindings/` (generado).
- `app/` — proyecto Android (Kotlin + Jetpack Compose). No se commitea
  `build/`, `.gradle/`, `.kotlin/`, `local.properties` (ruta del SDK,
  depende de la máquina), ni `jniLibs/` (el `.so` compilado).

## Cómo reconstruir desde cero

Requisitos: Android Studio (para el JDK/SDK), NDK instalado vía SDK Manager,
`rustup target add aarch64-linux-android`, `cargo install cargo-ndk`.

```sh
# 1. Compilar el núcleo de Rust para Android
cd rust-core
cargo ndk -t aarch64-linux-android build --release

# 2. Generar los bindings Kotlin
cargo run --features bindgen --bin uniffi-bindgen -- generate \
  --library target/aarch64-linux-android/release/libcontrol_acceso_mobile.so \
  --language kotlin --out-dir bindings

# 3. Copiar ambos al proyecto Android
cp bindings/uniffi/control_acceso_mobile/control_acceso_mobile.kt \
   ../app/app/src/main/java/uniffi/control_acceso_mobile/
mkdir -p ../app/app/src/main/jniLibs/arm64-v8a
cp target/aarch64-linux-android/release/libcontrol_acceso_mobile.so \
   ../app/app/src/main/jniLibs/arm64-v8a/

# 4. Compilar el APK
cd ../app
echo "sdk.dir=<ruta al SDK, con / no \\>" > local.properties
./gradlew assembleDebug
```

`local.properties` usa formato Java Properties — `\` es carácter de escape,
así que la ruta del SDK debe ir con `/` (o `\\` si se insiste en backslash).
