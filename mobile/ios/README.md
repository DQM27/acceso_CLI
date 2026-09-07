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

## Estado actual

La app SwiftUI arranca con una pantalla de esqueleto. El siguiente paso es
conectar `Generated/Swift` y `ControlAccesoMobile.xcframework` a pantallas
equivalentes a las de Android.
