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
- Triggers para mantener índices FTS, hacer inmutable la cédula y proteger el historial
  contra modificaciones, eliminaciones o salidas duplicadas.
- FTS5 con tokenizador de trigramas para búsquedas parciales por cédula, nombre y
  empresa, ignorando mayúsculas y diacríticos.
- Identificadores `INTEGER PRIMARY KEY` sin `AUTOINCREMENT`.
- Fechas persistidas en UTC con formato canónico.

Estas funciones ya dan a la base una buena protección contra errores de programación.
Los triggers y restricciones no son, sin embargo, una defensa contra una persona con
acceso completo al archivo: SQLite no incluye usuarios internos ni permisos
`GRANT`/`REVOKE`.

## Configuración no definida explícitamente

El código sólo fija actualmente `foreign_keys`. No establece de forma explícita:

- `journal_mode`.
- `synchronous`.
- `busy_timeout`.
- `application_id`.
- `trusted_schema`.
- `secure_delete`.
- Rutinas operativas de `optimize`, `quick_check` o `integrity_check`.

En una base nueva SQLite normalmente usa `journal_mode=DELETE` y `synchronous=FULL`.
Además, la versión actual de `rusqlite` crea conexiones con un tiempo de espera de
bloqueo de cinco segundos. No conviene depender de estos valores implícitos porque
pueden variar con la biblioteca, el archivo o una configuración externa.

## Recomendaciones antes de producción

### 1. Respaldo y restauración

Activar la característica `backup` de `rusqlite` y usar la Online Backup API. Una copia
directa del archivo mientras SQLite está abierto no es el mecanismo correcto. La copia
generada debe validarse y la restauración debe probarse con rollback.

Esta tarea se detalla en [plan-respaldos.md](plan-respaldos.md).

### 2. Perfil explícito de durabilidad

Evaluar y probar inicialmente este perfil:

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = DELETE;
PRAGMA synchronous = EXTRA;
PRAGMA busy_timeout = 5000;
```

- `DELETE` mantiene un modelo sencillo para una única conexión y una única instancia.
- `EXTRA` prioriza durabilidad ante un corte eléctrico.
- `busy_timeout` evita depender del valor predeterminado de `rusqlite`.
- Si se agota el tiempo de espera, `SQLITE_BUSY` debe traducirse a un mensaje claro para
  el operador y quedar registrado en el log técnico.

Los valores finales deben confirmarse mediante pruebas de bloqueo, cierre inesperado y
pérdida de energía simulada; no deben aplicarse únicamente por intuición.

### 3. Identificar el archivo de la aplicación

Asignar y comprobar un `PRAGMA application_id` propio. Esto complementa
`user_version` y permite rechazar un archivo SQLite que no pertenezca a Control Acceso.

Debe definirse una constante estable y una migración compatible con las bases ya
existentes.

### 4. Verificar la integridad

Incorporar estas comprobaciones en momentos controlados:

```sql
PRAGMA quick_check;
PRAGMA integrity_check;
PRAGMA foreign_key_check;
```

- `quick_check` puede utilizarse después de una migración o un cierre anormal.
- `integrity_check` debe validar los respaldos y las restauraciones.
- `foreign_key_check` debe ejecutarse además de `integrity_check`, porque comprueba las
  relaciones entre tablas.

Una validación fallida debe impedir reemplazar la base productiva.

### 5. Mantener estadísticas del planificador

Ejecutar `PRAGMA optimize` después de cambios importantes del esquema y de creación de
índices. También puede ejecutarse periódicamente o al cerrar la aplicación, siempre que
se mida su efecto.

### 6. Probar `trusted_schema=OFF`

`PRAGMA trusted_schema = OFF` reduce el riesgo de que un esquema manipulado invoque
funciones o tablas virtuales no seguras. Debe probarse contra todas las migraciones,
triggers y tablas FTS antes de adoptarlo.

### 7. Evaluar tablas `STRICT`

Las tablas `STRICT` refuerzan los tipos almacenados y permiten que las comprobaciones de
integridad detecten valores de un tipo incorrecto. Son convenientes como defensa
adicional, pero convertir las tablas existentes requiere una migración completa.

No es una corrección urgente: el modelo Rust, los parámetros tipados y las restricciones
actuales ya reducen considerablemente este riesgo.

### 8. Definir protección de datos en reposo

`PRAGMA secure_delete=FAST` puede reducir restos recuperables de datos actualizados o
eliminados, pero no cifra la base. Debe evaluarse según el rendimiento y la política de
privacidad.

Si se requiere protección ante robo del equipo o copia del archivo, las alternativas
son cifrado completo del dispositivo, como BitLocker, o una variante cifrada de SQLite,
como SQLCipher. Esto exige una política de almacenamiento y recuperación de claves; no
debe activarse sin ella.

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

