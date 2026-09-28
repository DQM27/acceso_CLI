# Auditoría del OCR móvil: cédulas, documentos y placas (2026-09-28)

Alcance: todo el camino cámara → texto → dato de dominio en Android
(`CamaraOcr.kt`, `RecorteImagenOcr.kt`, `EstadoCamaraOcr.kt`,
`EstabilizadorLectura.kt`, `LectorDocumentosIdentidad.kt`, `TextoCedula.kt`,
`MrzParser.kt`, `LectorVehiculoRuta.kt`, `LectorComprobanteRuta.kt`,
`LectorCarnetKof.kt`, `EncuadreAdaptativo.kt`, las 4 `PantallaEscanear*.kt`)
y el parser MRZ en Rust (`mobile/rust-core/src/mrz.rs`). iOS no tiene OCR
todavía (`mobile/ios/Sources` sólo trae el esqueleto de la app).

## Veredicto corto

La base está bien hecha: recorte al recuadro guía sobre YUV sin pasar por
ARGB, `KEEP_ONLY_LATEST` con límite de ~6-7 frames/s, invalidación de
callbacks con `sesionActiva`, MRZ con dígito verificador y corrección de
confusables guiada por checksum (en Rust, con buenos tests), confirmación
por repetición en ventana deslizante, `FLAG_SECURE` en release, ninguna
imagen se guarda y el texto crudo sólo se loguea en debug.

Lo que falta no es "rehacerlo", sino cerrar cuatro frentes:

1. **Seguridad**: la salida automática por gafete confía en un número sin
   dígito verificador (S-2) y un escaneo podía abrir el formulario de la
   persona equivocada (S-1, corregido).
2. **Exactitud**: se usa sólo el texto plano de ML Kit y se tira la
   geometría y la confianza que ML Kit ya entrega (E-3); las placas no
   corregían confusiones letra/dígito (E-2, corregido).
3. **Rendimiento**: se copiaban los planos YUV enteros en cada frame (R-1,
   corregido) y el post-proceso corre en el hilo principal (R-3).
4. **Arquitectura**: el MRZ ya está en Rust pero la clasificación y
   extracción siguen en Kotlin (A-1), migración que la auditoría del
   2026-09-25 ya dejó priorizada para antes de activar iOS.

## Corregido en este cambio

| Id | Tipo | Qué pasaba | Qué se hizo |
|---|---|---|---|
| R-1 | Rendimiento | `recortarParaOcr` copiaba los 3 planos YUV **enteros** a arreglos (~4 MB por frame a 1080p) y recién ahí recortaba el recuadro, que es cerca de un cuarto del frame; además `BuffersOcrReutilizables` retenía esos ~4 MB entre frames. | `recortarYuvANv21` lee directo de cada `ByteBuffer` sólo las filas del recorte (sobre `duplicate()`, sin mover el búfer original). Se elimina `BuffersOcrReutilizables`. Tests nuevos: última fila de croma sin relleno y búfer con posición inicial ≠ 0. |
| R-2 | Rendimiento | `EstabilizadorLectura` clasificaba el texto y `leerDocumentoDeTexto` lo volvía a clasificar (todas las regex otra vez) en el mismo frame. | `leerDocumentoDeTexto(texto, tipo)` acepta el tipo ya calculado. |
| E-1 | Exactitud | `extraerCedulaDeTexto` aceptaba números que empiezan en 0 (ninguna provincia es 0). En el reverso de la cédula anterior devolvía el número de control `001234567`, quien llamaba lo descartaba y se perdía la cédula real que venía después. | Las regex exigen primer dígito 1-9 y siguen buscando. Tests nuevos. |
| E-2 | Exactitud | Placas: una `S` por `5`, `O` por `0` o `8` por `B` hacía fallar la placa o la convertía en otra cosa. | Corrección según la posición (letras primero, dígitos después) con **una** corrección como máximo por placa (sin dígito verificador, más sería adivinar). Entre carga y particular gana la que necesitó menos correcciones. Tests nuevos, incluido que un número de unidad `228051` no se convierta en placa. |
| S-1 | Seguridad | Tras escanear, si la búsqueda (parcial) devolvía **un solo** contratista se abría su formulario aunque su cédula no fuera la leída. Con un dígito mal leído en el frente (sin checksum) podía abrirse el de otra persona. | `CoincidenciaEscaneo.kt`: con un número se exige cédula **exacta**; por nombre (gafete In House sin cédula) se mantiene el resultado único. Tests nuevos. |
| D-1 | Documentación | `arquitectura-ocr.md` decía que se usaba la háptica de Compose y que `VIBRATE` sobraba (el código hace lo contrario, a propósito); el doc de `analizarCedula` decía que se analizaba el frame completo sin recortar. | Corregidos. |

