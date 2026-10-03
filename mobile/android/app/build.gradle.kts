import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

// Firebase Cloud Messaging (notificaciones con la app cerrada). El
// `google-services.json` nunca va al repo (ver mobile/.gitignore): el repo es
// público y la API key que trae quedaría expuesta para siempre en el
// historial. Mismo criterio que keystore.properties: sin el archivo el build
// sigue funcionando (CI, clones nuevos), sólo que la app no recibe push --
// el plugin de Google Services falla el build si no lo encuentra, por eso se
// aplica sólo cuando existe. Se baja de la consola de Firebase (proyecto
// `lattis-f823c`) y se copia a mobile/android/app/.
if (file("google-services.json").exists()) {
    apply(plugin = "com.google.gms.google-services")
}

// Firma de release — nunca al repo (ver mobile/.gitignore). Sin
// keystore.properties el build de debug sigue funcionando igual; sólo
// assembleRelease necesita esto.
val keystorePropertiesFile = rootProject.file("keystore.properties")
val keystoreProperties = Properties().apply {
    if (keystorePropertiesFile.exists()) {
        keystorePropertiesFile.inputStream().use { load(it) }
    }
}

// Hash corto del commit para identificar el build `diagnostico`; "sinhash"
// si git no está disponible o el árbol no es un repositorio.
val hashCortoDelCommit: String = try {
    providers.exec {
        commandLine("git", "rev-parse", "--short", "HEAD")
        isIgnoreExitValue = true
    }.standardOutput.asText.get().trim().ifEmpty { "sinhash" }
} catch (e: Exception) {
    "sinhash"
}

