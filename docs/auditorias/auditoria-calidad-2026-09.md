# Auditoría de calidad — acceso_CLI (2026-09)

> Estado: consolidado, listo para retomar. Cubre la auditoría completa de la
> app (Android/Kotlin/Compose + núcleo Rust) y el problema puntual del
> secreto de dispositivo sin cifrar, incluyendo el diagnóstico externo
> recibido y su verificación. Nada de lo de la sección 3 está implementado
> todavía — es la decisión pendiente para retomar en la PC.

## 1. Resumen ejecutivo

- **Núcleo Rust (`control_acceso`)**: sin hallazgos críticos ni importantes.
  Argon2id correcto, cero inyección SQL, cero `unsafe`, manejo de panics y
  concurrencia sólido, buena separación de capas, cobertura de tests amplia
  sobre la lógica crítica (acceso, PRAIND/gafete).
- **Android/Kotlin/Compose**: un solo hallazgo real de peso — el secreto de
  dispositivo queda en **texto plano** en el APK real pese a que varios
  comentarios en el código afirman que se cifra. Resto de la app limpio:
  sin datos sensibles en logs, sin inyección SQL, coroutines bien atadas al
  lifecycle, sin archivos "Dios", sin `GlobalScope`, `LazyColumn` con `key`
  estable.
- **El módulo de lectura de documentos de identidad** (MRZ/OCR) se auditó
  aparte, en la misma sesión, y ya tiene sus fixes aplicados y commiteados
  (ver commits `fff06d2`, `920c32e` en la rama
  `claude/ocr-cedula-licencia-refinement-sud0by`) — no repetido acá.

## 2. El hallazgo: secreto de dispositivo sin cifrar en Android

### 2.1 Qué es y por qué importa

El "secreto de dispositivo" (`dispositivo-nube.secret`) es la credencial que
autentica todo el teléfono/tablet contra Supabase. En el APK real de Android
queda guardado **en texto plano** en el almacenamiento de la app, pese a que:

- `MainActivity.kt:45-47`, `NubeViewModel.kt:34-36`, `SincronizacionPeriodica.kt:31-32`
  afirman en comentarios que "cifra el secreto de dispositivo en disco".
- El mecanismo de cifrado (AES-256-GCM, `src/nube/credenciales.rs`) existe y
  está implementado — simplemente nunca se activa para el build móvil.

Mitigantes ya existentes: sandbox de Android (otro proceso normal no puede
leer el archivo) y `allowBackup="false"` (no se va en el backup automático
de Google). El riesgo real es: dispositivo rooteado, acceso físico
prolongado, o extracción forense — exactamente el escenario de una tablet
desatendida en la entrada de una instalación todo el día.

### 2.2 Por qué está apagado — no es un descuido, fue una reversión deliberada

Secuencia real de commits (2026-09-06/07):

1. **`daaa19b`** — activó el cifrado, pero tenía un bug real: tres funciones
   de `AppCore` (`refrescar_catalogo_sin_sesion`, `sincronizar_con_nube`,
   `usuario_sigue_activo_remoto`) nunca recibían el `ANDROID_ID` necesario
   para descifrar. La sincronización periódica y el reintento de login
   estaban rotos desde que se activó el cifrado. **Esto se arregló.**
2. **`eee2eea`** (11 min después) — apareció un **segundo problema
   distinto**: el mismo emulador dejó de poder leer su propio secreto
   cifrado, **sin causa clara identificada en su momento**. Decisión:
   revertir. `cifrado-secreto-dispositivo-portable` sale de las features de
   `mobile/rust-core`. Cita textual del commit:
   > "...decisión explícita: no vale la pena la complejidad para lo que
   > protege."
3. **`2212f0f`** — documenta la reversión en `docs/planes-implementados/plan-panel-administrativo-web.md`.
4. **`57c24fa`** — registra que una auditoría detectó que el feature sigue
   apagado y lo deja como pendiente de decisión.

Desktop (`desktop/src-tauri/Cargo.toml`, feature `cifrado-secreto-dispositivo`,
Machine GUID de Windows) **nunca tuvo este problema** — sigue activo tal
cual, sin tocar.

Única pista dejada para el futuro
(`docs/planes-implementados/plan-panel-administrativo-web.md:274-321`):
> "...antes de reactivarlo habría que probarlo bien en un dispositivo real
> desde el principio, no sólo unitarias."

El fallo original ocurrió en emulador, nunca se probó después en físico.

### 2.3 Estado actual del código (confirmado)

