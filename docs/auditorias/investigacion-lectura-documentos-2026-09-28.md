# Investigación: cómo leer documentos y placas mejor (2026-09-28)

Complementa `auditoria-ocr-lectores-2026-09-28.md`. Aquella revisó el
código existente; ésta parte del problema (identificar rápido y bien a una
persona o un vehículo en el puesto de control) y de lo que los documentos
reales, la normativa y la literatura técnica permiten hoy. Las fuentes van
al final; lo que no se pudo confirmar está marcado como tal.

## 1. Qué trae de verdad cada documento

| Documento | Datos legibles por máquina | Confiabilidad | Estado en la app |
|---|---|---|---|
| **Cédula nueva** (policarbonato, emitida desde el 6-oct-2025) | **MRZ TD1** en el reverso; el TSE la puso *en lugar* del código de barras. Sin chip. Un análisis de Regula menciona un QR de verificación, **no confirmado** por el TSE. | Alta: dígitos verificadores ICAO. | MRZ ya se lee (Rust). Muestra real: `IDCRI1000002190<C004780077<<<<` -> el número de documento ES la cédula y trae además el número de tarjeta. |
| **Cédula anterior** (azul / serie 2019) | Sólo texto impreso + **PDF417** en el reverso. Sigue vigente hasta su vencimiento: conviven varios años. | Texto: baja (sin verificador). PDF417: ver §3. | Se lee por texto con regex. |
| **TIM** (12-17 años, 2025) | MRZ. | Alta. | Se reclasifica por edad. |
| **DIMEX** (formato 2023+) | MRZ `C<CRI` (confirmado contra un documento real en el repo) y **QR** con información cifrada "no sensible", verificable en el sitio de la DGME. Existe además el DIMEX digital en PDF. | Alta (MRZ). El QR agrega autenticidad, pero en línea. | MRZ sí; QR no. |
| **Pasaporte CR** | MRZ TD3. Serie 2022: **chip** con MRZ, foto y huellas. | Alta; el chip es la única fuente que prueba autenticidad. | MRZ sí; chip no. |
| **Licencia de conducir** (serie 2009) | Texto; código de barras de contenido no documentado. Fondo verde que perjudica el OCR y diseño con márgenes variables. La licencia digital con QR está anunciada para el 2.º semestre de 2026 (hubo varios retrasos). | Baja. | Se lee por texto. |
| **Gafetes propios** (CRC, In House) | Sólo texto impreso. **Los imprime la empresa**: es el único documento cuyo formato se puede elegir. | Baja hoy. | Se lee por texto y **dispara una salida sin confirmación**. |
| **Placas** | Texto. Formatos oficiales: particular 3 letras + 3 dígitos (o sólo dígitos en las antiguas), `CL` carga liviana, `C` carga pesada, `MOT` + 3 dígitos + 3 letras (formato nuevo de moto), más prefijos de bus, taxi, remolque, etc. | Baja (sin verificador). | Reglas armadas con muestras sueltas. |

**Conclusión:** para la mayoría de las personas que entran hoy, y cada vez
más, **el documento ya trae una fuente con verificador (MRZ)**. La app
trata el texto libre como fuente principal y la MRZ como un caso más. Hay
que invertir esa jerarquía.

## 2. Lo que dice la evidencia técnica

- **ML Kit es la elección correcta**, no Tesseract. En el estudio
  MIDV-2019 (tabla 3), Tesseract 4.1 sin entrenamiento específico para la
  fuente OCR-B acertó líneas MRZ completas sólo entre ~0,3 % y 8,9 % de las
  veces, según la condición de captura.
- **ML Kit ya entrega más de lo que se usa**. Cada `Text.Line` y
  `Text.Element` trae `boundingBox`, `cornerPoints`, `angle` y `confidence`
  (0-1). Con el modelo empaquetado, la confianza siempre está disponible.
  Google recomienda caracteres de al menos 16 px (sin ganancia por encima
  de 24 px) y dejar caer frames mientras el reconocedor está ocupado (ya se
  hace).