android {
    namespace = "com.brisas.controlacceso"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.dqm27.lattis"
        // Dispositivo real conocido: Samsung A25 5G (arm64) — ver
        // docs/plan-app-movil.md. jniLibs trae sólo arm64-v8a.
        minSdk = 26
        targetSdk = 36
        versionCode = 23
        versionName = "1.4.1"

        // Referenciado desde AndroidManifest.xml (`${sentryEnvironment}`) --
        // el default acá es "development" (debug); `release {}` abajo lo
        // pisa a "production". Mismo criterio que `cfg!(debug_assertions)`
        // del lado de escritorio.
        manifestPlaceholders["sentryEnvironment"] = "development"

        // Sólo el procesador del dispositivo real (arm64-v8a, Samsung A25):
        // el APK no se usa en emuladores. Las librerías nativas de ML Kit y
        // compañía para otras ABI eran peso muerto (medido: APK de debug de
        // 94,5 MB a 64,7 MB sin x86/armeabi-v7a, y 18 MB más sin x86_64).
        ndk {
            abiFilters += listOf("arm64-v8a")
        }

        // A qué Supabase apunta el núcleo (ver AplicacionControlAcceso.kt) y
        // si se recolecta telemetría de rendimiento (ver Telemetria.kt). Los
        // dos en `false` para release: producción nunca los activa.
        buildConfigField("boolean", "AMBIENTE_STAGING", "false")
        buildConfigField("boolean", "TELEMETRIA", "false")
    }

    signingConfigs {
        if (keystorePropertiesFile.exists()) {
            create("release") {
                storeFile = rootProject.file(keystoreProperties.getProperty("storeFile"))
                storePassword = keystoreProperties.getProperty("storePassword")
                keyAlias = keystoreProperties.getProperty("keyAlias")
                keyPassword = keystoreProperties.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        debug {
            // `applicationId` propio -- sin esto, un debug build (que ya
            // apunta solo a staging, ver AplicacionControlAcceso.kt) se
            // instala como "actualización" de la app real de producción en
            // el mismo teléfono (mismo paquete, Android no distingue por
            // firma hasta ahí) y la reemplaza. Con el sufijo, Android los
            // trata como dos apps distintas -- coexisten sin pisarse.
            applicationIdSuffix = ".debug"
            buildConfigField("boolean", "AMBIENTE_STAGING", "true")
        }
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            if (keystorePropertiesFile.exists()) {
                signingConfig = signingConfigs.getByName("release")
            }
            manifestPlaceholders["sentryEnvironment"] = "production"
        }
        // Build para MEDIR rendimiento en el teléfono: compilado igual que
        // release (R8, sin `debuggable`) porque un build de debug corre sin
        // optimizaciones y daría tiempos peores que los reales. Firmado con
        // la llave de debug (no requiere la de producción), apunta al
        // sandbox (staging) y manda telemetría a su tabla
        // `telemetria_diagnostico`. Sufijo propio: convive con la app real y
        // con el build de debug sin pisarlos. `./gradlew assembleDiagnostico`.
        create("diagnostico") {
            initWith(getByName("release"))
            applicationIdSuffix = ".diag"
            // Con el hash corto del commit al final, `version_app` de cada
            // fila de telemetría dice de qué código salió ("1.3.0-diag+abc1234").
            versionNameSuffix = "-diag+$hashCortoDelCommit"
            signingConfig = signingConfigs.getByName("debug")
            isDebuggable = false
            // Permite adjuntar el profiler de Android Studio si algún día hay
            // una PC a mano, sin volver la app `debuggable`.
            isProfileable = true
            matchingFallbacks += listOf("release")
            buildConfigField("boolean", "AMBIENTE_STAGING", "true")
            buildConfigField("boolean", "TELEMETRIA", "true")
            manifestPlaceholders["sentryEnvironment"] = "diagnostico"
        }
    }

    // Los builds de prueba (debug y diagnostico) llevan el icono con la cinta
    // TEST (`src/beta/res`, generado por `scripts/generar_iconos_beta.py`):
    // junto a la app real en el mismo teléfono se distinguen a simple vista.
    sourceSets {
        getByName("debug").res.srcDir("src/beta/res")
        getByName("diagnostico").res.srcDir("src/beta/res")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation(platform("androidx.compose:compose-bom:2026.01.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-core")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.camera:camera-camera2:1.5.0")
    implementation("androidx.camera:camera-lifecycle:1.5.0")
    implementation("androidx.camera:camera-view:1.5.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    implementation("com.google.mlkit:text-recognition:16.0.1")
    // PDF417 del reverso de la cédula anterior (sólo cédula y nombre, ver
    // mobile/rust-core/src/pdf417_cedula.rs). Modelo empaquetado (~2,4 MB):
    // sin descarga en el primer uso, igual que el de texto.
    implementation("com.google.mlkit:barcode-scanning:17.3.0")
    implementation(platform("io.github.jan-tennert.supabase:bom:3.2.2"))
    implementation("io.github.jan-tennert.supabase:realtime-kt")
    implementation("io.ktor:ktor-client-okhttp:3.2.2")
    implementation("io.ktor:ktor-client-websockets:3.2.2")
    // Requerido por el código Kotlin que genera uniffi para llamar al .so vía FFI.
    implementation("net.java.dev.jna:jna:5.15.0@aar")
    // ViewModel + su integración con Compose (`viewModel()`, `viewModelScope`)
    // — ver mobile/android/ARQUITECTURA.md: el estado y las llamadas a Nucleo
    // viven acá, no en el @Composable.
    // 2.9.4 es la última que compila contra compileSdk 36 — 2.10+ pide 37
    // (ver AAR metadata al subir la versión; no forma parte de este cambio
    // subir compileSdk).
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.9.4")
    // `LocalLifecycleOwner` (PantallaPrincipal.kt): pausa la conexión
    // Realtime de la nube cuando la app pasa a segundo plano, en vez de
    // dejarla despierta gastando batería sin nadie mirando la pantalla.
    // Misma versión que lifecycle-viewmodel-compose de arriba, mismo motivo.
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.9.4")
    // Error-tracking (docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md,
    // punto 5.4 -- espejo mobile del que ya se conectó en desktop). Init
    // automático vía meta-data en AndroidManifest.xml, sin tocar código:
    // captura crashes no manejados desde el primer arranque después de
    // instalarlo.
    implementation("io.sentry:sentry-android:8.9.0")
    // Firebase Cloud Messaging: recibir avisos con la app cerrada (la
    // conexión Realtime sólo vive con la app en primer plano, ver
    // NubeRealtime.kt). El BOM fija versiones compatibles entre sí.
    implementation(platform("com.google.firebase:firebase-bom:34.19.0"))
    implementation("com.google.firebase:firebase-messaging")

    // Tests unitarios de los ViewModel (JVM puro, sin emulador) — ver
    // mobile/android/app/src/test/.../NucleoDePrueba.kt para el porqué de cada uno.
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.9.0")
    // JNA "de escritorio" (no el @aar de arriba, que es sólo para Android)
    // — necesario para que los bindings de uniffi puedan cargar el .so de
    // mobile/rust-core compilado para el host en un test JVM normal.
    testImplementation("net.java.dev.jna:jna:5.15.0")
    // Sólo para sembrar fixtures con SQL crudo antes de abrir el Nucleo
    // real del test — Nucleo no expone ningún método para insertar datos
    // sin autenticarse primero, y el primer usuario Root todavía no existe
    // en una base recién creada.
    testImplementation("org.xerial:sqlite-jdbc:3.53.4.0")
}

// mobile/rust-core compilado para el HOST (Linux, no Android) — no es el
// .so que se empaqueta en el APK (ese va en jniLibs vía cargo-ndk, ver
// mobile/README.md). Este es sólo para que los tests unitarios de acá
// puedan cargar el Nucleo real sin emulador ni dispositivo. Se reconstruye
// solo con `cargo build --release`, que es incremental — no vale la pena
// evitarlo con un `onlyIf`, el costo cuando ya está compilado es de
// milisegundos.
val compilarNucleoParaHost = tasks.register<Exec>("compilarNucleoParaHost") {
    workingDir = rootProject.file("../rust-core")
    commandLine("cargo", "build", "--release")
}

val rutaNucleoHost = rootProject.file("../rust-core/target/release").absolutePath

tasks.withType<Test>().configureEach {
    dependsOn(compilarNucleoParaHost)
    systemProperty("jna.library.path", rutaNucleoHost)
}
