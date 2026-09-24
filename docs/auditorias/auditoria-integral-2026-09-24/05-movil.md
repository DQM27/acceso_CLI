# Auditoría 05 — Móvil (Android + puente UniFFI + base iOS)

Fecha: 2026-09-24 · Commit base: `372d93d` · Modalidad: análisis estático de solo lectura (sin compilar Gradle ni ejecutar cargo).
Alcance: `mobile/rust-core/` (lib.rs, Cargo.toml), `mobile/android/` completo, `mobile/ios/`, `.github/workflows/build-test-mobile.yml` (y el job `build-android` de `release.yml` / `test-android` de `ci.yml`, porque condicionan lo que llega al APK).
Contraste con auditorías previas: `docs/auditorias/auditoria-integral-android-2026-09-09.md`, `auditoria-calidad-2026-09.md`, `plan-qa-buenas-practicas-2026-09-17.md`, `checklist-qa-2026-09-17.md`.

## Resumen ejecutivo

| Severidad | Cantidad |
|---|---|
| Crítica | 0 |
| Alta | 2 |
| Media | 7 |
| Baja | 15 |
| Info | 2 |
| **Total** | **26** |

**Top 5**

1. **[MV-01] Alta — se puede saltar el cambio obligatorio de contraseña.** El login con contraseña temporal deja cacheado su hash local *antes* del cambio; si el operador pulsa "Cancelar" y vuelve a entrar con la misma temporal, entra por el camino local con `debe_cambiar_password = false`. Se puede renovar cada 24 h. Además la sesión Rust queda abierta tras cancelar.
2. **[MV-03] Alta — la base `control_acceso.db` del teléfono está en claro, y `allowBackup="false"` no impide la transferencia entre dispositivos (D2D) en Android 12+.** El cifrado ya estaba pendiente de la auditoría del 09-09. El vector D2D es **nuevo**: el Samsung A25 (Smart Switch) es justo el tipo de fabricante que la documentación señala.
3. **[MV-04] Media — Proveedores no tiene chequeo cruzado entre sitios en móvil.** `proveedorActivoEnOtroSitioConSecreto` nunca se llama y los `conflictosIngresoProveedor` de la sincronización no se muestran. Escritorio sí bloquea en ese caso.
4. **[MV-05] Media — la app se cae al elegir un contratista si falla el Keystore.** `ActivosViewModel.elegir` no captura `SecretoDispositivoStoreException`.
5. **[MV-06] Media — la salida se registra sola al escanear un gafete.** No hay confirmación y el número de gafete no tiene dígito verificador: basta que el OCR confunda un dígito dos veces para cerrar el ingreso de otra persona.

También relevante: [MV-02] (tras cambiar la contraseña, la nueva no funciona en el teléfono durante 24 h y la temporal sí), [MV-07] (el OCR hace 4 copias de imagen por frame) y [MV-21] (varios ViewModels sin pruebas, y las pruebas ejercitan el login heredado, no el que usa producción).

---

## Hallazgos

### [MV-01] Bypass del cambio obligatorio de contraseña por caché local de la contraseña temporal
- Severidad: Alta
- Categoría: Seguridad
- Ubicación: `mobile/rust-core/src/lib.rs:2745-2768` (`autenticar_supabase`), `mobile/rust-core/src/lib.rs:1324-1379` (`autenticar_con_secreto`, `debe_cambiar_password: false` fijo en 1377), `mobile/android/app/src/main/java/com/brisas/controlacceso/LoginViewModel.kt:81-84,145-148`
- Estado: Nuevo
- Descripción y evidencia: al autenticarse contra Supabase con una contraseña temporal (`debe_cambiar_password = true`), `autenticar_supabase` ya fija la sesión Rust (`*self.sesion_lock() = Some(identidad)`, l.2745) y cachea el hash de **esa contraseña temporal** como login offline válido por 24 h (`cachear_password_local(identidad.id, password)`, l.2759-2761; tope `TOPE_CACHE_LOCAL_OFFLINE` en `src/services/autenticacion_service.rs:17`). Kotlin muestra `PantallaCambioObligatorio`. Si el operador pulsa "Cancelar", solo se hace `cambioObligatorio = null` (l.145-148): **no se llama a `nucleo.cerrarSesion()`**. En el siguiente intento, `core_lock().autenticar(cedula, temporal)` valida el hash cacheado y el camino local devuelve `ResultadoLogin { debe_cambiar_password: false }` (l.1376-1379). La persona entra con la contraseña temporal sin haberla cambiado. Pasadas 24 h, el caché vence, se vuelve a Supabase (que otra vez pide el cambio), se cachea de nuevo la temporal, y el ciclo se repite indefinidamente.
  ```rust
  let resultado_cache = self.core_lock().cachear_password_local(identidad.id, password); // antes del cambio
  ...
  Ok(ResultadoLogin { sesion: identidad.into(), debe_cambiar_password: sesion_supabase.debe_cambiar_password })
  ```
- Escenario de impacto concreto: un administrador genera una contraseña temporal y la comparte por un canal inseguro (chat, papel). El operador cancela el cambio y sigue operando con la temporal todo el tiempo que quiera en ese teléfono, sin conexión incluso. Cualquiera que haya visto la temporal puede entrar como esa persona, con su rol, en ese teléfono. El control "cambio obligatorio" queda anulado. Escritorio tiene el mismo patrón (`desktop/src-tauri/src/comandos/autenticacion.rs:297-326`); conviene coordinarlo con el agente de escritorio.
- Referencia externa: OWASP MASVS-AUTH-1/-2 https://mas.owasp.org/MASVS/controls/MASVS-AUTH-2/ ; OWASP ASVS V2.1 (credenciales iniciales/temporales deben cambiarse al primer uso) https://owasp.org/www-project-application-security-verification-standard/
- Recomendación concreta: (1) en `autenticar_supabase`, no llamar a `cachear_password_local` cuando `sesion_supabase.debe_cambiar_password` sea `true`, y tampoco fijar `sesion_lock()` hasta completar el cambio (o guardar un estado "pendiente de cambio" que `actor_autenticado()` rechace). (2) En `LoginViewModel.cancelarCambioObligatorio()`, llamar a `nucleo.cerrarSesion()`. (3) Prueba en `rust-core` que verifique que, tras un login con `debe_cambiar_password`, un login local con esa misma contraseña falla.