## Segunda entrega (mismo día): rediseño guiado por la investigación

Después de `investigacion-lectura-documentos-2026-09-28.md` se implementó:

| Id | Qué | Dónde |
|---|---|---|
| — | **PDF417 de la cédula anterior**: sólo cédula y nombre, prefijo de 91 bytes, bytes crudos (con huellas) puestos en cero | `rust-core/src/pdf417_cedula.rs`, `LecturaFrame.kt` |
| — | **Votación por carácter** entre frames (número, MRZ y placas), ponderada por nitidez; otra persona empieza de cero | `rust-core/src/votacion.rs`, `EstabilizadorLectura.kt`, `EscaneoCompartido.kt` |
| — | **Filtro de calidad** (nitidez y reflejo) antes de ML Kit; aviso de reflejo medido | `CalidadFrame.kt` |
| R-3 | Post-proceso **en el hilo del analizador**; sólo el resultado va al principal | `CamaraOcr.kt`, pantallas |
| R-4 | El encuadre reutiliza la orientación del estabilizador (una clasificación por frame) | `EncuadreAdaptativo.kt` |
| E-3 (parcial) | La geometría de ML Kit se usa para **acotar el recorte a la banda del MRZ** | `SeguidorBandaMrz` |
| E-4 (parcial) | **Linterna** en las 4 pantallas (zoom sigue pendiente) | `BotonLinterna` |
| — | **Precarga** de ML Kit mientras arranca la cámara | `EstadoCamaraOcr` |
| — | **Métricas** por sesión en debug, sin datos personales (`adb logcat -s OcrMetricas`) | `MetricasOcr` |
| — | APK sólo con las ABI del núcleo Rust: debug de 94,5 MB a 64,7 MB | `build.gradle.kts` |

Sigue pendiente: S-2 (gafetes con código o confirmación), el resto de E-3
(reconstruir renglones por geometría y usar la confianza), zoom, y A-1
(migrar clasificación y extracción a Rust).

## Tercera entrega: peso del APK, placa CL y telemetría

- **Sólo `arm64-v8a`.** El APK únicamente se instala en el Samsung A25; se
  quitó `x86_64` del APK y de `release.yml` (−18 MB en debug).
- **Núcleo Rust optimizado.** Perfil `release` propio en
  `mobile/rust-core/Cargo.toml` (`lto = "thin"`, `codegen-units = 1`,
  `strip = true`): `.so` de 11,2 MB a 8,0 MB. `panic` se deja en `unwind`
  a propósito: UniFFI convierte un pánico en excepción de Kotlin en vez de
  cerrar la app.
- **Placa "CL" apilada.** Las letras una sobre otra se leen como "E"; con
  exactamente 6 dígitos (formato de carga liviana) se restituye "CL"
  (hoy en `lectura_documentos/vehiculo.rs`, tras integrar A-1).
- **Build `diagnostico` y telemetría.** El teléfono no permite depuración
  USB: un build compilado como release (R8), firmado con la llave de debug y
  apuntado a staging manda métricas técnicas (arranque, frames trabados,
  memoria, posibles fugas, CPU, batería, temperatura, red, llamadas al
  núcleo, sesiones OCR, motivos de cierre del proceso, StrictMode) a
  `telemetria_diagnostico`. Sin datos personales. Detalle en
  `mobile/android/docs/telemetria-diagnostico.md`. APK `diagnostico`:
  33,2 MB.

## Entrega A-1 y E-3: lectura de documentos en Rust y renglones visuales

- **A-1 resuelto.** Clasificación, extractores de cada documento, búsqueda
  del MRZ dentro del texto, placas, comprobante de ruta, carnet KOF y el
  estabilizador de documentos pasaron a `mobile/rust-core/src/lectura_documentos/`
  (el estabilizador como objeto UniFFI, `EstabilizadorDocumento`). Kotlin
  conserva sus funciones de siempre como envoltorios de una línea. Se
  portó con el comportamiento exacto: los 268 tests JVM existentes pasaron
  SIN cambios contra el código de Rust, y se agregaron 42 tests Rust. Las
  diferencias de semántica entre `java.util.regex` y `regex` (sin
  lookaround, `\d` Unicode, `lines()`, desempates de `maxByOrNull`) están
  resueltas y comentadas en el código.