- **Combinar varios frames supera a exigir la misma cadena repetida.** En
  MIDV-500, alinear los resultados de cada frame (método ROVER: alineación
  por distancia de Levenshtein y votación por carácter) y ponderarlos por
  **nitidez de la imagen** dio el menor error. La mejor estrategia fue
  combinar el 50 % de frames más nítidos. Error medio en números de
  documento: 0,0605 sin ponderar contra 0,0431 ponderando por nitidez (−29 %).
  Elegir sólo "el mejor frame" fue la peor estrategia. La nitidez se estima
  barato: cuantil 0,95 de los gradientes de la imagen.
- **También se estudió cuándo dejar de capturar** ("stopping rules"):
  cortar apenas un frame más ya no cambiaría el resultado combinado. Es
  exactamente la decisión que hoy toma el estabilizador con contadores
  fijos.
- **Detectar un documento mostrado en otra pantalla o fotocopiado** es un
  campo de investigación activo (moiré, bordes, redes neuronales). No hay
  una solución ligera y confiable para correr en el teléfono. No conviene
  prometerlo; la defensa práctica es de procedimiento (ver §4).

## 3. Decisiones de privacidad y legales

- **Actualización (2026-09-28): decisión del negocio.** El sitio es zona
  franca y quien ingresa acepta un contrato de condiciones de uso de datos;
  se decidió leer el PDF417 tomando SÓLO cédula y nombre. Implementado con
  minimización por diseño: ver `mobile/android/docs/arquitectura-ocr.md`
  ("PDF417 de la cédula anterior"). El análisis original, que sigue
  valiendo como fundamento de esa minimización:
- **PDF417 de la cédula anterior: NO decodificarlo.** Existe una clave XOR
  publicada por ingeniería inversa y proyectos que la usan, pero ese código
  **incluye huellas dactilares**. La PRODHAB, en la resolución 029-2026-RF,
  reafirmó que los biométricos son datos sensibles (Ley 8968) y que no
  pueden reutilizarse fuera de la finalidad para la que se recolectaron.
  Los fabricantes de lectores advierten además que descifrarlo requiere
  autorización. Leerlo expondría a la empresa sin necesidad: el número
  se obtiene del frente.
- **VID del TSE** (cotejo de huella en línea, vía RACSA): es un servicio
  pago, requiere lector biométrico y conexión, y es sólo para
  costarricenses mayores de edad. Desproporcionado para una portería; no
  se recomienda.
- **Minimizar lo que se guarda.** La app ya no guarda imágenes y sólo
  loguea el texto en debug. Mantenerlo así. No persistir el número de
  tarjeta (`C004780077`) ni la fecha de nacimiento si el negocio no los
  usa.

## 4. Seguridad: qué protege cada medida

Leer un documento **identifica el número impreso, no autentica a la
persona**. La MRZ tampoco prueba autenticidad: cualquiera puede calcular
dígitos verificadores válidos. Por eso:

1. **Gafetes con código de barras o QR** (los imprime la empresa). Es la
   mejora de mayor impacto. ML Kit Barcode Scanning (modelo empaquetado,
   ~2,4 MB; soporta QR, Code 128, PDF417 y DataMatrix; tiene zoom
   automático desde la 17.2.0) lee en un frame, sin confusiones 3/8, y el
   contenido puede llevar dígito verificador. Elimina la clase de error que
   hoy puede registrar la salida de otra persona. Un QR copiable no
   impide que alguien fotocopie un gafete, pero eso tampoco lo impide el
   número impreso; el control del gafete físico sigue igual.
2. **Mientras no se reimpriman**: pedir un toque de confirmación en la
   salida por gafete. Cuesta un segundo y cierra el riesgo.
3. **Coherencia entre fuentes**: si en la misma sesión se leen frente y
   MRZ, el número debe coincidir; si no, rechazar.
