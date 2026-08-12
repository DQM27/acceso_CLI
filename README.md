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

La protección del último ROOT activo se aplica en el servicio. Antes de permitir varios
procesos escritores deberá convertirse en una operación transaccional atómica; el diseño
actual está dirigido a una aplicación local con un único flujo de escritura.

## Prototipo TUI

El login visual aislado se ejecuta con `cargo run --example brisas_cli`. Ratatui controla
caracteres, espaciado, líneas y colores, pero no la fuente de la terminal. Para conservar
la apariencia prevista se recomienda una terminal moderna con Cascadia Mono, Cascadia
Code, JetBrains Mono o una fuente monoespaciada similar.