### [MV-02] Después de cambiar la contraseña en el teléfono, la temporal sigue sirviendo 24 h y la nueva no
- Severidad: Media
- Categoría: Bug / Seguridad
- Ubicación: `mobile/rust-core/src/lib.rs:1387-1406` (`cambiar_password_supabase`); comparar con `desktop/src-tauri/src/comandos/autenticacion.rs:361-381`
- Estado: Nuevo
- Descripción y evidencia: escritorio, después de `nube::cambiar_password`, refresca el caché offline con la contraseña **nueva** (comentario explícito: "sin esto, el caché seguiría teniendo la contraseña VIEJA"). El puente móvil solo llama a `control_acceso::nube::cambiar_password(...)?; Ok(())` y no refresca nada. Como en MV-01 ya se había cacheado la temporal, al cerrar sesión y volver a entrar el flujo es este: la contraseña nueva falla contra el hash local (`CredencialesInvalidas`), se refresca el catálogo (que no toca `password_hash`, `src/nube/sincronizacion.rs:3771`), se reintenta y vuelve a fallar. El usuario ve "credenciales inválidas", mientras la temporal sigue funcionando.
- Escenario de impacto concreto: después del cambio obligatorio, el guardia cierra sesión (o la app la cierra sola a los 2 min en segundo plano, `PantallaPrincipal.kt:147-154`) y ya no puede entrar con su contraseña nueva durante un día. La contraseña temporal comprometida sigue siendo válida.
- Referencia externa: OWASP MASVS-AUTH-2 https://mas.owasp.org/MASVS/controls/MASVS-AUTH-2/
- Recomendación concreta: replicar en `Nucleo::cambiar_password_supabase` el refresco de escritorio: `self.core_lock().cachear_password_local(sesion.id, &password_nueva)`, en modo mejor esfuerzo con `log::warn!`. Idealmente, extraer una sola función en `AppCore` ("cambiar contraseña y refrescar caché") que usen las dos plataformas.

### [MV-03] Base SQLite en claro en Android y transferencia D2D no bloqueada
- Severidad: Alta
- Categoría: Seguridad
- Ubicación: `mobile/rust-core/src/lib.rs:1189-1195` (`AppCore::abrir_con_reloj`, sin clave), `mobile/rust-core/src/lib.rs:2622-2631` (`conexion_secundaria(..., None)` con el comentario "Android todavía no aplica ninguna clave"), `src/application/mod.rs:112-117`, `mobile/android/app/src/main/AndroidManifest.xml:12-22` (solo `allowBackup="false"`, sin `android:dataExtractionRules`), `.github/workflows/release.yml:162-168`
- Estado: el cifrado en reposo es un **pendiente abierto** de `auditoria-integral-android-2026-09-09.md` ("Pendiente de endurecimiento adicional"). La **transferencia D2D** y el comentario engañoso del release son **nuevos**.
- Descripción y evidencia: el APK de release compila el motor `cifrado-sqlite3mc` (release.yml:182, "para que ambas plataformas usen el mismo cifrado de verdad"), pero Android nunca aplica una clave: `abrir_con_reloj` usa `open_database(path)`, que llama a `abrir_conexion(path, None)`. La base con cédulas, nombres, historial y hashes Argon2 queda en texto plano en `filesDir`. El comentario del manifiesto supone que `allowBackup="false"` impide copiar la base a un teléfono nuevo. La documentación oficial lo contradice para `targetSdk >= 31`: *"On devices from some device manufacturers, you can't disable device-to-device migration of your app's files"*. Falta un `dataExtractionRules` con `<device-transfer><exclude .../>`. El secreto sí está protegido, porque la clave del Keystore no viaja, pero la base sí.
- Escenario de impacto concreto: al reemplazar el Samsung A25 del puesto de control, la migración con Smart Switch copia `control_acceso.db` completa (PII de todos los contratistas y el historial de accesos) al equipo nuevo, que puede ser un teléfono personal. También cualquier extracción física o forense del dispositivo lee la base sin barreras. El secreto migrado no se puede descifrar en el equipo nuevo (la clave del Keystore no viaja), así que la app queda en error hasta reconfigurarla, con la PII ya copiada.
- Referencia externa: https://developer.android.com/guide/topics/manifest/application-element (atributo `allowBackup`, nota Android 12) ; https://developer.android.com/about/versions/12/behavior-changes-12 ("doesn't disable D2D transfers") ; OWASP MASVS-STORAGE-1/-2 https://mas.owasp.org/MASVS/controls/MASVS-STORAGE-2/ ; MASTG-TEST-0262 (backup) https://mas.owasp.org/MASTG/
- Recomendación concreta: (1) Ya mismo, agregar `android:dataExtractionRules="@xml/reglas_extraccion"` que excluya `database`, `file` y `sharedpref` tanto en `<cloud-backup>` como en `<device-transfer>`. Mantener `allowBackup="false"` y `fullBackupContent` para API < 31. (2) Terminar el pendiente de cifrado: generar una clave AES de 32 bytes, envolverla con Android Keystore (mismo esquema que `SecretoDispositivoStore`), exponer `Nucleo::abrir_cifrado(ruta, clave: Vec<u8>)` que use `abrir_con_reloj_cifrado`, y pasar esa clave a `conexion_secundaria`. Incluir una migración probada de base plana a cifrada (`sqlcipher_export`/`VACUUM INTO`). (3) Mientras no esté hecho, corregir el comentario de release.yml:162-168, que da a entender que el APK ya cifra.

