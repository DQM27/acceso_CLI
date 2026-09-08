# Plan: Motor general de lectura de documentos de identidad (Android)

> Estado: **planificación** — sin cambios de código todavía. Este documento se irá
> refinando en conversación antes de implementar. Última actualización: 2026-09-08.

## 0. Cambio de alcance: de "lector de cédulas CR" a motor general

A partir de esta revisión, el proyecto deja de plantearse como un lector de
cédulas costarricenses con extensiones puntuales, y pasa a diseñarse como un
**motor general de documentos de identidad**, donde Costa Rica es un conjunto
de reglas específicas (un "paquete de país") dentro del motor, no el centro
del diseño. Motivo: agregar soporte de pasaportes obliga a que el núcleo ya
sea agnóstico de país desde el modelo de datos, no como parche después.

### 0.1 Por qué el MRZ es la columna vertebral, no el layout visual

El estándar **ICAO Doc 9303** (OACI) define varias familias de zona de
lectura mecánica, cada una con estructura fija sin importar el país emisor:

| Formato | Estructura | Uso típico |
|---|---|---|
| TD1 | 3 líneas × 30 caracteres | cédulas, tarjetas de residencia (DIMEX, cédula CR 2025+) |
| TD2 | 2 líneas × 36 caracteres | documentos de viaje/tarjetas grandes |
| TD3 | 2 líneas × 44 caracteres | pasaportes |
| MRV-A | 2 líneas × 44 caracteres | visas |
| MRV-B | 2 líneas × 36 caracteres | visas |

El layout visual (frente del documento) cambia por país y por versión. El
MRZ no — es el mismo algoritmo de checksum (módulo 10, pesos `7,3,1`
repetidos, `A-Z`=10-35, `<`=0) para cualquier país que siga el estándar. Esto
ya lo aplicamos en `fixtures-ocr-sinteticos.md` para TD1; el mismo mecanismo
aplica sin cambios a TD2/TD3/MRV — solo cambia el layout de campos dentro de
la línea, no la matemática de validación.

**Implicación práctica:** el parser de MRZ debe diseñarse como un módulo
único parametrizado por formato (TD1/TD2/TD3/MRV-A/MRV-B), no como un parser
"de DIMEX" que después se duplica para pasaporte.

### 0.2 Arquitectura de tres lectores en paralelo

```
CÁMARA
   ├── Barcode Scanner (ML Kit Barcode Scanning — on-device, ya soporta esto)
   │      ├─ PDF417
   │      ├─ QR
   │      └─ DataMatrix / Aztec
   │
   ├── MRZ Detector (módulo propio, sobre el texto de ML Kit OCR)
   │      ├─ TD1 / TD2 / TD3 / MRV-A / MRV-B
   │
   └── ML Kit OCR (ya en uso)
          └─ texto visual del frente / fallback cuando no hay MRZ ni barcode
```

Los tres desembocan en **un único modelo de identidad normalizado**,
independiente de por cuál vía llegó el dato:

```
tipo_documento
pais_emisor
numero_documento
nombre
apellidos
nacionalidad
fecha_nacimiento
sexo
fecha_vencimiento
fecha_emision
fuente_datos        (mrz | barcode | ocr_frente)
checksum_valido
ocr_confidence
texto_crudo
```

### 0.3 Caso Costa Rica: dos generaciones de cédula, dos estrategias

Verificado (comunicado oficial TSE, octubre 2025): la cédula costarricense
tiene un quiebre de formato, no una evolución continua.

- **Cédula anterior a oct-2025:** reverso con código de barras **PDF417**.
  Confirmado que el contenido de ese PDF417 **viene cifrado** — un lector
  puede decodificar el barcode sin problema (ML Kit lo soporta nativamente),
  pero el texto resultante no es utilizable sin la clave/licencia de
  descifrado del TSE. **No asumir que este camino funciona sin antes
  confirmar acceso legítimo al esquema de descifrado** (existe al menos una
  tesis del TEC que investigó el formato — revisar antes de invertir tiempo
  de implementación aquí). Mientras no se resuelva, la fuente primaria para
  cédulas viejas sigue siendo el **frente vía OCR**, como ya está planificado
  en las secciones 3-4 de este documento.
- **Cédula desde oct-2025:** reverso con **MRZ TD1** (reemplaza el PDF417
  directamente, sin barcode). Esta es la vía "fácil" — mismo mecanismo que ya
  diseñamos para DIMEX.

Ambas generaciones coexisten en circulación (el TSE no obliga a renovar
mientras la cédula esté vigente), así que el motor debe reconocer ambas: si
hay MRZ, usarlo como fuente primaria; si no, caer a OCR de frente + (a
futuro, si se resuelve el descifrado) PDF417.

### 0.4 Pasaportes (TD3) — la ganancia real de este pivote

