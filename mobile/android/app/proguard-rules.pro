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

# CameraX (PantallaEscanearCedula.kt) — CameraXConfig.Provider se resuelve
# por reflexión a partir de metadata del AndroidManifest de cada AAR
# (androidx.camera.camera2.Camera2Config$DefaultProvider), no por una
# referencia directa en código Kotlin. Sin esto, R8 no ve ningún uso real
# de esa clase y la elimina/renombra -- la app se cae al abrir la cámara,
# sólo en release (debug no minifica, por eso nunca se vio antes).
-keep class androidx.camera.camera2.Camera2Config { *; }
-keep class androidx.camera.core.impl.** { *; }
-keep class * implements androidx.camera.core.CameraXConfig$Provider { *; }

# ML Kit Text Recognition (mismo archivo) — TextRecognition.getClient()
# resuelve su implementación real vía Play Services (ComponentDiscovery/
# reflexión interna de com.google.android.gms), mismo riesgo que CameraX.
-keep class com.google.mlkit.** { *; }
-keep class com.google.android.gms.** { *; }
-dontwarn com.google.mlkit.**
-dontwarn com.google.android.gms.**