- `mobile/rust-core/Cargo.toml:26` → `features = ["nube"]` únicamente, sin
  el feature de cifrado.
- `desktop/src-tauri/Cargo.toml:26` → sí tiene su cifrado activo, intacto.
- El cableado completo para reactivarlo en Android ya existe (Kotlin ya lee
  `ANDROID_ID` y se lo pasa a Rust) — activar el feature era, en teoría,
  una línea en el `Cargo.toml`. La sección 3 explica por qué eso ya no es
  el plan.

## 3. Diagnóstico externo recibido y verificación propia

Se consultó a otra IA con el historial de commits de arriba. Resumen de su
análisis, con lo que se verificó como cierto y lo que queda como hipótesis:

### 3.1 Lo verificado como real

**`MainActivity.kt:58`** tiene, tal cual, exactamente lo que señalaron:

```kotlin
val identificadorDispositivo =
    Settings.Secure.getString(contentResolver, Settings.Secure.ANDROID_ID) ?: ""
```

**Matiz importante que su análisis no tenía**: esto no es un descuido
silencioso — el comentario justo arriba (líneas 53-56) ya documenta que es
un fallback defensivo a propósito ("en la práctica nunca es null desde API
26, pero el fallback a `""` evita un `!!` que podría tirar en el arranque
por un caso límite de un fabricante raro"). O sea: quien lo escribió ya
sabía que era una apuesta. Su punto sigue siendo válido igual —
`"nunca pasa"` no es lo mismo que `"nunca pasó"`, y si `getString` devolvió
algo inesperado durante la sesión de pruebas de septiembre, el síntoma
("mismo emulador, ya no puede leer su propio secreto") encajaría exacto:
cifra con `SHA256("")` en un momento, falla la lectura o se reinstala, y
después ya no calza con el identificador real.

### 3.2 Diagnóstico técnico — evaluado como sólido

- Descarta AES-GCM/el nonce como culpable del "dejó de leerse" — razonamiento
  correcto (el nonce va dentro del propio archivo, no depende de estado
  externo).
- La explicación de por qué `ANDROID_ID` puede variar por combinación de
  dispositivo + usuario + firma de la app desde Android 8 es consistente
  con el comportamiento documentado de la plataforma.
- Descarta el backup automático de Android como causa (`allowBackup=false`
  ya estaba activo antes del incidente) — correcto, confirmado en el repo.

### 3.3 Recomendación — evaluada como correcta, con una implicación de
arquitectura que su respuesta no mencionó

Recomienda migrar de "ANDROID_ID → SHA-256 → clave AES" a **Android
Keystore** (clave AES generada y resguardada por el sistema, nunca sale en
claro, puede estar respaldada por hardware). Esto no es solo su opinión —
es el mecanismo que Google documenta específicamente para este problema, y
elimina de raíz toda la clase de bug "identificador distinto → clave
distinta → no descifra" en vez de repararla caso por caso.

**Lo que agregué en la verificación**: si se usa Keystore de verdad, el
cifrado/descifrado tiene que vivir **en Kotlin**, no en Rust — la garantía
del Keystore es que el material de la clave nunca sale de ahí; exportarlo a
Rust para reusar el AES-GCM que ya existe anularía esa garantía. Eso
implica:

- Android usaría `androidx.security.crypto.EncryptedFile` (librería oficial
  de Jetpack que envuelve Keystore para exactamente este caso) en Kotlin.
- Rust dejaría de cifrar nada para Android — solo recibiría el secreto ya
  en texto plano desde Kotlin (que ya lo descifró él mismo).
- Desktop se queda intacto (Machine GUID vía Rust, sin tocar).
- Compatible con el modelo operativo ya aceptado: un Keystore también se
  pierde en reset de fábrica o reinstalación, igual que el esquema viejo —
  "pegar el secreto una vez más" ya era la expectativa documentada.

### 3.4 Plan de diagnóstico alternativo que también propuso (no ejecutado)

Antes de cambiar arquitectura, instrumentar temporalmente (sin loguear el
`ANDROID_ID` completo, solo un fingerprint corto) para confirmar si el
identificador cambió entre el cifrado y el fallo, probando: reinicio de
AVD, actualización de APK con la misma keystore de debug, cambio de firma
deliberado, y repetición en un dispositivo físico real. Queda como opción
si se prefiere confirmar la causa exacta antes de reescribir el mecanismo.

## 4. Decisión pendiente (para retomar en la PC)

Dos caminos, no mutuamente excluyentes:

**A. Ir directo a Android Keystore + `EncryptedFile`** (recomendado por la
otra IA y por mí): resuelve el problema de raíz, elimina la dependencia de
`ANDROID_ID`, alineado con la guía oficial de Android para este caso de uso.
Alcance: cambios en Kotlin (nuevo código de cifrado con Keystore), sin
tocar Rust ni desktop. Más grande que "una línea en el Cargo.toml", pero
acotado.

**B. Primero correr el diagnóstico instrumentado** (sección 3.4) para
confirmar la causa exacta del incidente de septiembre antes de decidir si
vale la pena seguir con el esquema `ANDROID_ID`+Rust o saltar directo a
Keystore.

**Mi recomendación**: A directamente — aunque se confirmara que el `?: ""`
fue la causa exacta, arreglar solo eso deja el diseño dependiendo de
`ANDROID_ID` para siempre, con la misma clase de fragilidad latente
(cambios de firma, resets, inconsistencias de fabricante). Keystore la
elimina en vez de parchearla.

## 5. Otros hallazgos de la auditoría (Android, sin acción pendiente)

Todo lo siguiente se revisó y salió limpio — se deja documentado como
evidencia de la auditoría, no como pendiente:

- Hash de contraseñas: Argon2id, parámetros razonables (m=19456, t=2, p=1).
- Sin `Log.d`/logging de cédulas, contraseñas ni secretos en ningún archivo
  Kotlin (fuera del módulo de OCR, auditado aparte).
- Sin inyección SQL en ninguna query (`format!` solo arma fragmentos
  estáticos, valores siempre bindeados).
- `AndroidManifest.xml`: permisos mínimos (`INTERNET`, `CAMERA`,
  `ACCESS_NETWORK_STATE`), sin `usesCleartextTraffic`, BASE_URL es HTTPS,
  la "apikey" de Supabase es la clave pública (`sb_publishable_...`), no un
  secreto.
- Coroutines: `viewModelScope`/`rememberCoroutineScope` en todos lados, sin
  `GlobalScope`; `NubeRealtime`/`SincronizacionPeriodica` atados a
  `ON_START`/`ON_STOP` con cancelación explícita en `onDispose`.
- Sin `!!` en código propio; `catch` específicos (`NucleoException`), no
  `Exception` genérico que trague `CancellationException`.
- Debounce de 300ms ya aplicado en búsquedas (`ActivosViewModel`,
  `HistorialViewModel`), con cancelación del `Job` anterior.
- Sin archivo "Dios": el más grande (`PantallaActivos.kt`, 572 líneas) está
  bien descompuesto en Composables privados de una sola responsabilidad.
- Compose: `LazyColumn` con `key` estable en todos lados, keys de
  `LaunchedEffect`/`DisposableEffect` correctas, `rememberSaveable` usado
  con criterio explícito.

## 6. Núcleo Rust — sin hallazgos (resumen)

- Contraseñas: Argon2id vía crate `argon2`, salt aleatorio (`OsRng`),
  formato PHC estándar (`src/services/password.rs`).
- Cero `unsafe` en todo el crate (raíz + `mobile/rust-core`).
- Cero inyección SQL (verificado en `database/queries/*`).
- ~530 `.unwrap()/.expect()/panic!()` en el crate, ~100% en código de test.
  Único caso en producción (`cli/formulario.rs:410`) protegido por
  validación previa garantizada.
- Manejo correcto de `Mutex` envenenado en la frontera FFI de UniFFI.
- Patrón de "conexión secundaria" para no bloquear la DB durante I/O de red
  (requiere `journal_mode=WAL`, documentado).
- Sin fuga de features: `nube` correctamente gateado, sin código de
  escritorio/nube colándose en el build móvil sin el feature activo.
- Cobertura de tests amplia (37 archivos) sobre lógica crítica de negocio.

## 7. Nota de metodología

La auditoría se hizo con dos agentes en paralelo (uno para Android/Kotlin,
otro para el núcleo Rust), cada uno instruido a verificar afirmaciones
normativas contra documentación oficial en vez de dar solo opinión. El
agente de Android no tuvo acceso a búsqueda web en su corrida y lo marcó
explícitamente — sus afirmaciones sobre Android se basan en conocimiento
previamente verificado, no en una re-verificación en vivo. El diagnóstico
externo sobre el secreto de dispositivo (sección 3) sí se verificó en vivo
contra el código real del repo antes de aceptarlo (confirmado el `?: ""`
exacto en `MainActivity.kt:58`).