Un TD3 típico (ejemplo ICAO, datos ficticios):
```
P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<
L898902C36UTO7408122F3404159ZE184226B<<<<<16
```
Con el parser de MRZ ya generalizado (0.1), leer un pasaporte de cualquier
país que siga ICAO 9303 no requiere un parser nuevo por país — solo el
layout TD3 en vez de TD1. Ya se agregó un ejemplo TD3 sintético en
`fixtures-ocr-sinteticos.md` (sección 8); falta ampliarlo con más variantes
y casos corruptos, igual que se hizo para TD1.

### 0.5 Estrategia de datasets (más allá de los fixtures propios)

Se evaluaron fuentes externas de datos/documentos para robustecer las
pruebas más allá de los fixtures sintéticos propios (`fixtures-ocr-sinteticos.md`):

| Fuente | Qué es | Uso recomendado en este proyecto |
|---|---|---|
| PRADO (Consejo UE) | Catálogo de referencia de documentos reales por país (fotos + ficha técnica), no dataset de entrenamiento | Consultar para saber qué versiones/layouts existen por país (incluye Costa Rica); **no** redistribuir sus imágenes sin revisar condiciones de reuso |
| Casos de prueba MRZ de librerías ICAO 9303 | MRZ sintéticos válidos e inválidos a propósito | Directamente reutilizables — mismo enfoque que ya usamos en fixtures propios |
| MIDV-500 / MIDV-2020 / MIDV-Holo | Datasets académicos de video/foto de documentos ficticios (incluye IDs, licencias, pasaportes), pensados para captura desde celular | Útiles como prueba de robustez de captura (blur, reflejo, perspectiva) en una fase posterior — **revisar licencia de uso antes de integrarlos**, no asumir uso libre |
| DocXPand-25k | ~25k documentos sintéticos generados, incluye plantillas de pasaporte TD3 | Interesante porque es generador, no solo imágenes — permite producir casos con errores inyectados y verdad de referencia conocida; evaluar en fase de robustecimiento, no ahora |
| IDNet | ~598k imágenes sintéticas, ~400GB | Demasiado grande para el estado actual del proyecto; descartar por ahora, revisar solo si se llega a necesitar entrenamiento propio de modelo (no es el plan — seguimos usando ML Kit) |
| SIDTD | Documentos originales/manipulados, orientado a antifraude | Fuera de alcance actual (este proyecto no hace detección de fraude documental) |

**Decisión para esta etapa:** seguir con fixtures sintéticos propios
(`fixtures-ocr-sinteticos.md`) como base de pruebas unitarias — son gratis,
controlables, sin problema de licencia y ya cubren los casos límite
identificados. Los datasets externos (MIDV, DocXPand) se evalúan como
recurso de **robustecimiento en una fase posterior**, una vez el motor base
funcione, y solo tras confirmar sus términos de licencia.

### 0.6 Cámara e iluminación

Evaluado explícitamente el uso de flash/linterna como ayuda de iluminación:
**descartado**. Las cédulas/licencias son laminadas y semi-brillantes; a la
distancia típica de escaneo (10-15cm) el flash genera un punto de reflejo
(glare) que frecuentemente cae encima del número o del MRZ — empeora la
lectura en vez de mejorarla. No se implementa control de flash/torch.

En su lugar, mejoras de cámara con impacto real para este caso de uso:

1. **Enfoque continuo optimizado a distancia cercana.** El autofocus por
   defecto de muchos dispositivos no está calibrado para 10-15cm. Configurar
   el modo de autofocus de CameraX/Camera2 para rango cercano (o usar cámara
   macro si el dispositivo la expone) da más ganancia real que cualquier
   ajuste de iluminación.
2. **Detección de glare como parte del score de confianza** (mismo estado
   central `EstadoEscaneo` de la sección 7): si una zona sobresaturada se
   superpone al área de texto esperada, disparar el mensaje "Reducí el
   reflejo" ya definido en la sección 8 — ataca la causa real (ángulo/posición
   del documento), no la compensa con más luz.
3. **Exposición automática con posible compensación leve** en ambientes muy
   oscuros — más seguro que flash porque ajusta brillo global sin crear
   puntos calientes.
4. **Estabilización (OIS/EIS)** si el dispositivo la expone — ayuda contra
   motion blur (mano temblando), no contra falta de luz, pero es gratis de
   activar vía CameraX/Camera2 si está disponible.

Prioridad de implementación: (1) enfoque cercano y (2) detección de glare
primero, por atacar la causa real; (3) y (4) como mejoras secundarias.

### 0.7 Postura sobre ML Kit vs OCR propio

No se plantea reemplazar ML Kit Text Recognition por un modelo propio. ML
Kit ya resuelve detección + reconocimiento en tiempo real con bloques,
líneas, elementos y bounding boxes (ver sección de disección de OCR más
abajo). El esfuerzo del proyecto se concentra en lo que ML Kit **no** hace
por sí solo: clasificar tipo de documento, reconstruir y validar MRZ,
corregir mediante checksum, extraer campos, validar vigencia, leer
PDF417/QR vía Barcode Scanning, y normalizar todo a un modelo único.

---

## 1. Contexto y problema actual (cédula nacional CR — caso original)

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
7. ~~Dejar la arquitectura abierta para agregar placas vehiculares a futuro.~~
   Descartado: placas vehiculares no son parte de esta app — quedan
   documentadas por separado en `idea-lector-placas-vehiculares.md` para una
   aplicación distinta.

