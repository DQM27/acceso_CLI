# JNA — usado por los bindings de uniffi para hablar con el núcleo de
# Rust (mobile/rust-core) vía FFI. JNA mapea métodos Kotlin a símbolos
# nativos por NOMBRE (Native.register) y usa reflexión sobre Structure —
# sin estas reglas, R8 renombra esos métodos al minificar y la app se cae
# al intentar hablar con el núcleo (login, Activos, todo), sin ningún
# error en tiempo de compilación que lo avise. La AAR de JNA no trae
# reglas propias (se revisó su proguard.txt: no existe), así que hacen
# falta acá.
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
-dontwarn com.sun.jna.**

# Bindings generados por uniffi (ver mobile/rust-core/uniffi-bindgen.rs y
# mobile/README.md) — mismo motivo: las interfaces
# UniffiLib/IntegrityCheckingUniffiLib se registran con Native.register,
# que necesita los nombres de método tal cual quedaron generados. Es
# código vendorizado, no de mano — no vale la pena que R8 intente
# optimizarlo, se mantiene completo.
-keep class uniffi.control_acceso_mobile.** { *; }