### [MV-04] Proveedores sin chequeo cruzado entre sitios y conflictos de proveedor no mostrados
- Severidad: Media
- Categoría: Bug (paridad funcional)
- Ubicación: `mobile/android/app/src/main/java/com/brisas/controlacceso/ProveedoresViewModel.kt:210-256`; `mobile/rust-core/src/lib.rs:2313-2332` (export sin uso); `PantallaPrincipal.kt:96,125,183` (solo `conflictosIngreso`)
- Estado: Nuevo
- Descripción y evidencia: el doc-comment de Rust dice "llamar justo antes de `registrar_ingreso_proveedor`", y escritorio bloquea (`desktop/src-tauri/src/comandos/proveedores.rs:164`). En Kotlin, `grep proveedorActivoEnOtroSitioConSecreto` devuelve 0 usos en `src/main`. `registrarIngreso` solo verifica el gafete. Además `ResumenSincronizacion.conflictosIngresoProveedor` se calcula en Rust (lib.rs:2897-2902, 3007-3012) pero ninguna pantalla lo lee.
- Escenario de impacto concreto: un proveedor con ingreso abierto en la portería de otro sitio puede entrar en este sin ningún aviso, y el conflicto posterior tampoco se muestra. El registro de presencia queda inconsistente, y eso importa en una evacuación.
- Referencia externa: OWASP MASVS-CODE-4 (validación consistente) https://mas.owasp.org/MASVS/controls/MASVS-CODE-4/
- Recomendación concreta: en `ProveedoresViewModel.registrarIngreso`, antes de registrar, llamar a `nucleo.proveedorActivoEnOtroSitioConSecreto(secreto, cedula)` y bloquear con el mismo mensaje que escritorio. Mostrar `resumen.conflictosIngresoProveedor` en `PantallaPrincipal` igual que `conflictosIngreso`. Agregar la prueba correspondiente.

### [MV-05] Cierre de la app al elegir un contratista cuando falla el Keystore
- Severidad: Media
- Categoría: Bug
- Ubicación: `mobile/android/app/src/main/java/com/brisas/controlacceso/ActivosViewModel.kt:292-321`
- Estado: Nuevo
- Descripción y evidencia: `elegir()` llama a `secretoStore.cargar()`, que puede lanzar `SecretoDispositivoStoreException` (archivo corrupto, `AEADBadTagException` tras una restauración o migración, fallo del Keystore; ver `SecretoDispositivoStore.kt:83-87`). Solo se captura `NucleoException`, así que la excepción escapa de `viewModelScope` sin manejador y **la app se cae**. `seleccionIngreso` además queda en `Cargando`. El resto de los ViewModels sí capturan esa excepción (`NubeViewModel.kt:74`, `ProveedoresViewModel.kt:248`).
  ```kotlin
  val secreto = withContext(dispatcherIO) { secretoStore.cargar() }   // puede lanzar
  ...
  } catch (excepcion: NucleoException) { ... }                        // única captura
  ```
- Escenario de impacto concreto: en el escenario D2D de MV-03, o con el archivo del secreto dañado, cada vez que el guardia toca un contratista para registrar su ingreso la app se cierra. La operación principal queda bloqueada.
- Referencia externa: https://developer.android.com/kotlin/coroutines/coroutines-best-practices (manejo de excepciones en `viewModelScope`)
- Recomendación concreta: envolver solo el chequeo cruzado en `try/catch (SecretoDispositivoStoreException)` y tratarlo como "sin chequeo cruzado", en modo mejor esfuerzo como el resto. Agregar un `finally` que saque `seleccionIngreso` de `Cargando`. Prueba en `ActivosViewModelTest` con un `SecretoDispositivoStore` que lance la excepción.

### [MV-06] Salida registrada automáticamente por OCR de gafete, sin confirmación ni dígito verificador
- Severidad: Media
- Categoría: Bug (integridad de datos) / Mala práctica
- Ubicación: `PantallaActivos.kt:117-130` (`continuo = true`), `PantallaEscanearCedula.kt:200-268`, `ActivosViewModel.kt:408-451`, `EscaneoCompartido.kt`/`EstabilizadorLectura.kt:135-152` (2 coincidencias en una ventana de 4 frames), `LectorDocumentosIdentidad.kt:195,412-419`
- Estado: Nuevo (el comportamiento sin confirmación fue un pedido del usuario el 2026-09-20; aquí se evalúa su riesgo)
- Descripción y evidencia: en modo continuo, cada gafete confirmado dispara `registrarSalida` sin intervención humana. El número sale de la expresión `\bCRC\s*[-:]?\s*(\d{1,4})\b` y alcanza con que se repita en 2 de los últimos 4 frames. El gafete no tiene checksum. Las confusiones de OCR entre 3/8, 1/7 o 0/6 suelen repetirse en frames consecutivos del mismo reflejo.
- Escenario de impacto concreto: con el gafete 38 frente a la cámara, ML Kit lee "33" en dos frames seguidos y se cierra el ingreso del portador del gafete 33, que sigue dentro del sitio. El registro de presencia (evacuaciones, auditoría) queda falso, y el error solo se nota por un mensaje de 1,6 s.
- Referencia externa: guía de ML Kit Text Recognition (la precisión depende de las condiciones; se recomienda validar) https://developers.google.com/ml-kit/vision/text-recognition/v2/android ; OWASP MASVS-CODE-4 https://mas.owasp.org/MASVS/controls/MASVS-CODE-4/
- Recomendación concreta: exigir más frames en modo gafete (por ejemplo 3 de 4, y en frames no consecutivos), mostrar el nombre del activo encontrado y registrar la salida con un toque de confirmación, o permitir deshacer durante unos segundos. A mediano plazo, agregar dígito verificador o QR al gafete.

### [MV-07] Recorte OCR costoso por frame (YUV, NV21, JPEG y 3 Bitmaps)
- Severidad: Media
- Categoría: Rendimiento
- Ubicación: `mobile/android/app/src/main/java/com/brisas/controlacceso/PantallaEscanearCedula.kt:577-627`, `RecorteImagenOcr.kt:79-106`
- Estado: Nuevo
- Descripción y evidencia: en cada frame analizado (1280x720) se copian los planos Y/U/V a `ByteArray`, se arma el NV21, se comprime a JPEG q90, se decodifica a `Bitmap` ARGB (unos 3,7 MB), se crea un Bitmap rotado (otros 3,7 MB) y después el recortado. No se reutiliza nada ni se llama a `recycle()`. Lo usan las 4 pantallas de escaneo. Además, el JPEG con pérdida degrada el texto que recibe ML Kit.
- Escenario de impacto concreto: en el A25, durante el escaneo continuo de gafetes a la salida de un turno, el recolector de basura corre sin parar, se pierden frames, aumenta la latencia de lectura y el consumo de batería, y el teléfono se calienta. Es justo lo que la auditoría del 09-09 quería reducir.
- Referencia externa: https://developer.android.com/media/camera/camerax/analyze (`OUTPUT_IMAGE_FORMAT_RGBA_8888`, `ImageProxy.toBitmap()`) ; https://developers.google.com/ml-kit/vision/text-recognition/v2/android#input-image-guidelines
- Recomendación concreta: pedir `setOutputImageFormat(OUTPUT_IMAGE_FORMAT_RGBA_8888)` y usar `imageProxy.toBitmap()` con un solo `Bitmap.createBitmap(src, rect)` sobre el rect calculado en coordenadas del sensor (invirtiendo la rotación, una sola vez). Alternativa: calcular el recorte directo sobre el NV21 y pasarlo con `InputImage.fromByteArray`. Así se elimina el JPEG intermedio. Reutilizar buffers entre frames.