## 3. Clasificación de tipo de documento

Paso previo a cualquier extracción de campos: analizar el texto crudo de ML
Kit contra palabras clave, antes de aplicar regexes de campos.

| Tipo | Señales clave en el texto OCR |
|---|---|
| `CEDULA_NACIONAL` | `TRIBUNAL SUPREMO DE ELECCIONES`, o patrón nacional `D-DDDD-DDDD` sin otras señales |
| `CEDULA_RESIDENCIA` (DIMEX) | `DGME`, `DIRECCIÓN GENERAL DE MIGRACIÓN`, `RESIDENTE PERMANENTE`/`RESIDENTE TEMPORAL` |
| `LICENCIA_NACIONAL` | `Licencia de Conducir` sin prefijo `DM-` en el número |
| `LICENCIA_EXTRANJERO` | `Licencia de Conducir` + número con prefijo `DM-` |
Modelo propuesto (sealed class/enum en Kotlin):

```kotlin
enum class TipoDocumento {
    CEDULA_NACIONAL,
    CEDULA_RESIDENCIA,
    LICENCIA_NACIONAL,
    LICENCIA_EXTRANJERO,
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

- Reconocimiento de placas vehiculares: **no es parte de esta app**. Análisis
  completo hecho y documentado en `idea-lector-placas-vehiculares.md`, para
  implementar en la aplicación que corresponda, no acá.

## 11. Orden sugerido de implementación (a confirmar)

1. ✅ Enum `TipoDocumento` + clasificador por keywords.
2. ✅ Extractores de campo específicos por tipo (frente).
3. ✅ Parser de MRZ + checksum (TD1 y TD3), incluyendo el mecanismo estándar
   ICAO de número extendido (>9 caracteres) y la convención costarricense
   del DIMEX (perfil `CR_DIMEX_TD1_2023`, resuelto y verificado contra un
   documento real recalculando los 4 checksums independientemente) — ver
   `fixtures-ocr-sinteticos.md` secciones 4.1 y 4.2. Sin especificación
   pública oficial del TSE/DGME que lo confirme por escrito, pero la
   evidencia matemática es sólida.
4. ✅ Estado central `EstadoEscaneo` + lógica de estabilidad/debounce.
5. ✅ Viewfinder con esquineros de 3 estados. Recorte del área de análisis
   implementado **filtrando los `TextBlock` de ML Kit por su `boundingBox`**
   contra el recuadro guía (`filtrarTextoEnAreaGuia`), no convirtiendo el
   frame a Bitmap para recortar píxeles -- esa alternativa habría agregado
   una conversión YUV→RGB completa por frame, violando la regla de la
   sección 0.6/5 (nada nuevo debe ser `O(imagen)`). Enfoque continuo en el
   centro implementado vía `FocusMeteringAction` de CameraX (costo único al
   iniciar la cámara, no por frame). **Pendiente:** detección de glare (sí
   requeriría analizar píxeles, evaluar por separado si vale el costo).
6. ✅ Mensajes in-cámara conectados al estado central, incluyendo el nombre
   del tipo de documento detectado (nadie lo selecciona a mano).
7. ✅ Cédula nacional 2025+ vs DIMEX distinguidas **solo por el MRZ**
   (código de documento en posiciones 1-2: `ID` = cédula nacional, `C<` =
   DIMEX) — ver `fixtures-ocr-sinteticos.md` sección 4.3. La regla vive en
   `ResultadoMrz.aDocumentoDetectado()` (país + código), no en el parser
   MRZ genérico, para no llenarlo de excepciones por país cuando se agreguen
   otros. Vencimiento in-camara ahora se anuncia correctamente también para
   el reverso de ambos documentos, ya no solo para el frente.
8. ✅ Cédula de adulto vs Tarjeta de Identidad de Menores (TIM) — ambas usan
   `IDCRI...`, pero el MRZ ya trae fecha de nacimiento: se calcula la edad
   contra la fecha actual y se reclasifica si es menor de 18
   (`DocumentoDetectado.reclasificarPorEdad()`) — ver
   `fixtures-ocr-sinteticos.md` sección 4.3.
9. ✅ Vibración corta al confirmar (sección 8, "check verde + vibración").
   `VibrationEffect.createOneShot()` (permiso `VIBRATE`, normal, sin diálogo
   de runtime), disparada una sola vez en el mismo punto donde ya se
   garantizaba una única llamada a `onCedulaDetectada` (`compareAndSet`).

## 12. Preguntas abiertas / pendientes de refinar

- ¿El flujo actual permite escanear frente y reverso en la misma sesión, o
  hay que agregar un paso de "dale vuelta al documento"?
- ¿Qué hacer si el checksum del MRZ falla pero el frente sí calza (documento
  dañado/mal iluminado)? ¿Aceptar con advertencia o rechazar?
- Confirmar fps de análisis actual de CameraX para calibrar el número de
  frames consistentes necesarios en el debounce.
