# Arquitectura del OCR móvil

## Objetivo

El OCR convierte frames de CameraX en un `DocumentoDetectado` verificable.
No autoriza entradas ni salidas: las reglas de acceso siguen perteneciendo
al núcleo Rust. Esta separación evita que reconocer un número equivalga por
accidente a permitir una operación.

## Flujo

1. `PantallaEscanearCedula` solicita permiso y abre una sesión de cámara
   (ML Kit se precarga mientras la cámara arranca).
2. `ImageAnalysis` conserva sólo el frame más reciente; entra a la tubería
   como máximo uno cada 80 ms y sólo si hay lugar (hasta 2 frames en
   proceso a la vez, `MAXIMO_EN_PROCESO`).
3. `analizarFrameOcr` recorta sobre los planos YUV sólo la región pedida:
   el recuadro guía o, si ya se vio el MRZ, su banda (`SeguidorBandaMrz`).
4. Se mide la nitidez y el reflejo del recorte (`CalidadFrame`). Un frame
   claramente más borroso que los recientes no pasa a ML Kit.
5. El `ImageProxy` se cierra APENAS se copia el recorte: la cámara prepara
   el frame siguiente mientras ML Kit procesa éste.
6. Texto y, si `PlanificadorLectores` lo pide, PDF417 corren EN PARALELO
   sobre el mismo recorte (sólo donde se acepta una cédula; en cada frame
   si el texto parece el reverso de la cédula anterior, uno de cada 3 si
   no, nunca con un MRZ en cuadro). Buscar el código no le quita frames al
   texto (p. ej. al carnet PRAIND).
7. Cuando terminan los dos, el resultado se procesa EN EL HILO DEL
   ANALIZADOR y se libera el lugar.
8. `EstabilizadorLectura` clasifica, extrae y vota por carácter entre
   frames (número, MRZ); el PDF417 confirma en una lectura.
9. Sólo el resultado se publica al hilo principal; la UI recibe el
   `DocumentoDetectado` completo, no únicamente un texto.
10. En modo continuo, `ActivosViewModel` termina la mutación SQLite/UniFFI
   antes de que la cámara acepte el siguiente gafete.

## Invariantes de concurrencia

- Recorte, calidad, clasificación, votación y cruces a Rust corren en el
  único hilo del analizador (`EstadoCamaraOcr.ejecutor`); ML Kit trabaja en
  sus propios hilos. Con 2 frames en vuelo, mientras uno se reconoce el
  otro se recorta, y los resultados se procesan de a uno en ese mismo hilo
  (pueden llegar en otro orden: la votación no depende del orden). Al hilo
  principal sólo va el estado de pantalla (`EstadoCamaraOcr.enPrincipal`).
- El lugar de un frame se reserva antes de recortar y se libera cuando
  terminan TODOS sus lectores (o si se descarta antes): nunca se forma una
  cola atrasada.
- `EstabilizadorLectura`, `EstabilizadorPorRepeticion`, `SeguidorBandaMrz`
  y `PlanificadorLectores` están sincronizados: la pantalla puede
  reiniciarlos desde el hilo principal.
- Un frame que ya estaba en proceso cuando se confirmó no vuelve a pintar
  "buscando" encima del resultado.
- `sesionActiva` invalida resultados de cámara u OCR posteriores al cierre.
- Los retrasos visuales usan una corrutina ligada a la composición; salir de
  la pantalla los cancela. No se usan `Handler.postDelayed` para acciones de
  negocio.
- `detectada.compareAndSet(false, true)` admite una sola confirmación por
  ciclo armado.
- Una salida iniciada pertenece al `viewModelScope`, no al Composable. Cerrar
  la cámara no cancela una escritura que ya pudo comenzar en Rust.
- `mutexMutaciones` serializa las salidas automáticas.
- El mismo gafete no vuelve a aceptarse por tiempo: debe desaparecer de la
  imagen durante varios frames. Esto evita repetir una salida sólo porque la
  persona dejó el carnet quieto frente a la cámara.

## Contrato del resultado

`DocumentoDetectado` conserva tipo, número, texto de búsqueda, nombre,
vencimiento, nacionalidad, origen y validez del checksum. El consumidor
elige qué campo usar; el escáner no destruye información antes de llegar al
dominio.

Un MRZ con checksum correcto confirma integridad de lectura, pero no implica
que su tipo esté soportado. `DESCONOCIDO` se rechaza expresamente.

## Fechas MRZ

ICAO usa años de dos dígitos. El siglo de nacimiento se calcula contra una
fecha inyectable: se elige 20XX sólo si no queda en el futuro; de lo contrario
se usa 19XX. No existe una constante anual que haya que editar al cambiar de
año. Toda fecha pasa por `LocalDate`, por lo que valores como 31/02 se
descartan.

