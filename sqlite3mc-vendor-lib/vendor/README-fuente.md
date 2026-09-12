# Procedencia de este vendor

Archivos copiados sin modificar del release oficial de _SQLite3 Multiple
Ciphers_:

- Origen: https://github.com/utelle/SQLite3MultipleCiphers/releases/tag/v2.5.1
- Asset: `sqlite3mc-2.5.1-sqlite-3.53.4-amalgamation.zip`
- SHA-256 (verificado contra `sqlite3mc-2.5.1-SHA256SUMS` del mismo release):
  `4125f8ff275ea953dabb3289331b20a0e76d4fc060f57148f4a5df3bf3b0d5e0`
- Versión SQLite3MC: 2.5.1 (sobre SQLite 3.53.4)

Archivos vendorizados:

- `sqlite3mc_amalgamation.c` / `.h` -- el motor completo (SQLite + capa
  multi-cipher), reemplazo drop-in de `sqlite3.c`/`sqlite3.h` normal.
- `sqlite3.h` -- header original de SQLite, sin modificar (mismo API que
  usa `libsqlite3-sys` para generar sus bindings).

No se vendorizó `shell3mc_amalgamation.c` (shell de línea de comandos, no
hace falta para enlazar contra `rusqlite`) ni `sqlite3ext.h` (API de
extensiones cargables en runtime, tampoco necesaria acá).

Cifrado por defecto: ChaCha20-Poly1305 (`CODEC_TYPE_CHACHA20`, el default
del propio amalgamation si no se sobreescribe `CODEC_TYPE` -- ver
`sqlite3mc_amalgamation.c`).