### [MV-08] La región que se analiza no coincide con el marco en pantalla; el ViewPort no tiene efecto
- Severidad: Baja
- Categoría: Bug
- Ubicación: `PantallaEscanearCedula.kt:411-423,544,577-623`
- Estado: Nuevo
- Descripción y evidencia: el comentario afirma que `setViewPort(previewView.viewPort)` "ata el recorte de `analisis`" a lo visible. Para `ImageAnalysis`, CameraX solo informa `ImageProxy.getCropRect()`; el búfer no se recorta. El código nunca lee `cropRect` (`grep cropRect` devuelve 0) y recorta sobre `imagen.width/height` del frame completo. Además, `previewView.viewPort` es `null` mientras la vista no tiene tamaño, algo probable dentro de `factory`. Como la vista previa usa `FILL_CENTER`, el rectángulo analizado es proporcionalmente más ancho que el marco que ve el guardia.
- Escenario de impacto concreto: se reconoce texto que está fuera del recuadro (otro documento o gafete sobre la mesa). La promesa "lo que ves es lo que se lee" no se cumple, y a veces se confirma un documento distinto del que el operador está encuadrando.
- Referencia externa: https://developer.android.com/media/camera/camerax/transform-output ("for ImageAnalysis, the output crop rect is getCropRect")
- Recomendación concreta: calcular la región sobre `imagen.cropRect` (con el ViewPort ya aplicado una vez que la vista tiene tamaño) o usar `CoordinateTransform`/`ImageProxyTransformFactory` de `camera-view`. Corregir el comentario.

### [MV-09] Mutaciones y E/S del núcleo dentro de Composables, en `Dispatchers.Default`
- Severidad: Media
- Categoría: Mala práctica / Acoplamiento
- Ubicación: `PantallaConfirmarIngreso.kt:177-211` (chequeo de gafete y `registrarIngreso`), `PantallaNuevoContratista.kt:157-163,458-491` (`listarEmpresas`, `crearContratista`)
- Estado: Nuevo (la auditoría de calidad dio por bueno "sin lógica en Composables"; estas pantallas la contradicen)
- Descripción y evidencia: estas dos pantallas llaman directamente a `nucleo.*` con `rememberCoroutineScope()`. `PantallaNuevoContratista` hace SQLite y FFI bloqueantes en `Dispatchers.Default` (pool de CPU del tamaño de los núcleos), no en `IO`. Si la composición se desmonta (por el Atrás o por el cierre automático de sesión) durante `registrarIngreso`, la corrutina se cancela: la escritura en Rust termina igual, pero no se ejecutan `onRegistrado()` ni `CambiosNube.solicitar()`. Son pantallas de 440 y 505 líneas con reglas de negocio (`requierePraind`, `praindVencido`, emparejamiento de empresas) que no tienen pruebas.
- Escenario de impacto concreto: un ingreso queda grabado, pero la UI no se refresca y la sincronización no se dispara hasta el siguiente pulso de 2 min, así que otro dispositivo podría reasignar ese gafete mientras tanto. Además, bloquear `Default` retrasa el OCR y otras tareas de CPU.
- Referencia externa: https://developer.android.com/topic/architecture/ui-layer/stateholders ; https://developer.android.com/kotlin/coroutines/coroutines-best-practices#inject-dispatchers
- Recomendación concreta: pasar ambos flujos a ViewModels con clave por intento (por ejemplo `viewModel(key = "ingreso-${preparacion.contratistaId}")` dentro del `ViewModelStore` de sesión), usar `dispatcherIO` inyectado y cubrirlos con pruebas como `ActivosViewModelTest`.

### [MV-10] Reglas y utilidades duplicadas (Kotlin frente al núcleo y entre pantallas)
- Severidad: Baja
- Categoría: Acoplamiento
- Ubicación: `PantallaNuevoContratista.kt:63-64` (`requierePraind`) frente a `src/domain/contratista.rs:8-14`; `PantallaNuevoContratista.kt:78-88` frente a `PantallaRutas.kt:331-342` (`aTextoDDMMYYYY*`/`textoDDMMYYYYaIso*` copiados); lógica de búsqueda con debounce repetida 7 veces (`RutasViewModel` x3, `ProveedoresViewModel`, `GafetesProvisionalesViewModel`, `ActivosViewModel`); boilerplate de cámara repetido en 4 pantallas (`Executors`, `TextRecognition`, `DisposableEffect`, `ImageAnalysis.Builder`: `PantallaEscanear*.kt`)
- Estado: Nuevo
- Descripción y evidencia: `requierePraind` reimplementa en Kotlin la regla de dominio `personal_ruta || Praind || InHouse`. Si la regla cambia en Rust, el formulario va a pedir o bloquear mal. Las fechas se formatean con `"%02d".format` sin `Locale`, así que en un dispositivo con dígitos no latinos se generaría una fecha ISO que Rust no puede interpretar.
- Escenario de impacto concreto: se agrega un tipo de ingreso nuevo que requiere PRAIND. Escritorio y el núcleo lo exigen, pero el formulario móvil no marca la fecha vencida, y el error solo aparece al guardar.
- Referencia externa: https://developer.android.com/topic/architecture/domain-layer
- Recomendación concreta: exportar `requiere_praind` y `requiere_gafete` por UniFFI (o incluirlas en un record de "reglas del formulario") y eliminar la copia en Kotlin. Extraer `FechaFormulario.kt` (con `Locale.ROOT`), un `BuscadorConDebounce<T>` y un `@Composable CamaraOcr(...)` compartidos.

