# Evaluación y recomendaciones de SQLite

Este documento registra las capacidades de SQLite relevantes para Control Acceso, la
configuración utilizada actualmente y los ajustes que conviene evaluar antes de
producción.

La aplicación funciona en una sola terminal, sin conexión obligatoria a internet, con
una base local y una única instancia por archivo. SQLite es adecuado para este modelo:
es una base embebida, transaccional y diseñada para almacenamiento local con poca
concurrencia de escritura.

Referencias principales:

- [Usos apropiados de SQLite](https://www.sqlite.org/whentouse.html).
- [PRAGMAs oficiales](https://www.sqlite.org/pragma.html).
- [Write-Ahead Logging](https://www.sqlite.org/wal.html).
- [Online Backup API](https://www.sqlite.org/backup.html).
- [Tablas STRICT](https://www.sqlite.org/stricttables.html).
- [FTS5](https://www.sqlite.org/fts5.html).

## Capacidades utilizadas actualmente

- `rusqlite` con la característica `bundled`: SQLite se compila dentro del ejecutable
  y no depende de una instalación del sistema.
- Archivo local en una ruta estable dentro de los datos del usuario, con
  `CONTROL_ACCESO_DB` como sobreescritura explícita.
- Claves foráneas activadas mediante `PRAGMA foreign_keys = ON`.
- Restricciones `NOT NULL`, `UNIQUE` y `CHECK` para impedir datos inválidos.
- Índices normales, parciales y únicos para acelerar consultas y proteger reglas como
  un solo ingreso activo por contratista y un solo gafete activo.
- Transacciones `IMMEDIATE` para migraciones y operaciones críticas de escritura.
- Transacción de lectura para obtener de forma coherente el total y la página del
  historial.
- Migraciones secuenciales controladas con `PRAGMA user_version`.
- Triggers para mantener índices FTS y proteger el historial contra modificaciones,
  eliminaciones o salidas duplicadas.
- FTS5 con tokenizador de trigramas para búsquedas parciales por cédula, nombre y
  empresa, ignorando mayúsculas y diacríticos.
- La cédula vigente de un contratista puede corregirse únicamente por un usuario ROOT o
  Administrador. Cada cambio efectivo queda auditado con el actor y los valores anterior
  y nuevo; los movimientos conservan por separado la cédula copiada al registrar el
  ingreso, por lo que una corrección no reescribe el historial.
- Identificadores `INTEGER PRIMARY KEY` sin `AUTOINCREMENT`.
- Fechas persistidas en UTC con formato canónico.

Estas funciones ya dan a la base una buena protección contra errores de programación.
Los triggers y restricciones no son, sin embargo, una defensa contra una persona con
acceso completo al archivo: SQLite no incluye usuarios internos ni permisos
`GRANT`/`REVOKE`.

## Configuración no definida explícitamente

**Actualizado (2026-08-18): `journal_mode`, `synchronous`, `busy_timeout`,
`application_id`, `trusted_schema` y `secure_delete` ya se fijan explícitamente** en
`initialize_database` (`src/database/schema.rs`), junto con una verificación de
`PRAGMA quick_check` en cada apertura y `PRAGMA optimize` al cerrar `AppCore`. Ver el
perfil exacto en la sección 2 más abajo. El cifrado en reposo (SQLCipher/BitLocker,
sección 8) sigue abierto como decisión de política — ver `docs/pendientes.md`;
`secure_delete` reduce restos recuperables pero no cifra la base.

## Recomendaciones antes de producción

### 1. Respaldo y restauración

**Implementado.** `rusqlite` con la característica `backup` y la Online Backup API;
copia validada, restauración con rollback automático, pantalla Configuración →
Respaldos, respaldo obligatorio pre-migración y respaldo automático diario con
retención. Detalle histórico del diseño e implementación en el `git log` de los commits
que tocaron `src/database/backup.rs` (el plan de fases que lo guió se consolidó en
`docs/pendientes.md`, que conserva sólo la acción "Eliminar respaldo" aún no acordada).

### 2. Perfil explícito de durabilidad — implementado

`initialize_database` (`src/database/schema.rs`) fija este perfil en cada apertura,
antes de abrir la transacción de migración:

```sql
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA journal_mode = DELETE;
PRAGMA synchronous = EXTRA;
PRAGMA trusted_schema = OFF;
PRAGMA secure_delete = FAST;
```

- `DELETE` mantiene un modelo sencillo para una única conexión y una única instancia.
- `EXTRA` prioriza durabilidad ante un corte eléctrico.
- `busy_timeout` evita depender del valor predeterminado de `rusqlite`/SQLite (que es
  `0`, es decir, fallar de inmediato ante cualquier bloqueo transitorio).
- `secure_delete = FAST` sobrescribe con ceros el espacio liberado por un `DELETE`/
  `UPDATE` en vez de dejarlo recuperable en el archivo, a cambio de un costo de
  escritura marginal. No cifra la base — ver sección 8 para eso.
- `trusted_schema = OFF` se probó contra toda la suite (migraciones, triggers y las
  tres tablas FTS5) sin romper nada.
- Tratar `SQLITE_BUSY` como error observable para el operador se evaluó y se **descartó
  a propósito** (decisión explícita del usuario, mismo motivo que el resto de "errores
  observables": no proporcional a lo que esta app necesita) — no es un pendiente, es
  una decisión ya cerrada.

Cubierto por `tests/configuracion_sqlite.rs::fija_el_perfil_de_durabilidad_esperado`.
Los valores se aplicaron siguiendo la evaluación de este documento, no sólo por
intuición; falta todavía la prueba de pérdida de energía simulada en un entorno real
antes de darlos por definitivos en producción.

### 3. Identificar el archivo de la aplicación — implementado

`APPLICATION_ID` (`src/database/schema.rs`, bytes de "BRIS") se verifica y adopta en
`verificar_identidad_de_archivo`: una base nueva o creada antes de este cambio
(`application_id = 0`) lo adopta sin fricción; un archivo con un `application_id`
ajeno se rechaza con `SchemaError::BaseAjena` antes de tocar el esquema.

Cubierto por `tests/configuracion_sqlite.rs` (`adopta_el_application_id_...`,
`rechaza_un_archivo_con_application_id_de_otra_aplicacion`).

### 4. Verificar la integridad — `quick_check` implementado, el resto pendiente

`verificar_integridad_rapida` ejecuta `PRAGMA quick_check` en cada apertura, antes de
migrar, y rechaza el archivo con `SchemaError::IntegridadInvalida` si no responde
`"ok"`. Cubierto por `tests/configuracion_sqlite.rs::rechaza_un_archivo_truncado...`.

`integrity_check` y `foreign_key_check` (más costosos, no aptos para cada apertura)
siguen pendientes — su lugar natural es la validación de respaldos
([plan-respaldos.md](plan-respaldos.md)), no el arranque normal de la aplicación.

Una validación fallida debe impedir reemplazar la base productiva.

### 5. Mantener estadísticas del planificador — implementado

`AppCore` ejecuta `PRAGMA optimize` en su `impl Drop` (`src/application.rs`), es decir,
al cerrar la aplicación normalmente — el punto de cierre único, ya que `AppCore` es
dueño exclusivo de la conexión. Es mantenimiento, no corrección: un fallo ahí se
descarta silenciosamente en vez de impedir el cierre. Cubierto por
`tests/configuracion_sqlite.rs::drop_de_appcore_no_entra_en_panico_al_optimizar_al_cerrar`.

### 6. Probar `trusted_schema=OFF` — implementado

`PRAGMA trusted_schema = OFF` ya se aplica en cada apertura. Se probó contra toda la
suite de pruebas (migraciones, los triggers de inmutabilidad del historial, la auditoría
de correcciones de cédula y las tres tablas FTS5 de contratistas/empresas/usuarios) sin
ninguna regresión.

### 7. Evaluar tablas `STRICT`

Las tablas `STRICT` refuerzan los tipos almacenados y permiten que las comprobaciones de
integridad detecten valores de un tipo incorrecto. Son convenientes como defensa
adicional, pero convertir las tablas existentes requiere una migración completa.

No es una corrección urgente: el modelo Rust, los parámetros tipados y las restricciones
actuales ya reducen considerablemente este riesgo.

### 8. Definir protección de datos en reposo — parcialmente implementado

`PRAGMA secure_delete=FAST` ya está activado (ver sección 2): reduce restos
recuperables de datos actualizados o eliminados, pero **no cifra la base**.

Si se requiere protección ante robo del equipo o copia del archivo, las alternativas
son cifrado completo del dispositivo, como BitLocker, o una variante cifrada de SQLite,
como SQLCipher. Ninguna de las dos está implementada — ambas exigen una decisión de
política (almacenamiento y recuperación de claves) antes de tocar código:

- **BitLocker** es una decisión de despliegue, no de desarrollo: la activa un
  administrador a nivel de sistema operativo, sin tocar Control Acceso. Cubre el
  escenario de robo del equipo apagado.
- **SQLCipher** es la opción más fuerte (cifra cada página del archivo), pero su
  integración vía `rusqlite` es frágil en Windows específicamente: la variante
  `bundled-sqlcipher-vendored-openssl` requiere Perl y NASM instalados para compilar
  OpenSSL desde código fuente, y hay reportes de que no compila de forma confiable en
  Windows ([rusqlite#1025](https://github.com/rusqlite/rusqlite/issues/1025)); la
  variante `bundled-sqlcipher` evita compilar OpenSSL pero exige tener OpenSSL ya
  instalado en cada máquina de build y apuntarlo a mano con `OPENSSL_DIR`. No es un
  simple flag de Cargo en este sistema operativo. Si la amenaza real es "un respaldo
  termina en un medio sin cifrar", cifrar sólo los archivos exportados de
  `plan-respaldos.md` es una alternativa más liviana que reintentar SQLCipher.

## Funciones que no conviene activar actualmente

### WAL

WAL mejora la convivencia entre lectores y escritores, pero añade archivos `-wal` y
`-shm`, checkpoints y nuevas decisiones operativas. Con una instancia, una conexión
principal y escrituras breves ofrece poco beneficio. Debe reconsiderarse únicamente si
la interfaz pasa a utilizar varios workers y conexiones simultáneas.

### `AUTOINCREMENT`

`INTEGER PRIMARY KEY` ya genera los identificadores necesarios. `AUTOINCREMENT` añade
escrituras y almacenamiento para impedir reutilizar identificadores eliminados. Los
movimientos históricos no se eliminan, por lo que no aporta valor.

### `auto_vacuum`

El historial crece de forma acumulativa y casi no libera páginas. No debe activarse sin
medir primero crecimiento y espacio desperdiciado. Un `VACUUM` completo también bloquea
y reescribe el archivo, por lo que debe tratarse como mantenimiento controlado.

### Ajustes manuales de memoria

No modificar `cache_size`, `mmap_size`, `temp_store` o `threads` sin mediciones y casos
de carga reproducibles. Los valores predeterminados son suficientes para el volumen
previsto actualmente.

### Shared cache y extensiones dinámicas

No usar shared cache ni habilitar `load_extension`. No resuelven una necesidad presente
y las extensiones dinámicas amplían innecesariamente la superficie de ataque.

### JSON, R-Tree y BLOB incremental

- JSON no mejora el modelo actual; contratistas, empresas y movimientos son datos
  relacionales.
- R-Tree sólo sería útil para búsquedas geográficas o multidimensionales.
- BLOB incremental sólo sería útil si la base almacenara archivos o imágenes grandes.

## Capacidades que pueden servir en el futuro

- Abrir respaldos en modo de sólo lectura para explorarlos sin riesgo.
- Vistas, funciones ventana y CTE para reportes complejos.
- Índices por expresión para identidades normalizadas.
- Savepoints si aparecen operaciones transaccionales anidadas.
- Hooks de trazado para diagnóstico y medición, sin registrar secretos.
- Funciones SQL personalizadas únicamente si existe una necesidad que no pueda
  resolverse claramente en Rust.

## Límites que SQLite no resuelve

SQLite no proporciona por sí solo:

- Respaldos programados ni restauración con rollback.
- Cifrado en la compilación actual.
- Usuarios internos y permisos por tabla.
- Protección contra un administrador del sistema con acceso al archivo.
- Replicación o sincronización por internet.
- Varios escritores verdaderamente simultáneos.
- Una auditoría imposible de alterar por alguien con control total del equipo.

Si en el futuro varias terminales necesitan operar sobre los mismos datos, no debe
compartirse directamente el archivo SQLite por red. En ese escenario habría que añadir
un servidor de aplicación o migrar a una base cliente-servidor.

## Orden propuesto

1. Implementar respaldo y restauración verificada.
2. Definir y probar `journal_mode`, `synchronous` y `busy_timeout`.
3. Manejar `SQLITE_BUSY` como error operativo comprensible.
4. Añadir y validar `application_id`.
5. Incorporar comprobaciones de integridad.
6. Ejecutar `PRAGMA optimize` en momentos controlados.
7. Probar `trusted_schema=OFF`.
8. Evaluar tablas `STRICT` en una migración posterior.
9. Definir la política de cifrado según el riesgo físico del equipo.

