# Arquitectura del OCR móvil

## Objetivo

El OCR convierte frames de CameraX en un `DocumentoDetectado` verificable.
No autoriza entradas ni salidas: las reglas de acceso siguen perteneciendo
al núcleo Rust. Esta separación evita que reconocer un número equivalga por
accidente a permitir una operación.

## Flujo

1. `PantallaEscanearCedula` solicita permiso y abre una sesión de cámara.
2. `ImageAnalysis` conserva sólo el frame más reciente.
3. ML Kit entrega texto; el `ImageProxy` se cierra siempre al completar.
4. `EstabilizadorLectura` clasifica, extrae y exige checksum MRZ o repetición.
5. La UI recibe el `DocumentoDetectado` completo, no únicamente un texto.
6. En modo continuo, `ActivosViewModel` termina la mutación SQLite/UniFFI
   antes de que la cámara acepte el siguiente gafete.

## Invariantes de concurrencia

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

La identidad estable de un candidato es `tipo + número normalizado`. Campos
opcionales que aparecen intermitentemente no reinician el conteo. La ventana
incluye frames sin candidato, de modo que tolera ruido breve sin conservar
indefinidamente lecturas antiguas.

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
- Se usa una sola háptica semántica de Compose. La vibración cruda duplicaba
  el efecto y obligaba a declarar `VIBRATE` sin necesidad.