### [MV-11] Superficie UniFFI heredada sin uso en producción (incluye lectura de un secreto en disco derivado de ANDROID_ID)
- Severidad: Baja
- Categoría: Código muerto / Seguridad
- Ubicación: `mobile/rust-core/src/lib.rs` — `autenticar` (1245), `configurar_dispositivo_inicial` (1841), `guardar_secreto_dispositivo` (1949), `secreto_dispositivo_guardado` (1966), `sincronizar_con_nube` (2014), `sesion_realtime_nube` (2046), `gafete_ocupado_en_sitio` (2135), `cerrar_ingreso_remoto` (2336), `cerrar_ingreso_proveedor_remoto` (2380), `cerrar_prestamo_gafete_provisional_remoto` (2429), `listar_usuarios` (1792), `crear_usuario` (1812), junto con `refrescar_catalogo_sin_sesion` (2556) e `intentar_sincronizar_con_nube` (2796)
- Estado: Nuevo
- Descripción y evidencia: la comparación de los 55 métodos exportados con los usos en `src/main` de Kotlin da 0 usos para los 12 métodos listados. `autenticar` solo se usa en pruebas (`NubeViewModelTest.kt:50`, `RutasViewModelTest.kt:128`, `ActivosViewModelTest.kt:151`). Varios siguen leyendo el secreto del archivo legado cifrado con una clave derivada de `ANDROID_ID`, el esquema que la auditoría de calidad reemplazó. `guardar_secreto_dispositivo` todavía **escribe** en ese formato débil.
- Escenario de impacto concreto: una regresión o un llamador nuevo que use `guardarSecretoDispositivo` vuelve a persistir el secreto fuera del Keystore sin que nadie lo note. Son unas 400 líneas de Rust mantenidas y probadas por nada.
- Referencia externa: OWASP MASVS-CODE-3 (minimizar superficie) https://mas.owasp.org/MASVS/controls/MASVS-CODE-3/
- Recomendación concreta: borrar esos exports, conservando solo `cargar_secreto_dispositivo_legado` y `borrar_secreto_dispositivo_legado` para la migración (con fecha de retiro). Pasar las pruebas Kotlin a `autenticarConSecreto`. Regenerar los bindings.

### [MV-12] Sincronización duplicada casi línea a línea en lib.rs
- Severidad: Baja
- Categoría: Mala práctica / Acoplamiento
- Ubicación: `mobile/rust-core/src/lib.rs:2796-2931` frente a `2933-3037`; 11 construcciones idénticas de `ContextoSincronizacion` (1290, 1357, 1901, 2161, 2196, 2229, 2264, 2298, 2322, 2363, 2407, 2456, 2572, 2593, 2836, 2960)
- Estado: Nuevo
- Descripción y evidencia: `intentar_sincronizar_con_nube` e `intentar_sincronizar_con_secreto` difieren solo en de dónde sale el secreto, y repiten la misma cadena de 11 pasos `recibir_*`. El mismo orden se repite en escritorio (`GuiState`) y en `src/application/nube.rs`, así que la secuencia de sincronización vive en 3 o 4 lugares. Ya causó un bug: "Faltaba -- `encargados_ruta`/`vehiculos_ruta` nunca se traían en mobile" (l.2867-2872).
- Escenario de impacto concreto: al agregar una tabla nueva a la sincronización se actualiza una sola copia y el móvil deja de recibir ese catálogo. Ese mismo bug ya ocurrió y está documentado en el código.
- Referencia externa: https://mozilla.github.io/uniffi-rs/latest/ (el puente debe ser delgado)
- Recomendación concreta: mover la secuencia a una sola función en `control_acceso::nube` (por ejemplo `sincronizar_sitio(&conexion, &contexto) -> ResumenSincronizacion`) y usarla desde móvil y escritorio. Agregar un helper `fn contexto<'a>(token: &'a TokenDispositivo) -> ContextoSincronizacion<'a>`.

### [MV-13] Panics de Rust y errores no-`NucleoException` tiran la app; errores de apertura sin registro
- Severidad: Baja
- Categoría: Bug / Robustez FFI
- Ubicación: generado `uniffi/.../control_acceso_mobile.kt:254` (`class InternalException`); ViewModels que capturan solo `NucleoException` (por ejemplo `RutasViewModel`, `ActivosViewModel.kt:103-108` lo declara a propósito); `AplicacionViewModel.kt:208-214`
- Estado: Nuevo (la auditoría de calidad solo valoró el `Mutex` envenenado)
- Descripción y evidencia: UniFFI atrapa los panics en la frontera y los convierte en `InternalException`, así que no hay UB. Pero ningún `catch` de Kotlin la maneja, y `viewModelScope` la propaga hasta cerrar la app. Tampoco se maneja `IllegalStateException` ("already destroyed") si se usa `Nucleo` después de `close()` en `AplicacionViewModel.onCleared`. En sentido contrario, `abrirEntorno` hace `catch (_: Exception)` sin registrar nada ni reportar a Sentry: si la base no abre, solo aparece "No fue posible abrir los datos" y no queda rastro para diagnóstico.
- Escenario de impacto concreto: un `unwrap` en una ruta poco común del núcleo (por ejemplo, un dato remoto inesperado durante la sincronización) cierra la app en el puesto de control en vez de mostrar un error. Una base corrupta no deja evidencia.
- Referencia externa: https://mozilla.github.io/uniffi-rs/latest/udl/errors.html (errores inesperados y panics como `InternalException`)
- Recomendación concreta: agregar un `CoroutineExceptionHandler` común (o capturar `InternalException` en un helper `llamarNucleo {}`) que muestre un error genérico y reporte con `Sentry.captureException`. En `abrirEntorno`, llamar a `Log.e` y `Sentry.captureException(e)` antes de devolver `Fallo`.

### [MV-14] Mensajes técnicos crudos (cuerpo HTTP y URL) en la UI y en Logcat de release
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `mobile/rust-core/src/lib.rs:934-938` (`interno()` hace `log::error!` y devuelve el texto a Kotlin), `lib.rs:1173-1182` (nivel `Warn` en release, así que `error!` sale); `src/nube/sincronizacion.rs:52` (`"El receptor respondió {status}: {cuerpo}"`), `src/nube/cliente.rs:10` (`reqwest::Error` incluye la URL); Kotlin: `error = excepcion.message` en todos los ViewModels
- Estado: Nuevo (la auditoría de calidad revisó solo `Log.*` de Kotlin)
- Descripción y evidencia: el cuerpo completo de una respuesta inesperada de PostgREST (que puede traer valores de filas, como `Key (cedula)=(…)`) y las URLs con filtros se escriben en Logcat en release y se muestran tal cual al operador ("error interno: El receptor respondió 409: {…}").
- Escenario de impacto concreto: un informe de errores (`bugreport`) o `adb logcat` durante soporte expone datos. El guardia ve JSON técnico en pantalla.
- Referencia externa: OWASP MASVS-STORAGE-2 / MASTG-TEST-0203 (logs) https://mas.owasp.org/MASTG/tests/android/MASVS-STORAGE/MASTG-TEST-0203/
- Recomendación concreta: en `interno()`, registrar solo la variante y el status (sin `cuerpo`) en release, y devolver a Kotlin un mensaje genérico usando `control_acceso::mensajes`, como hace escritorio con `mensaje_generico`.

