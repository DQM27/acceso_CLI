# Plan: Refinamiento de OCR y escaneo de documentos (Android)

> Estado: **planificación** — sin cambios de código todavía. Este documento se irá
> refinando en conversación antes de implementar. Última actualización: 2026-09-07.

## 1. Contexto y problema actual

La app (`PantallaEscanearCedula.kt`, ML Kit Text Recognition + CameraX) hoy solo
reconoce la cédula nacional costarricense con una única regex genérica:

```kotlin
fun extraerCedulaDeTexto(texto: String): String? {
    val normalizado = texto.replace(Regex("[^0-9\\n -]"), " ")
    Regex("""\b\d[- ]?\d{4}[- ]?\d{4}\b""").find(normalizado)
        ?.let { return it.value.filter(Char::isDigit) }
    return Regex("""\b\d{9}\b""").find(normalizado)?.value
}
```

No existe:
- Clasificación de tipo de documento (no distingue cédula nacional, cédula de
  residencia/DIMEX, licencia nacional, licencia de extranjero).
- Parseo de MRZ (reverso de la cédula de residencia).
- Validación de estabilidad de lectura (el OCR "dispara" antes de que el usuario
  termine de acomodar el documento).
- UI de guía interactiva (el marco de captura es un rectángulo fijo).
- Recorte del área de análisis al rectángulo guía (analiza el frame completo).

Casos reales que motivan esto (ver adjuntos del hilo):
- Cédula de residencia (DGME): el "Documento No." y el "Expediente No." son
  ambos números de 9-12 dígitos en el mismo campo visual → riesgo de leer el
  incorrecto sin un extractor específico por tipo.
- Licencia de conducir de extranjero: mismo número de documento pero con
  prefijo `DM-` — señal clara de que es un documento migratorio, no cédula
  nacional de 9 dígitos.
- Reverso de la cédula de residencia trae **MRZ tipo TD1** (3 líneas, formato
  ICAO 9303) con dígitos verificadores — fuente de datos mucho más confiable
  que el frente.

## 2. Objetivos

1. Detectar automáticamente el tipo de documento antes de extraer campos.
2. Extraer los campos correctos según reglas específicas por tipo (no una
   regex única "adivinando").
3. Leer y validar el MRZ del reverso de la cédula de residencia como fuente
   primaria de datos (con checksum).
4. Evitar lecturas prematuras/erróneas sin sacrificar la velocidad actual del
   OCR.
5. Dar feedback visual e in-cámara claro sobre el estado del escaneo, sin
   sacar al usuario del flujo de cámara.
6. Acotar el área de análisis al recuadro guía en pantalla (mejora velocidad
   y precisión).
7. Dejar la arquitectura abierta para agregar placas vehiculares a futuro.

## 3. Clasificación de tipo de documento

Paso previo a cualquier extracción de campos: analizar el texto crudo de ML
Kit contra palabras clave, antes de aplicar regexes de campos.

| Tipo | Señales clave en el texto OCR |
|---|---|
| `CEDULA_NACIONAL` | `TRIBUNAL SUPREMO DE ELECCIONES`, o patrón nacional `D-DDDD-DDDD` sin otras señales |
| `CEDULA_RESIDENCIA` (DIMEX) | `DGME`, `DIRECCIÓN GENERAL DE MIGRACIÓN`, `RESIDENTE PERMANENTE`/`RESIDENTE TEMPORAL` |
| `LICENCIA_NACIONAL` | `Licencia de Conducir` sin prefijo `DM-` en el número |
| `LICENCIA_EXTRANJERO` | `Licencia de Conducir` + número con prefijo `DM-` |
| `PLACA_VEHICULAR` (futuro) | Formato de placa MOPT, contexto distinto a licencia |

Modelo propuesto (sealed class/enum en Kotlin):

```kotlin
enum class TipoDocumento {
    CEDULA_NACIONAL,
    CEDULA_RESIDENCIA,
    LICENCIA_NACIONAL,
    LICENCIA_EXTRANJERO,
    PLACA_VEHICULAR, // futuro
    DESCONOCIDO
}
```

Cada tipo tiene su propio extractor de campos (no una regex compartida que
intenta cubrir todos los casos).

## 4. Extracción por tipo

- **DIMEX (frente):** extraer explícitamente el valor asociado a la etiqueta
  `Documento No.:`, descartando `Expediente No.:` (número de trámite interno,
  no identificación).
- **Licencia:** extraer el valor tras `Nº:`; si trae prefijo `DM-`, remover el
  prefijo para el número puro y marcar `esExtranjero = true` en el modelo.
- **Validación cruzada opcional:** si se escanea cédula y licencia de la misma
  persona, comparar que el número de documento coincida (chequeo de
  integridad, no obligatorio para aceptar el escaneo).

## 5. MRZ del reverso (cédula de residencia)

El reverso trae una zona de lectura mecánica estándar ICAO 9303 (formato
TD1, 3 líneas de 30 caracteres):

```
C<CRI1558243956105<<<<<<<<<<<<<<<<<<<<
8905307M2607285NIC<<<<<<<<<<<2
QUINTANA<MEDINA<<DANIEL<DE<JES
```

