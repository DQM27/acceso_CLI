# Idea a futuro: lector de placas vehiculares costarricenses

> **No es parte de esta app** (`acceso_CLI` / control de acceso de
> contratistas). Este documento existe solo para no perder el análisis
> hecho — la implementación real va en otra aplicación. Estado:
> **idea/investigación**, sin código ni validación oficial completa.

## Contexto

Durante el refinamiento del lector de documentos de identidad de esta app
(ver `plan-ocr-escaneo-documentos.md`), salió el tema de reconocer placas
vehiculares costarricenses como una extensión natural del mismo enfoque
(clasificar por estructura + corregir OCR por posición). Se investigó y se
armó una propuesta completa, pero **no corresponde a este repositorio** —
control de acceso de contratistas no necesita leer placas. Queda documentado
acá para cuando se arranque la app correspondiente.

## Las 4 familias propuestas

| Tipo | Formato | Ejemplo | Regex propuesto |
|---|---|---|---|
| Auto particular numérico (legacy) | 1-6 dígitos | `777082` | `^\d{1,6}$` |
| Auto particular alfanumérico | 3 letras + 3 números | `BRW919` | `^[A-Z]{3}\d{3}$` |
| Moto numérica (legacy) | `M`/`MOT` + hasta 6 dígitos | `M 077888` / `MOT077888` | `^(?:M|MOT)\s*\d{1,6}$` |
| Moto alfanumérica (nueva, dic-2025) | `MOT` + 3 números + 3 letras | `MOT123ABC` | `^MOT\d{3}[A-Z]{3}$` |

Simetría notable: auto nuevo es `LLLNNN`, moto nueva es `MOT` + `NNNLLL` —
patrones inversos, útil para desambiguar.

## Reglas de normalización y corrección OCR (mismo patrón que MRZ)

1. **Normalizar** antes de clasificar: mayúsculas, quitar espacios, guiones
   y puntos. `BRW-919`, `BRW 919`, `brw919` → todos a `BRW919`.
2. **Corregir OCR solo por posición estructural conocida** — nunca de forma
   indiscriminada:
   - Posición numérica esperada: `O→0, I/L→1, Z→2, S→5, G→6, B→8`.
   - Posición alfabética esperada: la conversión inversa.
   - Nunca alterar un carácter fuera de una posición cuyo tipo (letra/dígito)
     ya se conoce por el formato.
3. **Vocales permitidas** en la parte alfabética desde oct-2024 (antes solo
   consonantes) — no asumir que solo hay consonantes. `Ñ` no se usa.
4. **Caso ambiguo real**: una moto vieja que pierde la `M`/`MOT` en el OCR
   queda como un número de hasta 6 dígitos, indistinguible de un carro
   numérico legacy. **No forzar clasificación** — usar un tipo explícito
   `VEHICULO_CR_NUMERICO_AMBIGUO` y resolver con evidencia secundaria
   (tamaño/forma de la placa, contexto) si hace falta, en vez de adivinar.
5. Color de placa (ej. verde para eléctricos) es metadato aparte, no cambia
   el formato/familia de la matrícula.

## Modelo de datos propuesto

```kotlin
enum class TipoMatricula {
    AUTO_CR_NUMERICA,
    AUTO_CR_ALFANUMERICA,
    MOTO_CR_NUMERICA,
    MOTO_CR_ALFANUMERICA,
    VEHICULO_CR_NUMERICO_AMBIGUO,
    ESPECIAL_CR,
    DESCONOCIDA,
}
```

Conservar siempre: `textoOcrCrudo`, `matriculaNormalizada`, `tipoDetectado`,
`confianza`, y las correcciones OCR efectuadas (auditable, igual que
`checksumValido` en el lector de documentos de esta app).

## Advertencia importante: esto NO tiene verificación matemática

A diferencia del MRZ (donde cada checksum se pudo verificar de forma
independiente con Python antes de confiar en él), **una placa no tiene
dígito verificador**. Todo lo de arriba descansa en que las fechas/formatos
citados sean exactos:
- Vocales permitidas desde octubre 2024.
- Consecutivo numérico de motos agotado en `999999`, diciembre 2025.
- Nuevo formato de moto `MOT+NNN+LLL` según circular DBM-349-2025.

**Antes de convertir esto en regex de producción**, verificar al menos esos
dos cambios de formato (el de octubre 2024 y el de diciembre 2025) contra
una fuente oficial directamente — con este tipo de dato, un error no se
detecta solo (no hay checksum que falle), se traduce en clasificar mal en
silencio.

**Mensajería:** por la misma razón, el feedback in-cámara para placas no
debería decir "confirmada" con el mismo peso que usamos para un documento
de identidad con MRZ válido (eso sí es matemáticamente verificado). Algo
como "reconocida" es más honesto sobre el nivel de certeza real.

## Alcance

Cubre solo vehículos **particulares** (autos y motos). Fuera de alcance
por ahora: taxis, buses, camiones, placas diplomáticas/gubernamentales,
temporales o de agencia.
