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

// Compilación PUNTUAL de producción CON telemetría de rendimiento
// (`-PtelemetriaEnProduccion=true`, ver el workflow
// `build-produccion-medicion.yml`): la app real, con el mismo paquete y la
// misma firma, que opera contra la nube de producción (AMBIENTE_STAGING sigue
// en `false`) y sólo manda los números de rendimiento a la tabla
// `telemetria_diagnostico` del staging. Va en modo ligero (sin StrictMode ni
// vigilante de fugas, que fuerza recolecciones de basura). Sin la propiedad, un
// release es idéntico al de siempre: producción nunca activa la telemetría
// por su cuenta.
val telemetriaEnProduccion: Boolean =
    providers.gradleProperty("telemetriaEnProduccion").orNull == "true"

// A dónde van los errores de la app (ver `io.sentry.dsn` en
// AndroidManifest.xml). Con la variable de entorno `SENTRY_DSN_MOVIL` (el CI
// la toma del secreto `SENTRY_DSN`) apuntan al receptor propio
// (supabase/functions-staging/receptor-errores); sin ella, o vacía, siguen
// yendo al proyecto de Sentry de siempre. Es variable de entorno y no `-P`
// para que el DSN no quede en la línea de comandos ni en el registro del CI.
val dsnErrores: String =
    providers.environmentVariable("SENTRY_DSN_MOVIL").orNull?.takeIf { it.isNotBlank() }
        ?: "https://5fb608eb8db4b9a18aca8d2901a4b894@o4512103124041728.ingest.us.sentry.io/4512104610463744"

android {
    namespace = "com.brisas.controlacceso"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.dqm27.lattis"
        // Dispositivo real conocido: Samsung A25 5G (arm64) — ver
        // docs/plan-app-movil.md. jniLibs trae sólo arm64-v8a.
        minSdk = 26
        targetSdk = 36
        versionCode = 56
        versionName = "1.8.11"

        // Referenciado desde AndroidManifest.xml (`${sentryEnvironment}`) --
        // el default acá es "development" (debug); `release {}` abajo lo
        // pisa a "production". Mismo criterio que `cfg!(debug_assertions)`
        // del lado de escritorio.
        manifestPlaceholders["sentryEnvironment"] = "development"
        manifestPlaceholders["sentryDsn"] = dsnErrores

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
        // Telemetría "ligera": sin StrictMode ni vigilante de fugas, para
        // medir en producción sin cargar el teléfono (ver `telemetriaEnProduccion`).
        buildConfigField("boolean", "TELEMETRIA_LIGERA", "false")

        // De dónde lee el actualizador el manifiesto de la última versión
        // (ver Actualizador.kt y docs/actualizaciones-android.md). Vacío =
        // actualizador apagado: sólo `release` lo enciende, porque los builds
        // de prueba tienen otro paquete y no pueden instalar el APK real.
        buildConfigField("String", "URL_MANIFIESTO_ACTUALIZACION", "\"\"")

        // Nombre que se ve bajo el icono. Los builds de prueba lo pisan para
        // que junto a la app real no se confundan (ver `buildTypes`).
        manifestPlaceholders["etiquetaApp"] = "Lattis"
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
            manifestPlaceholders["etiquetaApp"] = "Lattis DEBUG"
        }
        release {
            isMinifyEnabled = true
            // Quita del APK los recursos que el código ya no referencia
            // (íconos de material-icons-extended, recursos de librerías). Corre
            // después de R8, así que sólo saca lo que de verdad nadie usa.
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            if (keystorePropertiesFile.exists()) {
                signingConfig = signingConfigs.getByName("release")
            }
            manifestPlaceholders["sentryEnvironment"] = "production"
            buildConfigField(
                "String",
                "URL_MANIFIESTO_ACTUALIZACION",
                "\"https://lt.megabrisas.com/android/latest.json\"",
            )
            if (telemetriaEnProduccion) {
                buildConfigField("boolean", "TELEMETRIA", "true")
                buildConfigField("boolean", "TELEMETRIA_LIGERA", "true")
            }
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
            // Completa (no ligera): este build es sólo para pruebas y sí puede
            // cargar la medición pesada.
            buildConfigField("boolean", "TELEMETRIA_LIGERA", "false")
            // Hereda la URL de `release` por `initWith`: se apaga, este build
            // tiene otro paquete y otra firma.
            buildConfigField("String", "URL_MANIFIESTO_ACTUALIZACION", "\"\"")
            manifestPlaceholders["sentryEnvironment"] = "diagnostico"
            manifestPlaceholders["etiquetaApp"] = "Lattis DIAG"
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

// Los datos que llegan del núcleo (los `data class` de UniFFI, con campos
// `var`) son estables para Compose: nadie los modifica después de recibirlos.
// Sin esto Compose los compara por instancia, y como cada refresco de una
// lista (sincronización, Realtime) trae objetos nuevos, redibujaba todas las
// tarjetas visibles aunque no hubiera cambiado nada.
composeCompiler {
    stabilityConfigurationFiles.add(project.layout.projectDirectory.file("estabilidad_compose.conf"))
    // `-PinformesCompose=true`: qué clases ve estables y qué funciones puede
    // saltar, en build/informes_compose (para revisar, no se publica).
    if (providers.gradleProperty("informesCompose").orNull == "true") {
        reportsDestination.set(layout.buildDirectory.dir("informes_compose"))
        metricsDestination.set(layout.buildDirectory.dir("informes_compose"))
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
    // 1.6: CameraX corre sobre CameraPipe, el mismo motor de la cámara de los
    // Pixel (2026-10-27, para bajar lo que tarda en abrir; se mide con
    // `ocr_sesion.ms_hasta_vista_previa` en el build de diagnóstico).
    implementation("androidx.camera:camera-camera2:1.6.2")
    implementation("androidx.camera:camera-lifecycle:1.6.2")
    implementation("androidx.camera:camera-view:1.6.2")
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
    // 5.16/5.17 corrigen las llamadas en teléfonos con páginas de memoria de
    // 16 KB (Android 15 en adelante); 5.19.1 repara el soporte de Android
    // viejos que rompió 5.19.0. Mismas versiones en el JNA de los tests.
    implementation("net.java.dev.jna:jna:5.19.1@aar")
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
    // Instala el Baseline Profile (`src/main/baseline-prof.txt`) en APKs
    // instalados fuera de Play Store -- el caso de esta app, que se
    // actualiza sola (ver docs/actualizaciones-android.md) -- y deja
    // consultar si ya se compiló (`ProfileVerifier`, ver Telemetria.kt).
    implementation("androidx.profileinstaller:profileinstaller:1.4.1")
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
    testImplementation("net.java.dev.jna:jna:5.19.1")
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