4. **Procedimiento**: quien opera compara la foto del documento con la
   persona. La app puede recordarlo en la confirmación, pero no
   reemplazarlo.
5. **Chip del pasaporte (NFC)**: la única verificación criptográfica
   disponible. El A25 tiene NFC y existe una librería abierta (JMRTD), pero
   la validación completa exige certificados de cada país. Sólo vale la
   pena si los pasaportes son frecuentes en el sitio.

## 5. Diseño propuesto

### Tubería por frame

```
CameraX (YUV) -> recorte al recuadro -> CALIDAD (nitidez, reflejo) --mala--> descartar + guiar
                                          | buena
                                          v
                              ML Kit texto  (+ ML Kit códigos en modo gafete)
                                          v
                Kotlin: normalizar a "LecturaFrame"
                (renglones reconstruidos por geometría, confianza por carácter, calidad)
                                          v  (1 cruce FFI por frame)
                Rust: LectorDocumentos (objeto con estado, UniFFI)
                  1. clasificar           4. validar (checksum MRZ, formato cédula/placa)
                  2. extraer por campo    5. decidir si ya se puede parar
                  3. votar por carácter entre frames, ponderando por calidad
                                          v
                              Kotlin: UI, vibración, sonido
```

### Qué cambia respecto de hoy y por qué

| Hoy | Propuesto | Beneficio |
|---|---|---|
| Se procesa todo frame que llega (limitado a ~6/s) | Se mide nitidez y reflejo sobre el plano Y ya recortado (barato) y los frames malos **no pasan por ML Kit** | Menos CPU mientras la persona acomoda el documento. Además los mensajes "hay reflejo" o "está borroso" pasan a ser mediciones, no suposiciones. |
| Texto plano de ML Kit, en el orden que llegue | Renglones reconstruidos por posición, con confianza | Desaparecen los parches por orden de bloques (DIMEX, etiquetas de la cédula, MRZ partida). |
| Misma cadena completa 2-3 veces | Votación por carácter ponderada por nitidez + validación | Lee antes con reflejos intermitentes y acepta menos errores (ver §2). |
| Texto primero, MRZ como un caso más | Jerarquía: código de barras > MRZ > texto | Usa la fuente con verificador cuando existe. |
| Recuadro de cédula para placas | Recuadro de placa (~2:1), zoom y **linterna** | Placas lejanas y de noche. |
| Reglas de placa armadas con muestras sueltas | Gramática con los prefijos oficiales del Registro Nacional | Menos falsos positivos y menos correcciones a mano. |
| Lógica de lectura repartida entre Kotlin y Rust | Contrato `LecturaFrame` -> Rust; Kotlin sólo cámara, ML Kit y UI | Una sola implementación para Android, iOS y escritorio, y pruebas en `cargo test`. |

### Por qué el contrato es la clave

`LecturaFrame` es un dato serializable (renglones, cajas, confianzas,
calidad). Guardarlo en builds de depuración permite **reproducir sesiones
reales en tests de Rust** sin imágenes ni teléfono. Eso resuelve a la vez
la migración a Rust y la falta de medición.

## 6. Medición antes de optimizar

1. En debug, grabar la secuencia de `LecturaFrame` de cada escaneo en el
   almacenamiento privado de la app. Ninguna imagen.
2. Escanear en el A25 un conjunto real: cada tipo de documento, de día y de
   noche, con y sin reflejo, más las placas. Las sesiones contienen datos
   personales: sólo se versionan anonimizadas o con documentos propios o de
   prueba.
3. Métricas de referencia:
   - % de documentos leídos en ≤ 2 s;
   - tiempo mediano hasta confirmar;
   - **lecturas equivocadas aceptadas** (objetivo: 0).
4. Cada cambio se evalúa contra esas cifras, no contra una muestra suelta.

## 7. Comprar en vez de construir

