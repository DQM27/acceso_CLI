# App iOS

Base inicial para portar la app movil a iPhone usando SwiftUI + UniFFI sobre
el mismo nucleo Rust que usa Android.

Este directorio no se puede compilar completo desde Windows: Apple exige Xcode
en macOS para firmar, correr simulador iOS o instalar en un iPhone. Desde este
repo quedan listas las fuentes y scripts para que, en una Mac o CI macOS, el
flujo sea reproducible.

## Estructura

- `Sources/App/` - app SwiftUI inicial.
- `Generated/Swift/` - bindings Swift generados por UniFFI.
- `Frameworks/` - `ControlAccesoMobile.xcframework` generado desde Rust.
- `Scripts/` - pasos reproducibles para Mac.
- `project.yml` - definicion para generar el `.xcodeproj` con XcodeGen.

## Requisitos en Mac

- Xcode instalado.
- `rustup`.
- Targets Rust para iOS:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

- XcodeGen:

```sh
brew install xcodegen
```

## Generar proyecto y framework

Desde `mobile/ios`:

```sh
./Scripts/build-rust-xcframework.sh
xcodegen generate
open ControlAccesoIOS.xcodeproj
```

En Xcode falta seleccionar el `Team` de firma en el target
`ControlAccesoIOS`. Luego se puede correr en simulador, instalar por USB en un
iPhone desde la Mac, o subir por TestFlight.

## CI con GitHub Actions

El workflow `.github/workflows/ios.yml` usa un runner macOS con Xcode para:

```sh
cd mobile/ios
./Scripts/build-rust-xcframework.sh
xcodegen generate
xcodebuild -project ControlAccesoIOS.xcodeproj -scheme ControlAccesoIOS -sdk iphonesimulator CODE_SIGNING_ALLOWED=NO build
```

Ese build valida el proyecto iOS sin firma y publica como artefactos el
`.xcodeproj`, el `XCFramework` y los bindings Swift generados.

El mismo workflow tiene un modo manual para producir un `.ipa` firmado. En
GitHub Actions se ejecuta con `Run workflow` y `generar_ipa_firmado=true`.
Antes hay que agregar estos secretos del repo:

- `IOS_CERTIFICATE_P12_BASE64`
- `IOS_CERTIFICATE_PASSWORD`
- `IOS_PROVISIONING_PROFILE_BASE64`
- `APPLE_TEAM_ID`
- `APPLE_BUNDLE_ID`

Una cuenta activa de Apple Developer sirve para crear esos certificados,
perfiles y, si se decide, una API key de App Store Connect para TestFlight.
El primer `.ipa` queda como artifact de Actions; para TestFlight se agrega
despues el paso de upload a App Store Connect.

## Estado actual

La app SwiftUI arranca con una pantalla de esqueleto. El siguiente paso es
conectar `Generated/Swift` y `ControlAccesoMobile.xcframework` a pantallas
equivalentes a las de Android.
