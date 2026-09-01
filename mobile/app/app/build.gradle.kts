plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.brisas.controlacceso"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.brisas.controlacceso"
        // El piloto es un solo teléfono conocido (Samsung A25 5G, arm64) —
        // ver docs/plan-app-movil.md. jniLibs sólo trae arm64-v8a a propósito.
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1-piloto"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
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
    implementation("androidx.compose.ui:ui-tooling-preview")
    // Requerido por el código Kotlin que genera uniffi para llamar al .so vía FFI.
    implementation("net.java.dev.jna:jna:5.15.0@aar")
}
