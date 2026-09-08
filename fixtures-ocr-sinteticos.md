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
