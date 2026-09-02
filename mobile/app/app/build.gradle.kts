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
            isMinifyEnabled = false
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
}
