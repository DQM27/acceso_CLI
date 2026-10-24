# Importador de datos desde Excel — plan

Estado: **propuesta, sin código todavía** (2026-10-22). Proyecto aparte:
se usará en acceso_CLI y en proyectos futuros (decisión del dueño).

Herramienta en Python para cargar datos de clientes desde Excel o CSV
(contratistas, empresas, encargados KOF, gafetes…) limpiándolos y
validándolos antes de que toquen la base.

## Por qué

- Cada cliente nuevo llega con sus listas en Excel, con formatos distintos:
  cédulas con guiones, nombres en minúscula, fechas como texto, empresas
  escritas de tres formas, filas vacías y celdas combinadas.
- Hoy esas cargas se hacen a mano por SQL (por ejemplo, los 1.438
  encargados KOF).
- Un dato mal cargado lo sufre después el operador en el puesto de control.

## Arquitectura: núcleo reutilizable + perfil por proyecto

```
importador/            núcleo (repositorio propio, cualquier proyecto)
  leer        Excel/CSV; detecta hoja y fila de encabezados
  mapear      columnas del archivo → campos ("Céd.", "Identificación" → cedula)
  limpiar     espacios, mayúsculas, tildes, fechas, números
  validar     reglas por campo y por fila
  deduplicar  repetidos dentro del archivo y contra lo ya existente
  informar    Excel de resultado: filas OK, corregidas y rechazadas con motivo
  cargar      sólo con aprobación; idempotente

perfiles/acceso/       en este repositorio
  contratistas, empresas, encargados KOF, gafetes
```

## Flujo: siempre en dos pasos

1. **Simulación** (por defecto): no toca la base. Entrega el Excel de
   resultado, por ejemplo "438 filas: 410 listas, 20 corregidas (cédula con
   guiones), 8 rechazadas (cédula con letras)".
2. **Carga**: sólo tras revisar el informe. Queda constancia de quién cargó
   qué archivo y cuándo.

## Buenas prácticas

- **Las mismas reglas que la app.** Validar con el crate `reglas` (Rust),
  el que ya comparten escritorio, móvil y Edge Functions, llamándolo desde
  Python (PyO3). Nunca copiar las reglas a Python: se desincronizan.
- **Coincidencias aproximadas con confirmación.** "BROOMDAY S.A." y
  "Broomday" se proponen como la misma empresa; no se unen solas.
- **Mapeo por configuración**, un archivo por cliente o formato: un cliente
  nuevo no requiere programar.
- **Carga respetando los permisos del proyecto**: en acceso_CLI, a través
  de RPC o Edge Functions de administración, nunca escribiendo directo
  saltándose las reglas de negocio.
- **Datos personales**: los archivos de clientes no se suben al
  repositorio; las pruebas usan datos inventados.
- **Pruebas** con archivos sucios a propósito (filas vacías, celdas
  combinadas, fechas como texto, encabezados en la fila 3).

Herramientas sugeridas: `uv` (entorno), `polars` o `pandas` + `openpyxl`
(lectura), `pydantic` (validación), `rapidfuzz` (coincidencias), `pytest`.

## Usos relacionados (mismo núcleo)

- Detector de registros de prueba e ingresos sospechosos (por ejemplo, de
  menos de un minuto) para revisar antes de borrar.
- Detección de personas duplicadas con la cédula escrita distinto.
- Reportes mensuales para clientes en Excel o PDF.

## Pendiente de decidir

- Nombre y repositorio del núcleo.
- Un Excel real de un cliente (o unas filas con datos cambiados) para
  diseñar el mapeo y la limpieza sobre problemas reales.