### [MV-15] Dependencias Android desactualizadas con avisos conocidos
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `mobile/android/app/build.gradle.kts:95-98,118`
- Estado: Nuevo
- Descripción y evidencia: `io.ktor:ktor-client-okhttp/websockets:3.2.2` es anterior a 3.4.1, afectada por CVE-2026-68762 (DoS por descompresión de WebSocket, CVSS 5.9); la app usa WebSocket para Realtime. `io.sentry:sentry-android:8.9.0` es anterior a 8.14.0 (GHSA-7cjh-xx4r-qh3f, datos sin enmascarar en Session Replay con Compose 1.8 o superior). Hoy no aplica porque Session Replay no está configurado, pero sí se aplicaría si alguien lo activa. `core-ktx 1.15.0` y `activity-compose 1.9.3` están atrasadas. En CI, `cargo install cargo-ndk` va sin versión ni `--locked` (build-test-mobile.yml y release.yml), lo que rompe la reproducibilidad y expone a la cadena de suministro.
- Escenario de impacto concreto: un servidor Realtime comprometido o un intermediario con un certificado válido podría provocar un DoS del cliente. Si se activa Replay para diagnosticar, se filtran capturas con cédulas.
- Referencia externa: https://stack.watch/product/jetbrains/ktor/ (CVE-2026-68762) ; https://github.com/advisories/GHSA-7cjh-xx4r-qh3f
- Recomendación concreta: subir el BOM de supabase-kt y Ktor a una versión 3.4.1 o posterior, junto con `sentry-android` 8.14 o posterior (y declarar `io.sentry.session-replay.*-sample-rate=0` explícito). Usar `cargo install cargo-ndk --locked --version X.Y.Z`. Agregar Dependabot o OWASP dependency-check para Gradle.

### [MV-16] Clave del Keystore sin `setUnlockedDeviceRequired`/StrongBox y sin sincronización al crearla
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `mobile/android/app/src/main/java/com/brisas/controlacceso/SecretoDispositivoStore.kt:108-122`
- Estado: Nuevo (la auditoría de calidad y el plan QA dieron el Keystore por resuelto; esto es endurecimiento adicional)
- Descripción y evidencia: la clave AES-GCM está bien, pero se puede usar con el dispositivo bloqueado, no pide StrongBox (con respaldo si no está disponible) y `obtenerClave()`/`guardar()` no están sincronizados. Si dos hilos migran a la vez, cada uno genera una clave con el mismo alias y el archivo puede quedar cifrado con la clave que se sobrescribió. Es poco probable porque la migración ocurre en el login.
- Escenario de impacto concreto: con el teléfono bloqueado y robado, un atacante con ejecución de código en el contexto de la app (root) puede descifrar el secreto sin conocer el PIN. Con la carrera, el secreto queda inutilizable y hay que reconfigurar el dispositivo.
- Referencia externa: https://developer.android.com/privacy-and-security/keystore ; MASVS-STORAGE-1 / MASTG-TEST-0209 https://mas.owasp.org/MASTG/
- Recomendación concreta: `.setUnlockedDeviceRequired(true)` (API 28+), intentar `.setIsStrongBoxBacked(true)` con respaldo ante `StrongBoxUnavailableException`, y marcar `guardar`/`cargar` como `@Synchronized`. La prueba con Robolectric sigue pendiente (plan QA punto 10).

### [MV-17] El motor SQLite de las pruebas y del build de prueba no es el de release
- Severidad: Baja
- Categoría: Pruebas
- Ubicación: `mobile/rust-core/Cargo.toml:33` (predeterminado `cifrado-sqlcipher`), `mobile/android/app/build.gradle.kts:142-145` (`cargo build --release` con el predeterminado), `.github/workflows/build-test-mobile.yml:51-53` (predeterminado), `.github/workflows/release.yml:180-193` (`--no-default-features --features cifrado-sqlite3mc`)
- Estado: Nuevo
- Descripción y evidencia: las pruebas Kotlin (`ci.yml:56-58`, `release.yml` paso "Tests") y el APK de debug manual usan SQLCipher, mientras el APK distribuido usa SQLite3MC. Nada prueba de punta a punta el binario que llega a producción. Además, `build-test-mobile.yml` no corre pruebas y genera un APK `debuggable` apuntando a staging, pensado para instalarse en el dispositivo real (`run-as` permite leer la base en claro de MV-03).
- Escenario de impacto concreto: una diferencia entre motores (pragmas, `PLEGAR`, migraciones) rompe el APK real sin que falle ninguna prueba.
- Referencia externa: OWASP MASVS-CODE-1 https://mas.owasp.org/MASVS/controls/MASVS-CODE-1/
- Recomendación concreta: cambiar el predeterminado de `mobile/rust-core` a `cifrado-sqlite3mc`, o pasar `--no-default-features --features cifrado-sqlite3mc` en `compilarNucleoParaHost` y en el workflow de prueba. Agregar `./gradlew testDebugUnitTest` a `build-test-mobile.yml`.

