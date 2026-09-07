# App móvil

Ver `docs/plan-persistencia-nube.md` en la raíz del repo para las decisiones
de diseño (multi-dispositivo, sincronización, roles) que aplican también al
móvil — el plan original específico de la app móvil quedó superado por ese
documento y se retiró.

- `rust-core/` — puente `uniffi` sobre `control_acceso` compartido por las
  apps moviles. No se commitea `target/` ni `bindings/` (generado).
- `android/` — proyecto Android (Kotlin + Jetpack Compose). No se commitea
  `build/`, `.gradle/`, `.kotlin/`, `local.properties` (ruta del SDK,
  depende de la máquina), `jniLibs/` (el `.so` compilado), ni la keystore de
  release (`android/keystore/`, `android/keystore.properties`).
- `ios/` — base iOS (SwiftUI + UniFFI) preparada para generar el proyecto con
  XcodeGen y compilarlo desde una Mac o CI macOS.
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
   ../android/app/src/main/java/uniffi/control_acceso_mobile/
mkdir -p ../android/app/src/main/jniLibs/arm64-v8a
cp target/aarch64-linux-android/release/libcontrol_acceso_mobile.so \
   ../android/app/src/main/jniLibs/arm64-v8a/

# 4. Compilar el APK
cd ../android
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

## Tests unitarios de los ViewModel

```sh
cd android
./gradlew test
```

Corren en el JVM del host, sin emulador ni dispositivo — el propio
`android/build.gradle.kts` compila `rust-core` para el host (no para Android,
`cargo build --release` normal, sin NDK) antes de correr los tests, así
que `./gradlew test` alcanza solo. Cada test abre un `Nucleo` real sobre
un archivo `SQLite` temporal, con la misma lógica de negocio que corre en
el teléfono — ver `android/app/src/test/.../NucleoDePrueba.kt` para cómo se
siembran los fixtures (con SQL crudo vía JDBC, porque `Nucleo` no expone
ningún método para insertar datos sin autenticarse primero).

## Compilar un APK de distribución (release firmado)

Android no deja instalar un `release` sin firmar. La keystore vive en
`android/keystore/release.keystore.jks` — **no está en git, hay que resguardarla
aparte** (ej. gestor de contraseñas + copia de la carpeta) junto con
`android/keystore.properties` (contraseñas + alias). Sin ese archivo,
`assembleRelease` genera un APK sin firmar (inservible) — el build sigue
funcionando igual para `assembleDebug`, que no lo necesita. Si se pierde la
keystore no hay forma de firmar una actualización compatible con una versión
ya instalada: hay que resguardarla como si fuera una contraseña maestra.

```sh
cd android
./gradlew assembleRelease
```

El APK firmado queda en `android/app/build/outputs/apk/release/app-release.apk`.
`versionCode`/`versionName` (en `android/app/build.gradle.kts`) hay que subirlos
a mano en cada release para que Android reconozca que es una actualización.

## Base de datos de desarrollo (emulador/dispositivo de prueba)

`rust-core/examples/seed_dev_db.rs` crea una base SQLite con el esquema real
(las mismas migraciones que usa `AppCore::abrir`) y la llena con:
- los contratistas reales de `contratistas_base_final_limpia_v15.sql` (raíz del repo),
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
usuarios, que sigue sin resolver (ver `docs/pendientes.md`).
