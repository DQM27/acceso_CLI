import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
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

android {
    namespace = "com.brisas.controlacceso"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.brisas.controlacceso"
        // Dispositivo real conocido: Samsung A25 5G (arm64) — ver
        // docs/plan-app-movil.md. jniLibs trae arm64-v8a (dispositivo real)
        // y x86_64 (emulador de desarrollo).
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"
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
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            if (keystorePropertiesFile.exists()) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation(platform("androidx.compose:compose-bom:2026.01.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-core")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    // Requerido por el código Kotlin que genera uniffi para llamar al .so vía FFI.
    implementation("net.java.dev.jna:jna:5.15.0@aar")
    // ViewModel + su integración con Compose (`viewModel()`, `viewModelScope`)
    // — ver mobile/app/ARQUITECTURA.md: el estado y las llamadas a Nucleo
    // viven acá, no en el @Composable.
    // 2.9.4 es la última que compila contra compileSdk 36 — 2.10+ pide 37
    // (ver AAR metadata al subir la versión; no forma parte de este cambio
    // subir compileSdk).
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.9.4")

    // Tests unitarios de los ViewModel (JVM puro, sin emulador) — ver
    // mobile/app/src/test/.../NucleoDePrueba.kt para el porqué de cada uno.
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
