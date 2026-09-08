# Fixtures sintéticos para pruebas de OCR

> Anexo de `plan-ocr-escaneo-documentos.md`. Todos los datos (nombres, números,
> fechas) son **inventados** — ninguno corresponde a una persona real. Los MRZ
> están generados con el algoritmo real de checksum ICAO 9303 (pesos 7-3-1,
> letras A-Z=10-35, `<`=0), así que son estructuralmente válidos aunque los
> datos sean ficticios. Pensado para alimentar tests unitarios estilo
> `CedulaOcrTest.kt`, no para pegarse literal en producción.

## 1. Por qué texto crudo con ruido, no texto limpio

El valor de estos fixtures no es que representen una lectura perfecta, sino
que reproducen los errores típicos que ML Kit comete de verdad: confusión
O↔0, S↔5, I↔1↔l, acentos perdidos, saltos de línea partidos, espacios extra.
Un fixture "ideal" no prueba nada sobre robustez.

## 2. Cédula nacional (frente)

Ya cubierto en `CedulaOcrTest.kt` — se mantiene igual, solo referencia:

```
"CEDULA IDENTIDAD\n1-1234-0567\nCOSTA RICA"   -> 112340567
"Identificacion 1 1234 0567"                  -> 112340567
"CR 112340567"                                -> 112340567
"Documento sin numeros completos"             -> null
```

Casos nuevos a agregar (ruido OCR):
```
"CEDULA DE IDENTIDAD\n1-O234-0567\nCOSTA RlCA"
  -- O en vez de 0 dentro del número -> debe fallar o corregirse antes de aceptar
"TRIBUNAL SUPREMO DE ELECCIONES\nl-1234-0567"
  -- I mayúscula/l minúscula confundida con 1 al inicio
```

## 3. Cédula de residencia (DIMEX) — frente

Caso real que motivó el plan: dos números de longitud similar en el mismo
bloque de texto.

```
"DIRECCIÓN GENERAL DE MIGRACIÓN Y EXTRANJERÍA
REPÚBLICA DE COSTA RICA
RESIDENTE PERMANENTE
LIBRE CONDICIÓN
Apellidos:
PEREZ RAMIREZ
Nombre:
MARIA JOSE
Nacionalidad:
NICARAGUA
Documento No.: 999888777
Expediente No.: 135-453544
Vence: 28 07 2026"
```
Extracción esperada:
- `tipoDocumento = CEDULA_RESIDENCIA`
- `numeroDocumento = "999888777"` (NO `135453544`, el de expediente)
- `nombre = "MARIA JOSE"`, `apellidos = "PEREZ RAMIREZ"`
- `nacionalidad = "NICARAGUA"`
- `vencimiento = 28/07/2026`

Variante con ruido (línea partida, típico cuando el documento no está bien
plano bajo la cámara):
```
"...Documento No.:
999888777
Expediente No.: 135-
453544..."
```
Este caso es exactamente el que justifica usar `Line`/`TextBlock` en vez de
`resultado.text` plano (ver sección de disección de OCR): con texto plano
concatenado, "135-" y "453544" pueden terminar pegados al número de
documento por accidente de orden de líneas.

## 4. Cédula de residencia — MRZ (reverso, TD1)

**Válido:**
```
IDCRI9998887774<<<<<<<<<<<<<<<
9001011F3001019NIC<<<<<<<<<<<8
PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
```
Decodificado: doc `999888777` (check `4`), nacimiento `01/01/1990` (check
`1`), sexo `F`, vencimiento `01/01/2030` (check `9`), nacionalidad `NIC`,
composite check `8`.

**Corrupto a propósito** (un solo dígito alterado en el número de documento,
línea 1 — el resto queda igual):
```
IDCRI9998887784<<<<<<<<<<<<<<<
9001011F3001019NIC<<<<<<<<<<<8
PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
```
El check digit `4` ya no corresponde a `999888778` → el parser debe
**rechazar esta línea** (checksum inválido), no aceptarla silenciosamente.
Este es el caso de prueba central de la sección 5 del plan (estabilidad de
lectura vía checksum).

### 4.1 Número extendido (más de 9 caracteres) — mecanismo estándar ICAO