### [MV-18] La versión real de la app no llega al núcleo (chequeo de versión mínima inactivo en móvil)
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `mobile/rust-core/src/lib.rs` (no exporta `establecer_version_app`; `src/application/mod.rs:103-106` existe), `AplicacionViewModel.kt:196`
- Estado: Pendiente de auditoría previa (`plan-qa-buenas-practicas-2026-09-17.md` punto 9 y `checklist-qa-2026-09-17.md`, "Chequeo de versión mínima en mobile")
- Descripción y evidencia: la versión solo viaja en la activación inicial. En cada renovación de token no se envía, así que `device-auth` no puede rechazar un APK viejo.
- Escenario de impacto concreto: un APK con una vulnerabilidad corregida (MV-01, por ejemplo) sigue operando indefinidamente en un puesto de control que nunca actualizó.
- Referencia externa: MASVS-CODE-2 (forzar actualizaciones) https://mas.owasp.org/MASVS/controls/MASVS-CODE-2/
- Recomendación concreta: exportar `Nucleo::establecer_version_app(version: String)` y llamarlo con `BuildConfig.VERSION_NAME` justo después de `Nucleo.abrir`. Hacer que `autenticar_y_cachear` del puente la envíe (hoy usa su propio caché y no pasa por `AppCore`).

### [MV-19] Carácter NUL literal en el código fuente: git trata el archivo como binario
- Severidad: Baja
- Categoría: Mala práctica
- Ubicación: `mobile/android/app/src/main/java/com/brisas/controlacceso/EscaneoCompartido.kt:94`
- Estado: Nuevo
- Descripción y evidencia: `private const val CLAVE_SIN_CANDIDATO = "<NUL>"` contiene un byte 0x00 literal. `file` lo reporta como `data`, `grep` como "binary file matches" y `git diff` lo muestra como binario. `EstabilizadorLectura.kt:214` usa correctamente `"\u0000"`.
- Escenario de impacto concreto: los cambios de revisión en este archivo, que comparten las 4 pantallas de escaneo, no se ven en los diffs ni en la revisión de PR.
- Referencia externa: https://git-scm.com/docs/gitattributes#_marking_files_as_binary
- Recomendación concreta: reemplazarlo por `"\u0000"`.

### [MV-20] Listas de Proveedores y KOF: un fallo remoto vacía también los activos locales
- Severidad: Baja
- Categoría: Bug
- Ubicación: `ProveedoresViewModel.kt:103-119`, `GafetesProvisionalesViewModel.kt:75-91` (compárese con `ActivosViewModel.kt:221-226`, `remotosSeguro`)
- Estado: Nuevo
- Descripción y evidencia: `listarProveedoresActivos() to listarIngresosProveedorRemotos()` va en un solo bloque. Si la parte remota lanza una excepción (sesión expulsada, `autorizar_uso_nube`), no se asigna `activos` y se muestra un error, aunque la parte local sí funcionó. En Activos esto se resolvió con `remotosSeguro`.
- Escenario de impacto concreto: el guardia no ve los proveedores y préstamos locales que tiene que cerrar.
- Referencia externa: —
- Recomendación concreta: reutilizar el patrón `remotosSeguro` (local obligatorio, remoto en modo mejor esfuerzo).

### [MV-21] Pruebas: ViewModels y flujos críticos sin cobertura; las pruebas usan el login heredado
- Severidad: Media
- Categoría: Pruebas
- Ubicación: `mobile/android/app/src/test/…` (16 archivos); no existe `src/androidTest`
- Estado: Parcialmente pendiente de auditoría previa (09-09: "Pruebas instrumentadas con Activity/CameraX"; plan QA punto 10: Keystore sin prueba). El resto es nuevo.
- Descripción y evidencia: sin pruebas quedan `ProveedoresViewModel`, `GafetesProvisionalesViewModel`, `PrimerArranqueViewModel`, `AplicacionViewModel`, `SincronizacionPeriodica`, `NubeRealtime`, `AndroidKeystoreSecretoDispositivoStore`, el flujo de `PantallaConfirmarIngreso`/`PantallaNuevoContratista` y el cambio obligatorio de contraseña (MV-01 y MV-02 habrían aparecido con una prueba). Las pruebas de ViewModel autentican con `nucleo.autenticar(...)` (heredado), no con `autenticarConSecreto`. Los parsers (`MrzParser`, `LectorDocumentosIdentidad`, `Estabilizador*`, lectores de ruta y KOF) sí tienen buena cobertura.
- Escenario de impacto concreto: las regresiones de MV-04, MV-05 y MV-20 llegaron a `main` sin que fallara ninguna prueba.
- Referencia externa: https://developer.android.com/training/testing/fundamentals ; MASVS-CODE
- Recomendación concreta: agregar pruebas JVM para los 3 ViewModels (el patrón `NucleoDePrueba` ya existe) y una prueba Rust de `autenticar_supabase` con `debe_cambiar_password`. Sumar Robolectric para el Keystore y un androidTest mínimo de ciclo de vida de CameraX.

### [MV-22] MRZ: la línea de nombres, sin checksum, se acepta en un solo frame; número TD1 sin recortar
- Severidad: Baja
- Categoría: Bug
- Ubicación: `EstabilizadorLectura.kt:91-113`, `MrzParser.kt:109,193,199`
- Estado: Nuevo
- Descripción y evidencia: con los checksums válidos se confirma en el mismo frame. Pero en TD1 los checksums solo cubren número y fechas; la línea 3 (nombres) no tiene verificador y el OCR suele confundir `<` con `K`/`C`. En TD1, `numeroDocumento` no hace `trimEnd('<')` (TD3 sí), así que un documento extranjero con número corto trae `<`.
- Escenario de impacto concreto: se da de alta un proveedor con el nombre "MARIAKKJOSE" leído en un solo frame.
- Referencia externa: ICAO Doc 9303 Parte 5 (TD1) https://www.icao.int/publications/Documents/9303_p5_cons_en.pdf
- Recomendación concreta: aceptar el número en un frame, pero exigir 2 frames coincidentes para el campo de nombres (o normalizar `K<`). Hacer `trimEnd('<')` también en TD1.

### [MV-23] Autoselección del único resultado de búsqueda tras escanear
- Severidad: Baja
- Categoría: Bug
- Ubicación: `ActivosViewModel.kt:469-474`
- Estado: Nuevo
- Descripción y evidencia: `if (resultados.size == 1) return resultados.single()` elige al único resultado de la búsqueda `LIKE` aunque su cédula no coincida con la escaneada (por ejemplo, una cédula de 9 dígitos contenida en un DIMEX de 12, o un nombre parcial de un carnet in-house).
- Escenario de impacto concreto: se abre el formulario de ingreso de otra persona. Hay confirmación posterior, pero se induce un error.
- Referencia externa: —
- Recomendación concreta: autoseleccionar solo si los dígitos coinciden exactamente (o el nombre coincide exactamente, en el caso del carnet in-house).