Contiene: tipo de documento, país emisor, número de documento, fecha de
nacimiento, sexo, fecha de vencimiento, nacionalidad y nombre — **cada campo
numérico con su propio dígito verificador**.

Propuesta:
- Parser de MRZ dedicado, ejecutado cuando se detecta el reverso (o cuando el
  usuario voltea el documento en el flujo de escaneo).
- Usar el MRZ como **fuente primaria** de datos para DIMEX; el frente queda
  como respaldo/validación cruzada.
- El checksum permite aceptar una lectura como válida en un solo frame sin
  necesidad de esperar múltiples lecturas consistentes (ver sección 6).

## 6. Estabilidad de lectura (evitar OCR "prematuro")

Problema: el OCR es tan rápido que puede devolver un resultado a medio
acomodar el documento.

Regla propuesta:
- Si el resultado del frame actual **pasa un checksum** (MRZ) → aceptar de
  inmediato, sin esperar frames adicionales.
- Si no hay checksum disponible (ej. frente de cédula nacional) → exigir que
  el mismo resultado se repita en 2-3 frames consecutivos (~70-150ms a
  15-30 fps de análisis) antes de aceptarlo.
- Este es el único punto del plan que agrega latencia deliberada, y es
  mínima/imperceptible en la práctica.

**Regla de performance para todo el plan:** ningún paso nuevo puede ser
`O(imagen)` (correr OCR de nuevo, agregar otro modelo pesado). Todo lo demás
(clasificación por keywords, parseo MRZ, checksum) es `O(texto)`, procesa el
resultado que ML Kit ya entregó, y corre en microsegundos — no debería
notarse ninguna diferencia de velocidad respecto al comportamiento actual.

## 7. Viewfinder interactivo (esquineros con 3 estados)

Reemplazar el rectángulo fijo actual por 4 esquineros tipo "L", con 3 estados
visuales controlados por un único estado central de confianza:

| Estado | Color | Condición |
|---|---|---|
| Buscando / sin enfocar | Gris (neutro) | Sin texto detectable, o documento fuera de rango |
| Detectado pero inválido | Rojo | Hay texto/documento en el recuadro pero no calza ningún tipo conocido, o falla el checksum |
| Correcto | Verde | Match completo de un tipo válido + checksum OK (o N frames consistentes) |

```kotlin
enum class EstadoEscaneo { BUSCANDO, INVALIDO, CONFIRMADO }
```

Este mismo estado alimenta:
- El color/animación de los esquineros (se contraen al pasar a `CONFIRMADO`).
- El mensaje de texto in-cámara (sección 8).
- La señal de éxito (vibración corta + check verde) al confirmar.

## 8. Mensajes in-cámara (sin salir del flujo)

Patrón estándar de la industria (ID-scanning SDKs tipo Scanbot, Dynamsoft):
**feedback siempre dentro de la cámara**, nunca un diálogo/pantalla de error
que interrumpa el flujo. Mensajes específicos y accionables, no genéricos:

| Estado | Mensaje sugerido |
|---|---|
| `BUSCANDO` sin texto | "Acercá el documento" |
| `BUSCANDO` con poca luz/blur | "Buscá mejor luz" / "Mantené firme" |
| `INVALIDO` (texto detectado, ningún tipo calza) | "Documento no reconocido" |
| `INVALIDO` (tipo detectado pero campo incompleto/glare) | "Reducí el reflejo" |
| `CONFIRMADO` | Check verde + vibración, transición automática |

## 9. Acotar el área de escaneo al recuadro guía

Confirmado que es viable y recomendable:
- CameraX permite definir un `cropRect` en el `ImageAnalysis.Analyzer` para
  que solo la región del recuadro guía se mande a ML Kit.
- Beneficios: menos píxeles a procesar (más rápido, no más lento), y ML Kit
  no se distrae con texto de fondo fuera de la tarjeta.
- El recuadro guía deja de ser solo decorativo: define el límite real de
  análisis, coherente con los esquineros de la sección 7.

## 10. Fuera de alcance por ahora (futuro)

- Reconocimiento de placas vehiculares (mencionado como caso de uso futuro).
  El enum `TipoDocumento` y el pipeline de clasificación quedan abiertos para
  agregarlo sin tocar la lógica de cédulas/licencias.

## 11. Orden sugerido de implementación (a confirmar)

1. Enum `TipoDocumento` + clasificador por keywords.
2. Extractores de campo específicos por tipo (frente).
3. Parser de MRZ + checksum (reverso DIMEX).
4. Estado central `EstadoEscaneo` + lógica de estabilidad/debounce.
5. Viewfinder con esquineros de 3 estados + recorte del área de análisis.
6. Mensajes in-cámara conectados al estado central.

## 12. Preguntas abiertas / pendientes de refinar

- ¿El flujo actual permite escanear frente y reverso en la misma sesión, o
  hay que agregar un paso de "dale vuelta al documento"?
- ¿Qué hacer si el checksum del MRZ falla pero el frente sí calza (documento
  dañado/mal iluminado)? ¿Aceptar con advertencia o rechazar?
- Confirmar fps de análisis actual de CameraX para calibrar el número de
  frames consistentes necesarios en el debounce.