Existen SDK comerciales que ya reconocen documentos costarricenses y
resuelven detección, enderezado, reflejos y MRZ (Regula, Microblink,
ID Analyzer, entre otros). Costo: licencia por dispositivo o por volumen,
código cerrado y dependencia de un proveedor, y no cubren gafetes propios
ni placas. Con MRZ en la mayoría de los documentos nuevos y los gafetes
bajo control propio, **construir sobre ML Kit es razonable**; conviene
revisarlo si aparecen muchos documentos extranjeros o si se necesita
verificar autenticidad.

## 8. Orden recomendado

1. **Gafetes**: confirmación en la salida (inmediato) y decidir si se
   reimprimen con QR o Code 128.
2. **Medición**: grabación de `LecturaFrame` en debug + sesiones reales.
3. **Calidad por frame** (nitidez y reflejo) y **linterna/zoom**.
4. **Lector en Rust con votación por carácter**, empezando por MRZ y
   número de cédula; luego el resto de los campos y las placas.
5. Evaluar QR de DIMEX (verificación en línea) y NFC de pasaportes según
   la frecuencia real en el sitio.

## Pendiente de confirmar con documentos físicos

- ¿La cédula nueva trae QR? (Regula lo menciona; el TSE no.)
- ¿Qué contiene el QR del DIMEX: una URL de la DGME o datos cifrados?
  Basta con apuntarle la cámara del teléfono y ver qué tipo de contenido
  muestra, sin compartir los datos.
- Distancia mínima de enfoque real del A25 con cédula y con placa.

## Fuentes

- TSE, comunicado sobre el nuevo diseño de la cédula: https://www.tse.go.cr/comunicado1081.html
- Tico Times, nueva cédula: https://ticotimes.net/2025/10/06/costa-rica-updates-national-id-card-with-enhanced-security-featuresy
- Biometric Update, nueva cédula: https://www.biometricupdate.com/202510/costa-rica-unveils-new-identity-card-design-with-enhanced-security-features
- Regula, documentos de Costa Rica: https://regulaforensics.com/blog/costa-rica-id-card-processing/
- TSE, Identidad Digital Costarricense: https://www.tse.go.cr/idc/
- PDF417 de la cédula: https://es.linkedin.com/pulse/lector-de-pdf417-la-c%C3%A9dula-costa-rica-ron-du y https://github.com/carlos-jenkins/LectorCedulasCR
- IAPP sobre la resolución PRODHAB 029-2026-RF: https://iapp.org/news/a/datos-biom-tricos-y-autodeterminaci-n-informativa-en-la-resoluci-n-n-029-2026-rf-de-la-prodhab
- TSE, verificación de identidad (VID): https://www.tse.go.cr/verificacion_identidad.html
- BDS Asesores, nuevo DIMEX: https://publicaciones.bdsasesores.com/en-us/blog/costarica-bds_immigrationalert-new-format-of-immigration-identity-document-for-foreign-citizens-dimex
- Licencia digital: https://observador.cr/bcr-no-ofrecera-servicio-de-licencia-digital-anuncia-jerarca-del-mopt-entidad-dice-no-haber-sido-notificada-de-exclusion/
- Clases de placas: https://registronacional.com/costarica/vehiculos_placa_clases.htm
- ML Kit, reconocimiento de texto v2: https://developers.google.com/ml-kit/vision/text-recognition/v2/android
- ML Kit, `Text.Line`: https://developers.google.com/android/reference/com/google/mlkit/vision/text/Text.Line
- ML Kit, lectura de códigos: https://developers.google.com/ml-kit/vision/barcode-scanning/android
- Combinación ponderada de frames (MIDV-500): https://arxiv.org/abs/1911.12028
- Reglas de parada en video: https://arxiv.org/abs/2008.02566
- MIDV-2019 (Tesseract en MRZ): https://arxiv.org/pdf/1910.04009
- Detección de documentos en pantalla o impresos: https://ietresearch.onlinelibrary.wiley.com/doi/full/10.1049/bme2.12088
