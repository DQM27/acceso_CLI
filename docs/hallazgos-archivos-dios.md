# Hallazgos sobre archivos Dios

Fecha del análisis: 2026-08-20

## Propósito

Este documento identifica archivos grandes o con demasiadas responsabilidades en
`src/`. El número de líneas sirve como señal de búsqueda, no como veredicto: un
archivo largo y cohesivo puede ser correcto, mientras uno más pequeño puede tener
un acoplamiento peligroso.

Para clasificar cada caso se revisaron cuatro factores:

- cantidad de responsabilidades y motivos distintos para cambiar;
- número de subsistemas coordinados;
- mezcla de estado, reglas, infraestructura y presentación;
- dificultad para probar o modificar una parte de forma aislada.

## Resumen

| Prioridad | Archivo | Líneas aproximadas | Diagnóstico |
| --- | --- | ---: | --- |
| Alta | `src/tui/app.rs` | 1.556 | Hotspot y candidato principal; ya existe un plan de extracción |
| Alta | `src/application.rs` | 965 | Fachada útil, pero concentra demasiados casos de uso |
| Media | `src/tui/contratistas/state.rs` | 934 | Estado de pantalla con búsqueda, formulario y navegación mezclados |
| Media | `src/tui/usuarios/state.rs` | 870 | Estado, secretos, formularios y confirmaciones en un solo módulo |
| Media | `src/database/queries/ingresos.rs` | 807 | Une consultas de activos e historial, mapeo y pruebas SQL |
| Media-baja | `src/database/schema.rs` | 804 | Grande principalmente por esquema y migraciones; cohesión aceptable |
| Baja | archivos `render.rs` de 500–600 líneas | 502–603 | Largos, pero mayormente puros y limitados a una pantalla |
| Ninguna inmediata | `src/tui/app/tests.rs` | 1.338 | Archivo de pruebas grande, no un objeto ni archivo Dios de producción |

## 1. `src/tui/app.rs`

### Hallazgo

No es un objeto Dios de dominio, pero sí el principal coordinador y hotspot de la
TUI. Administra el bucle del terminal, navegación, sesión, preferencias, trabajos
de contraseña, despacho de acciones de todas las pantallas y recargas cruzadas.

Ya se redujo al sacar las pruebas y los mensajes de error. El análisis detallado
y el plan vigente están en `docs/refactor-app-y-errores.md`.

### Recomendación

Continuar, en este orden, con las extracciones ya planeadas:

1. trabajos asincrónicos de autenticación y contraseñas;
2. despachadores agrupados por área funcional;
3. navegación global;
4. runtime del terminal.

No conviene crear una abstracción genérica de pantallas mientras los `match`
exhaustivos sigan siendo fáciles de seguir.

## 2. `src/application.rs`

### Hallazgo

`AppCore` es la fachada correcta para mantener una única conexión SQLite, pero el
archivo contiene casos de uso de demasiados dominios:

- arranque y configuración inicial;
- autenticación;
- contratistas y empresas;
- ingresos, salidas e historial;
- usuarios y contraseñas;
- respaldos;
- exportación XLSX.

La estructura no es incorrecta, pero el archivo tiene muchos motivos
independientes para cambiar. Es el siguiente candidato serio después de
`tui/app.rs`.

### Recomendación

Conservar un solo `AppCore` y repartir sus bloques `impl` mecánicamente:

```text
src/application/mod.rs
src/application/autenticacion.rs
src/application/accesos.rs
src/application/catalogos.rs
src/application/usuarios.rs
src/application/respaldos.rs
src/application/historial.rs
```

`mod.rs` debería conservar la estructura, construcción, errores compartidos y
utilidades verdaderamente transversales. Esta separación no exige traits nuevos
ni varias conexiones.

## 3. Estados de Contratistas y Usuarios

### Hallazgo

`contratistas/state.rs` reúne el lenguaje de búsqueda, columnas, formulario,
desplegables, paginación y máquina de estados. `usuarios/state.rs` reúne además el
manejo de secretos, alta/edición, cambio de contraseña y confirmaciones de
activación. Ambos siguen limitados a una pantalla, por lo que son módulos grandes
y cohesivos, pero el costo de modificación ya es alto.

### Recomendación

Extraer sólo conceptos con límites claros:

```text
contratistas/query.rs
contratistas/form.rs
usuarios/form.rs
usuarios/password.rs
```

El estado público y `handle_key` pueden permanecer en `state.rs`. No se recomienda
fragmentar cada enum o cada función en un archivo distinto.

## 4. `src/database/queries/ingresos.rs`

### Hallazgo

El archivo contiene dos familias de lectura diferentes: ingresos activos e
historial. También incluye construcción dinámica de filtros, conversión de filas,
normalización de fechas y pruebas de planes de consulta. El mapeo compartido hace
que la unión sea comprensible, pero ambas consultas pueden evolucionar de forma
independiente.

### Recomendación

Separar por caso de lectura y conservar los conversores compartidos en el módulo:

```text
src/database/queries/ingresos/mod.rs
src/database/queries/ingresos/activos.rs
src/database/queries/ingresos/historial.rs
```

Las pruebas de índices deben quedar cerca de la consulta que verifican.

## 5. `src/database/schema.rs`

### Hallazgo

Su tamaño proviene principalmente de definición de esquema, migraciones y sus
pruebas. Tiene una sola responsabilidad técnica y el orden de las migraciones es
valioso, así que no se considera un archivo Dios urgente.

### Recomendación

Mover cada migración a un módulo o archivo SQL únicamente cuando agregar o revisar
migraciones empiece a ser difícil. Mantener en `schema.rs` la orquestación,
verificación de identidad e integridad de la base.

## 6. Archivos de render y pruebas

Los `render.rs` grandes son funciones puras o casi puras de una pantalla. Se deben
dividir sólo si una sección visual tiene vida propia, por ejemplo tabla,
formulario o diálogo. El tamaño por sí solo no justifica introducir más módulos.

`src/tui/app/tests.rs` es el segundo archivo Rust más largo, pero no es un archivo
Dios: no concentra comportamiento de producción. Puede organizarse por escenarios
para mejorar navegación, aunque hacerlo no reduce acoplamiento del sistema.

## Archivos revisados que no requieren una extracción urgente

- `src/database/backup.rs`: largo, pero cohesivo alrededor del ciclo de respaldo.
- `src/tui/ui_kit/shell.rs`: concentra la composición del marco visual común.
- `src/tui/ui_kit/text_input.rs`: componente autocontenido con comportamiento y
  pruebas estrechamente relacionados.
- `src/tui/historial/render.rs` y `src/tui/contratistas/render.rs`: candidatos a
  división visual futura, no archivos Dios actuales.

## Orden sugerido de trabajo

1. terminar la separación incremental de `tui/app.rs`;
2. repartir los bloques de `AppCore` sin cambiar su API;
3. separar las consultas de activos e historial;
4. extraer formularios de Contratistas y Usuarios;
5. reevaluar tamaños y acoplamiento antes de tocar esquema o renders.

Cada corte debe ser mecánico, conservar comportamiento y cerrar con formato,
pruebas completas y Clippy estricto.
