# Brisas — laboratorio de bases cifradas

Este directorio es deliberadamente independiente de la base de datos usada por Brisas. Su objetivo es probar en CI si las alternativas de SQLite cifrado compilan y funcionan realmente en Windows antes de modificar `AppCore`.

## Criterio de aprobación

Cada prueba debe demostrar cuatro cosas:

1. crea una base nueva con cifrado activado;
2. la cabecera del archivo no contiene `SQLite format 3\0` en claro;
3. la base se reabre y devuelve el dato esperado con la clave correcta;
4. una clave incorrecta no permite leer `sqlite_master`.

## Alternativas evaluadas

- `sqlcipher`: `rusqlite 0.40.2` + `bundled-sqlcipher-vendored-openssl`. Compila SQLCipher y OpenSSL desde las fuentes vendorizadas del grafo Cargo.
- `sqlite3mc`: `rusqlite 0.40.2` enlazado a SQLite3 Multiple Ciphers 2.5.1 x64 oficial y usando ChaCha20-Poly1305. El workflow fija y verifica el SHA-256 del ZIP antes de enlazarlo.

La rama de laboratorio no cambia `Cargo.toml`, `AppCore`, el esquema ni los datos de producción. Que este smoke test pase prueba compatibilidad básica de compilación/enlace/cifrado; no prueba todavía migración, DPAPI, rendimiento ni integración completa con Brisas.