Verificado contra un caso real documentado (cédula belga,
[issue #4 de Arg0s1080/mrz](https://github.com/Arg0s1080/mrz/issues/4)) y
recalculado con el mismo algoritmo de checksum:

```
IDBEL123456789<1233<<<<<<<<<<<
9001011F3001019BEL<<<<<<<<<<<8
PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
```
Posición 15 = `<` (no un check digit) señala la extensión. La continuación
(`123`) y su check digit (`3`) siguen en el campo opcional. El check digit
de la extensión se calcula sobre `"123456789" + "<" + "123"`, no solo sobre
la continuación. Número completo resultante: `123456789123`.

**Corrupto a propósito** (check digit de la extensión alterado, `3`→`4`):
```
IDBEL123456789<1234<<<<<<<<<<<
9001011F3001019BEL<<<<<<<<<<<8
PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
```

### 4.2 Número extendido — convención costarricense del DIMEX (RESUELTO)

El DIMEX real trae un **dígito**, no `<`, en la posición 15, con más dígitos
del número en el campo opcional. Investigación externa (confirmada de forma
independiente recalculando los checksums con el mismo algoritmo) estableció
que **sí es un mecanismo real, no ruido**: la posición 15 es el check digit
normal ICAO del bloque de 9 dígitos (no el `<` que exige el mecanismo
"long document number" de ICAO), y los dígitos que faltan del DIMEX de
11-12 dígitos continúan en el campo opcional **sin check digit propio**.

Contra el DIMEX real analizado, las 4 validaciones ICAO calzan:
- Check digit del bloque base (`155824395` → `6`) ✅
- Check digit de nacimiento ✅
- Check digit de vencimiento ✅
- Check digit compuesto final (cubre todo el campo opcional tal cual viene
  impreso) ✅

Perfil implementado como `CR_DIMEX_TD1_2023` en `parsearMrzTd1`: se activa
cuando el país emisor es `CRI`, el check digit de la posición 15 valida
correctamente el bloque de 9 dígitos, y hay dígitos (no relleno) inmediatamente
después. Fixture sintético (datos inventados, mismo patrón verificado):
```
C<CRI1999888772701<<<<<<<<<<<<
9001011M3001019NIC<<<<<<<<<<<0
PEREZ<<MARIA<JOSE<<<<<<<<<<<<<
```
Reconstruye a `199988877701`.

**Lo que sigue sin verificar formalmente:** no se encontró una especificación
pública del TSE/DGME que documente este layout con ese nivel de detalle — la
evidencia es matemática (4 checksums calzando en un documento real), no una
fuente oficial escrita. Si aparece esa fuente, confirmar contra ella.
**Nota de robustez:** DIMEX puede tener 11 o 12 dígitos según normativa
citada (Hacienda) — la continuación se lee como todos los dígitos hasta el
primer `<`, no como una longitud fija de 3.

**Trampa a evitar:** el MRZ ilustrativo que DGME publica en sus circulares
oficiales (número ficticio `123456789012`) **no tiene checksums ICAO
válidos** — no sirve como fixture de test, solo como referencia visual del
layout.

## 5. Licencia de conducir — nacional

```
"REPUBLICA DE COSTA RICA
Licencia de Conducir
Nº: 112340567
Expedición 03-04-2023
Vencimiento 03-04-2026
Tipo: B1
PEREZ RAMIREZ MARIA JOSE"
```
Esperado: `tipoDocumento = LICENCIA_NACIONAL`, `esExtranjero = false`,
`numero = "112340567"`.

## 6. Licencia de conducir — extranjero

```
"REPUBLICA DE COSTA RICA
Licencia de Conducir
Nº: DM-999888777
Expedición 03-04-2023
Vencimiento 03-04-2026
Tipo: A3
PEREZ RAMIREZ MARIA JOSE"
```
Esperado: `tipoDocumento = LICENCIA_EXTRANJERO`, `esExtranjero = true`,
`numero = "999888777"` (prefijo `DM-` removido, pero registrado como señal
de tipo).

Ruido típico: `Nº: DM-9998887ll` (I/l confundida con 1), `N9: DM-999888777`
(º leído como 9).

## 7. Casos de vigencia (vigente/vencido)

```
fechaVencimiento = 28/07/2026, fechaHoy = 08/09/2026  -> VIGENTE
fechaVencimiento = 28/07/2024, fechaHoy = 08/09/2026  -> VENCIDO
fechaVencimiento = 08/09/2026, fechaHoy = 08/09/2026  -> VIGENTE (límite inclusivo, a confirmar regla de negocio)
```

## 8. Pasaporte — MRZ (TD3, referencia futura)

Formato distinto (2 líneas de 44 caracteres, no 3 de 30 como TD1). Se deja
documentado para cuando se evalúe soporte de pasaporte:

```
P<CRIPEREZ<<MARIA<JOSE<<<<<<<<<<<<<<<<<<<<<<
A1234567<6CRI9001011F3001019<<<<<<<<<<<<<<04
```

## 9. Uso sugerido

Estos fixtures alimentan tests unitarios (estilo `CedulaOcrTest.kt`) para:
1. El clasificador de `TipoDocumento` (sección 3 del plan).
2. Los extractores por tipo (sección 4).
3. El parser + validador de checksum de MRZ (sección 5).
4. La lógica de vigencia (necesaria para el flujo de proveedor).

No reemplazan pruebas manuales con documentos reales antes de producción,
pero cubren los casos límite de forma reproducible y sin datos de personas
reales.