### [MV-24] Comentarios y documentación desactualizados en código de seguridad
- Severidad: Baja
- Categoría: Mala práctica
- Ubicación: `lib.rs:931-933` ("`inicializar_logging` ... todavía no se llamó", pero no existe y el logger se inicia en `abrir`); `lib.rs:1127` ("no hay cerrar sesión todavía", pero sí lo hay); `PantallaPrimerArranque.kt:40-42` y `PantallaCambioObligatorio.kt:40` (mencionan `PantallaFijarPasswordInicial`, eliminada); `release.yml:162-168` (ver MV-03); `PantallaEscanearCedula.kt:411-419` (ver MV-08); `auditoria-calidad-2026-09.md` ("sin `!!`", pero `EstabilizadorLectura.kt:137` usa `!!`)
- Estado: Nuevo
- Descripción y evidencia: comentarios que afirman garantías que el código no da.
- Escenario de impacto concreto: quien mantenga el código confía en el comentario (por ejemplo, "el APK ya cifra") y no corrige el problema.
- Referencia externa: —
- Recomendación concreta: corregirlos junto con MV-03 y MV-08.

### [MV-25] Base iOS abandonada
- Severidad: Info
- Categoría: Código muerto
- Ubicación: `mobile/ios/` (project.yml, `Sources/App/ContentView.swift`, `Scripts/build-rust-xcframework.sh`)
- Estado: Nuevo
- Descripción y evidencia: tiene un solo commit (2026-09-13). `ContentView` es texto estático ("listo para conectar al nucleo"), sin ninguna llamada a UniFFI. No hay job de CI. El script compila con el motor predeterminado (SQLCipher, OpenSSL vendorizado) y no tiene almacén seguro equivalente al Keystore, así que usaría los métodos heredados de MV-11.
- Escenario de impacto concreto: da una falsa sensación de soporte multiplataforma y condiciona la limpieza de MV-11.
- Referencia externa: —
- Recomendación concreta: declararla explícitamente como "no mantenida" en `mobile/ios/README.md` o eliminarla. Si se retoma, diseñar desde el inicio Keychain y la variante `*_con_secreto`.

### [MV-26] TLS del núcleo independiente de la configuración de red de Android; pinning
- Severidad: Info
- Categoría: Seguridad
- Ubicación: `Cargo.toml` raíz:66 (`reqwest` con `rustls-tls`, que trae `webpki-roots 1.0.9` en `mobile/rust-core/Cargo.lock:2621`); `AndroidManifest.xml:26`
- Estado: Nuevo
- Descripción y evidencia: el tráfico HTTP del núcleo sale por código nativo. `usesCleartextTraffic="false"` no lo cubre ("there's no expectation that the Socket API honors this flag"), y la confianza TLS viene de las raíces de Mozilla compiladas en el `.so`, no del almacén de Android. Esto tiene un lado bueno (ignora CAs instaladas por el usuario) y un lado malo (las raíces envejecen con el APK). El HTTPS depende solo de que `base_url` sea `https://`, y en release no se puede cambiar por variable de entorno (solo en DEBUG, `AplicacionControlAcceso.kt:155-158`). Hacer pinning contra Supabase no es recomendable por la rotación de certificados.
- Escenario de impacto concreto: un APK de hace años falla la conexión TLS si una raíz vence o es retirada.
- Referencia externa: https://developer.android.com/guide/topics/manifest/application-element#usesCleartextTraffic ; https://developer.android.com/privacy-and-security/security-config (advertencia sobre pinning y claves de respaldo) ; MASVS-NETWORK-2
- Recomendación concreta: rechazar en Rust cualquier `base_url` que no empiece con `https://`. Evaluar `rustls-platform-verifier` para usar el almacén del sistema. No hacer pinning; mantener actualizado el APK (ver MV-18).

---

## Aspectos bien resueltos

- **Secreto del dispositivo**: AES-256-GCM con clave generada dentro de Android Keystore, IV aleatorio, escritura atómica con `fsync` y migración del archivo legado que después se borra (`SecretoDispositivoStore.kt`).
- **Manifiesto**: `allowBackup="false"`, `usesCleartextTraffic="false"`, una sola Activity exportada (la del launcher), sin WebView, sin intents implícitos, sin providers ni receivers propios, permisos mínimos y cámara opcional.
- **Release**: R8 activado con reglas para JNA, UniFFI, CameraX y ML Kit; la firma sale de un secreto de CI y el keystore no está en el repo; los builds debug usan `applicationIdSuffix` y apuntan a staging.
- **`FLAG_SECURE`** en release; los campos de contraseña y secreto usan `KeyboardType.Password`; las imágenes de cédula **nunca se escriben a disco** (todo queda en memoria; sin `cacheDir`/`ImageCapture`); el texto crudo del OCR solo se registra con `BuildConfig.DEBUG`.
- **Puente UniFFI**: `#![forbid(unsafe_code)]`; recuperación de `Mutex` envenenado; red **fuera** de `core_lock()` con conexión secundaria WAL; `sincronizacion_en_curso` serializa las sincronizaciones; errores tipados (`NucleoError`). Los bindings Kotlin generados están **sincronizados** con los 55 exports de `lib.rs` (verificado con diff), y el chequeo de checksum de UniFFI hace fallar las pruebas JVM de CI si se desincronizan.
- **Concurrencia en Kotlin**: toda la E/S del núcleo va a `Dispatchers.IO` inyectable (salvo MV-09); no hay `GlobalScope` ni `runBlocking`; hay un `Mutex` para mutaciones y guardas contra doble toque; los servicios se detienen en `ON_STOP` y la sesión se cierra tras 2 min en segundo plano; se usa un `ViewModelStore` por sesión. Ningún ViewModel guarda `Context` o `Activity` (`GestorTema` solo retiene `SharedPreferences`).
- **Parsers OCR**: expresiones regulares precompiladas; MRZ con dígitos ICAO 7-3-1, compuesto que incluye el campo opcional, soporte del número extendido y del DIMEX costarricense verificado con un caso real; la inyección de la fecha actual permite probarlo. Tienen buena cobertura de pruebas.
- **CI**: acciones fijadas por SHA, `persist-credentials: false`, `permissions: contents: read`, `cargo audit`/`clippy`/`fmt` del crate móvil y pruebas JVM contra el núcleo real.
