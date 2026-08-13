# Control de acceso

Aplicación local en Rust para administrar empresas, contratistas, usuarios e ingresos y
salidas de una instalación. SQLite es la fuente de verdad; los repositorios persisten,
los servicios orquestan casos de uso y el dominio contiene reglas puras.

## Reglas principales

| Tipo | PRAIND | Gafete |
|---|---:|---:|
| PRAIND | Sí | Sí |
| IN_HOUSE | Sí | No |
| POR_CORREO | No | Sí |
| SWAT | No | No |
| Personal de ruta | Sí | No |

Personal de ruta es una característica del contratista, no un tipo de ingreso. Un gafete
ausente se persiste como `NULL` y se presenta como `S/G` (sin gafete).

## Desarrollo y verificación

```text
cargo check
cargo test
cargo test --features dev-auth
cargo clippy --all-targets
```

La feature `dev-auth` está deshabilitada por defecto y solo crea una identidad de
navegación en memoria. No escribe en SQLite ni contiene credenciales conocidas. Esa
identidad no es un actor válido para operaciones auditadas porque su ID no satisface las
claves foráneas de ingreso y salida; para esas operaciones debe seleccionarse o
autenticarse un usuario persistido en una base de desarrollo separada.

Los usuarios se desactivan, no se eliminan, para conservar las referencias históricas.

## Persistencia y tiempo

El esquema evoluciona mediante migraciones secuenciales y `PRAGMA user_version`. La
versión solo cambia después de completar la migración dentro de una transacción.

Las fechas y horas representan la hora local de Costa Rica y de la instalación. Se
persisten como texto con formato `YYYY-MM-DD HH:MM:SS`; actualmente no representan UTC.
Cambiar esta política requiere una migración de datos explícita.

La creación del ROOT inicial y la protección del último ROOT activo se ejecutan mediante
transacciones SQLite `IMMEDIATE`, de modo que la lectura de la condición y su escritura
son una sola operación atómica incluso con varios escritores.

## Arquitectura de arranque

```text
TUI → AppCore → Services / Queries → Repositories → SQLite
```

`AppCore` posee la única conexión de la aplicación y compone repositorios y servicios
temporales para cada caso de uso; no contiene reglas de negocio. `cargo run` utiliza
`control_acceso.db` en el directorio actual. La variable `CONTROL_ACCESO_DB` permite
indicar una ruta distinta sin codificar rutas absolutas.

La búsqueda textual usa índices separados SQLite FTS5 con tokenizer `trigram` para
contratistas, empresas y usuarios. La preparación técnica es compartida, pero cada
consulta conserva su propio read model y nunca mezcla tipos de resultados. Desde tres
caracteres la búsqueda es indexada e insensible a mayúsculas y diacríticos; uno o dos
caracteres usan el `LIKE` existente, siempre limitado por la consulta correspondiente.

Si la base está vacía, el núcleo exige crear el ROOT inicial antes del login. La pantalla
visual para ese paso aún no forma parte del prototipo; el caso de uso ya está disponible
mediante `AppCore::crear_root_inicial`.

## Prototipo TUI

El login visual aislado se ejecuta con `cargo run --example brisas_cli`. Ratatui controla
caracteres, espaciado, líneas y colores, pero no la fuente de la terminal. Para conservar
la apariencia prevista se recomienda una terminal moderna con Cascadia Mono, Cascadia
Code, JetBrains Mono o una fuente monoespaciada similar.