## Estabilización

Cada frame vota carácter por carácter (`VotadorPorPosicion`, en Rust), con
peso según su nitidez. Se confirma cuando, en todas las posiciones, el
ganador tiene respaldo de `framesRequeridos` frames (con media unidad de
tolerancia para frames apenas menos nítidos) y le saca al menos media
unidad a la segunda opción. Con frames de peso 1 nunca es más permisivo que
exigir N lecturas iguales, incluido el modo gafete.

Una lectura que difiere del consenso en más de un par de caracteres es
OTRO documento: se empieza de cero y no se heredan nombres. Campos
opcionales que aparecen intermitentemente se acumulan para el mismo
documento. La ventana incluye frames sin candidato, de modo que tolera
ruido breve sin conservar indefinidamente lecturas antiguas.

El MRZ se vota por líneas completas y el consenso se valida con sus
dígitos verificadores (2 frames de respaldo, por la línea de nombres).
El PDF417 de la cédula anterior confirma en una sola lectura: el código
tiene corrección de errores y Rust valida la forma de los datos.

## PDF417 de la cédula anterior

Decisión del negocio (2026-09-28): se lee, tomando SÓLO cédula y nombre.
Kotlin entrega a Rust únicamente los primeros 91 bytes (hasta el final del
nombre) y pone en cero los bytes crudos, que incluyen las huellas; Rust
descifra sólo ese prefijo, valida y pone en cero lo que recibió. Nunca se
descifran fechas ni huellas, nada se guarda ni se registra. Ver
`mobile/rust-core/src/pdf417_cedula.rs`.

## Guía visual

La guía es estática. Los rectángulos de ML Kit usan coordenadas del frame y
`PreviewView.FILL_CENTER` aplica otro recorte. Dibujarlos directamente sobre
Compose produce desplazamientos según dispositivo y orientación. Sólo debe
reactivarse seguimiento dinámico cuando se use la transformación oficial de
CameraX entre ambos sistemas de coordenadas.

## Pruebas obligatorias

- Tests JVM de clasificación, extracción, checksum, siglo y fechas inválidas.
- Tests de estabilización ante campos intermitentes y texto ajeno.
- Tests del ViewModel con base temporal para comprobar la salida por gafete.
- `CI / test-android` compila Kotlin y ejecuta `testDebugUnitTest` en cada
  push `fix/**` y en cada pull request hacia `main`.
- Prueba física final: enfoque, reflejos, distancia, sonido y vibración en el
  Samsung A25 con documentos reales. Esto complementa, no sustituye, CI.

## Decisiones deliberadas

- El modelo latino de ML Kit se empaqueta en el APK para evitar una descarga
  durante el primer uso.
- Se conserva `STRATEGY_KEEP_ONLY_LATEST`: para OCR en tiempo real interesa
  el frame actual, no procesar una cola atrasada.
- La vibración usa `Vibrator`/`VibrationEffect` con atributos de alarma
  (`EscaneoCompartido.kt`), no la háptica semántica de Compose: ésta no
  vibraba en el Samsung real con "Interacciones táctiles" y no deja
  distinguir éxito de error. Por eso el manifiesto declara `VIBRATE`
  (permiso normal, sin diálogo).
- El recorte al recuadro guía lee sólo las filas necesarias directo de los
  `ByteBuffer` de cada plano YUV; nunca se copian los planos enteros.
- La nitidez se compara sólo entre recortes del mismo tipo de región (la
  banda MRZ, texto denso, parece más nítida que la tarjeta entera) y nunca
  se descartan más de 3 frames seguidos: un umbral relativo mal calibrado
  no puede dejar al escáner sin leer.
- El APK sólo incluye `arm64-v8a` y `x86_64`, las ABI para las que existe el
  núcleo Rust (APK de debug de 94,5 MB a 64,7 MB).
- Linterna en las 4 pantallas de escaneo (noche en la portería, placas en
  sombra).
- Métricas por sesión sólo en debug y sin datos personales:
  `adb logcat -s OcrMetricas`.
- El número de cédula del frente exige primer dígito 1-9 (provincia). Las
  placas corrigen como máximo UNA confusión letra/dígito según la posición
  (formato fijo: letras, luego dígitos), porque no tienen dígito
  verificador que confirme una corrección.
- Un escaneo abre sólo el formulario del contratista cuya cédula es
  EXACTAMENTE el número leído (`CoincidenciaEscaneo.kt`); un único
  resultado de la búsqueda parcial ya no alcanza.