- **E-3 resuelto (con umbrales por calibrar).** `analizarFrameOcr` pasa al
  núcleo las líneas con sus cajas y las palabras con su confianza. El
  núcleo arma renglones visuales (misma altura, izquierda a derecha,
  corrigiendo la inclinación con las palabras de cada línea) y descarta
  palabras con confianza < 0,25. Los lectores prueban primero esa versión y
  después el texto original, así que no puede leer peor que antes. El MRZ
  por geometría (punto 3) queda cubierto por los renglones: sus líneas
  quedan consecutivas y una línea partida se une. Test de extremo a
  extremo: una cédula anterior cuyo texto original asignaba "GOMEZ" como
  nombre ahora sale "JUAN CARLOS" / "GOMEZ VARGAS".
- **Pendiente de estos puntos:** calibrar con muestras reales del A25 la
  tolerancia de altura (media altura de letra) y el umbral de confianza.
  El `EstabilizadorPorRepeticion` genérico de las pantallas simples queda
  en Kotlin a propósito (recibe funciones de Kotlin; ya vota con el
  `VotadorPorPosicion` de Rust).

## Pendiente, en orden de prioridad

### Seguridad

- **S-2 (alta) · Salida automática por gafete sin dígito verificador.**
  Ya reconocido como MV-06: tres lecturas iguales de "CRC - 12" registran
  una salida real sin confirmación. Un 3↔8 o 1↔7 consistente bajo el mismo
  reflejo cierra el ingreso de otra persona. La solución de fondo es
  física: imprimir en el gafete un **código QR o Code 128** y leerlo con
  ML Kit Barcode Scanning (`com.google.mlkit:barcode-scanning`, modelo
  empaquetado): se lee en un solo frame, a más distancia, sin confusiones
  de caracteres, y el formato puede llevar un dígito verificador. Mientras
  tanto, la mitigación sin reimprimir es pedir confirmación (un toque) en
  la salida por gafete. *Decisión del negocio: reimprimir gafetes o no.*
- **S-3 (media) · Documento vencido sólo avisa.** El código lo marca como
  pendiente ("un mecanismo que sí bloquee queda para otra pasada"). Si se
  decide bloquear, la regla debe vivir en el núcleo Rust (mismo criterio
  que el resto de reglas de acceso), no en la pantalla.
- **S-4 (baja) · Frente de cédula sin checksum.** Se confirma con 2
  lecturas iguales; el formulario lo revisa una persona, así que el riesgo
  es acotado. Para cualquier flujo que llegue a automatizarse, exigir el
  MRZ del reverso (checksum) en vez del frente.
- **Bien resuelto, no tocar:** `FLAG_SECURE` en release, `allowBackup=false`
  + `dataExtractionRules`, texto OCR en logcat sólo en debug, Sentry sólo
  recibe posición y carácter de cada corrección MRZ (sin datos personales),
  ninguna imagen se persiste.

### Exactitud

- **E-3 (alta, RESUELTO salvo calibración; ver "Entrega A-1 y E-3") · Usar la estructura de ML Kit, no sólo `Text.text`.**
  ML Kit v2 entrega `TextBlock → Line → Element` con `boundingBox`,
  `angle` y `confidence`. Hoy se usa sólo el texto plano, cuyo orden de
  bloques cambia entre frames; varios parches existen por eso (columnas
  del DIMEX pegadas, etiquetas y valores de la cédula en bloques separados,
  MRZ partido en dos bloques). Propuesta:
  1. Reconstruir **renglones visuales**: agrupar `Line` por solapamiento
     vertical y ordenarlas por X. El texto resultante es estable y los
     extractores por etiqueta se simplifican.
  2. Descartar `Element` con `confidence` baja antes de extraer números.
  3. Para el MRZ, tomar las líneas por geometría (las 2-3 inferiores, de
     ancho parecido), no por su forma de texto.
  Requiere muestras reales del A25 para fijar umbrales (el volcado debug
  `OcrLectura` sirve, agregándole las coordenadas).
- **E-4 (media) · Linterna y zoom.** No hay botón de linterna ni zoom: de
  noche en la portería y con placas a distancia es lo que más limita la
  lectura. CameraX lo da directo (`camera.cameraControl.enableTorch()`,
  `setLinearZoom()`); hace falta conservar el `Camera` que devuelve
  `bindToLifecycle`.
