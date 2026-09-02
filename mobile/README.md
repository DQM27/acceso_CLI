# App móvil

Ver `docs/plan-app-movil.md` en la raíz del repo para el plan completo y las
decisiones de diseño.

- `rust-core/` — puente `uniffi` sobre `control_acceso` (reusado sin
  modificar). No se commitea `target/` ni `bindings/` (generado).
- `app/` — proyecto Android (Kotlin + Jetpack Compose). No se commitea
  `build/`, `.gradle/`, `.kotlin/`, `local.properties` (ruta del SDK,
  depende de la máquina), `jniLibs/` (el `.so` compilado), ni la keystore de
  release (`app/keystore/`, `app/keystore.properties`).
- `dist/` — APKs de distribución ya compilados (tampoco se commitea).

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

Para el emulador (x86_64) además hace falta el `.so` de ese ABI — repetir el
paso 1 con `cargo ndk -t x86_64-linux-android build --release` y copiarlo a
`jniLibs/x86_64/`. El APK de distribución para el dispositivo real (Samsung
A25 5G, arm64) sólo necesita `arm64-v8a`, pero no está de más incluir ambos
si también se va a probar en el emulador antes de repartirlo.

## Compilar un APK de distribución (release firmado)

Android no deja instalar un `release` sin firmar. La keystore vive en
`app/keystore/release.keystore.jks` — **no está en git, hay que resguardarla
aparte** (ej. gestor de contraseñas + copia de la carpeta) junto con
`app/keystore.properties` (contraseñas + alias). Sin ese archivo,
`assembleRelease` genera un APK sin firmar (inservible) — el build sigue
funcionando igual para `assembleDebug`, que no lo necesita. Si se pierde la
keystore no hay forma de firmar una actualización compatible con una versión
ya instalada: hay que resguardarla como si fuera una contraseña maestra.

```sh
cd app
./gradlew assembleRelease
```

El APK firmado queda en `app/app/build/outputs/apk/release/app-release.apk`.
`versionCode`/`versionName` (en `app/app/build.gradle.kts`) hay que subirlos
a mano en cada release para que Android reconozca que es una actualización.

## Base de datos de desarrollo (emulador/dispositivo de prueba)

`rust-core/examples/seed_dev_db.rs` crea una base SQLite con el esquema real
(las mismas migraciones que usa `AppCore::abrir`) y la llena con:
- los contratistas reales de `importar_contratistas_db_browser.sql` (raíz del repo),
- un usuario ROOT de acceso rápido: cédula `123456789`, contraseña `clave_prueba_123`
  (hash Argon2 real en `rust-core/examples/seed_usuario_root.sql`, no texto plano),
- 25 gafetes (`rust-core/examples/seed_gafetes.sql`, numerados 1-25, todos
  `DISPONIBLE`) — el SQL de contratistas es anterior al catálogo de gafetes
  y no trae ninguno.

```sh
cd rust-core
cargo run --example seed_dev_db -- seed_dev.db
```

Para subirla al almacenamiento privado de la app (el emulador/dispositivo
debe tener la app ya instalada al menos una vez):

```sh
adb push seed_dev.db /data/local/tmp/control_acceso.db
adb shell run-as com.brisas.controlacceso cp /data/local/tmp/control_acceso.db files/control_acceso.db
```

Esto es solo para desarrollo — no reemplaza el flujo real de alta de
usuarios, que sigue sin resolver (ver puntos abiertos en
`docs/plan-app-movil.md`).