- **E-5 (baja) · Placas particulares numéricas antiguas** (sólo dígitos) se
  leen como número de unidad. Aceptable para la flota actual; el campo es
  editable.
- **E-6 (baja) · MRZ:** ampliar los sustitutos de `<` (hoy `K`, `«`, `(`,
  `[`, `{`) según lo que muestren los eventos de corrección en Sentry.
  Seguro de ampliar porque el checksum descarta lo que no calce.
- **E-7 (baja) · MRZ TD2** (2 × 36) no está soportado. Sólo importa si
  llegan documentos extranjeros con ese formato.

### Rendimiento

- **R-3 (media) · Post-proceso en el hilo principal.** Los listeners de
  ML Kit corren en `ejecutorPrincipal`, así que la clasificación (2 veces
  por frame: encuadre + estabilizador), las regex de extracción y las
  llamadas JNI a `leerMrz` ocupan el hilo de UI. Se puede usar el mismo
  ejecutor de análisis (ya serializado: CameraX no entrega otro frame
  hasta cerrar el `ImageProxy`) y publicar al principal sólo el estado de
  pantalla y los efectos (vibración, sonido, `launch`).
- **R-4 (baja) · Clasificación repetida en el encuadre.** El
  `ControladorEncuadre` vuelve a clasificar el texto; podría recibir el
  tipo que ya calculó el estabilizador.

### Arquitectura Rust / Kotlin

- **A-1 · (RESUELTO; ver "Entrega A-1 y E-3") Completar la migración de lectura de documentos a Rust.**
  Estado actual: el MRZ (checksum, confusables, siglos) ya está en Rust;
  clasificación, extractores por regex, estabilizador y lectores de placa,
  comprobante y carnet KOF siguen en Kotlin (~1 700 líneas con comentarios). Es lo que la
  auditoría del 2026-09-25 (§6 y punto 8) dejó priorizado. Moverlo a un
  módulo `lectura_documentos` del núcleo, expuesto como
  `leer_documento(texto, modo)` y un `Estabilizador` UniFFI, deja en Kotlin
  sólo CameraX, ML Kit, el recorte YUV y la UI. Gana: iOS lo recibe
  hecho, escritorio podría reutilizarlo, vigencia y mayoría de edad usan
  el reloj del núcleo, y una sola suite de tests. El costo FFI (un
  `String` por frame) es despreciable frente a ML Kit. Hacerlo **antes**
  de E-3 obligaría a portar dos veces; conviene definir primero si el
  contrato será texto plano o renglones con geometría.
- **A-2 (baja) · Nombres.** `analizarCedula` lo usan las 4 pantallas
  (placas y comprobantes incluidos); `analizarFrameOcr` describiría mejor
  lo que hace.

## Verificación

- Los tests JVM de la lógica pura del OCR (recorte, placas, cédula,
  documentos, estabilizador, MRZ vía el núcleo Rust compilado para el
  host y selección por escaneo) se corrieron localmente: 142 tests, 0
  fallos.
- `CamaraOcr.kt`, `PantallaEscanearCedula.kt` y `ActivosViewModel.kt`
  dependen de Android y se compilan en `CI / test-android`
  (`testDebugUnitTest`).
- Tercera entrega: 286 tests JVM, 0 fallos; `compileDiagnosticoKotlin` y
  `assembleDiagnostico` (R8) sin errores ni advertencias; núcleo Rust con
  `cargo fmt`, `clippy -D warnings` y 78 tests en verde. Inserción en
  `telemetria_diagnostico` probada con la llave publicable (201), y
  lectura/borrado con esa llave rechazados (401).
- Entrega A-1 y E-3: 271 tests JVM y 120 tests Rust, 0 fallos;
  `cargo clippy -D warnings` limpio; bindings Kotlin regenerados y
  verificados con el mismo comando que CI; `compileDebugKotlin` sin
  advertencias.
- Integración de las dos entregas (rama `claude/optimizacion-build`, con la
  regla CL portada a Rust): 289 tests JVM y 121 tests Rust, 0 fallos;
  `compileDiagnosticoKotlin` sin advertencias; clippy limpio y bindings al
  día.
- Falta la prueba física en el Samsung A25: cédula (frente y reverso),
  DIMEX, licencia, gafete continuo y placas, de día y de noche.
