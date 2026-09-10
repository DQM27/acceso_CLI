# Auditoría de arquitectura y rendimiento — Núcleo Rust de Brisas

**Fecha de consolidación:** 2026-09-10  
**Sistema:** `acceso_CLI` / Brisas Core / SQLite / nube / desktop  
**Naturaleza del documento:** reconstrucción consolidada de la revisión técnica previa del núcleo Rust, complementada con los hallazgos posteriores de cifrado y benchmark. No sustituye un benchmark físico completo en hardware de producción.

---

## 1. Veredicto ejecutivo

El núcleo Rust de Brisas está bien encaminado y no muestra señales de necesitar una reescritura arquitectónica. La mayor parte del riesgo de rendimiento no está en las reglas de negocio ni en SQLite como motor, sino en:

1. cargas demasiado grandes hacia la UI desktop;
2. sincronización nube elemento por elemento;
3. acumulación completa de páginas remotas en memoria;
4. creación repetida de clientes HTTP;
5. aperturas de conexiones SQLite fuera de un punto centralizado;
6. decisiones de durabilidad/cifrado que deben medirse, no asumirse.

La prioridad debe ser optimizar órdenes de magnitud antes de micro-optimizaciones.

---

## 2. Controles positivos del núcleo

### AppCore y concurrencia

- `AppCore` concentra el acceso al núcleo y posee la conexión SQLite principal.
- Las operaciones críticas de ingreso/salida usan transacciones `IMMEDIATE`.
- La validez del actor se vuelve a consultar dentro del flujo de mutación; no se confía ciegamente en una sesión vieja.
- Las decisiones definitivas de acceso se revalidan al confirmar una operación.
- La preparación/preview de un ingreso no se trata como autorización reservada.
- Índices únicos parciales protegen contra dos ingresos activos del mismo contratista y contra reutilización simultánea del mismo gafete.
- El historial guarda una fotografía de la decisión de acceso y datos relevantes del momento del ingreso.

### SQLite

Configuración observada:

```sql
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = EXTRA;
PRAGMA trusted_schema = OFF;
PRAGMA secure_delete = FAST;
```

El esquema y las migraciones tienen controles de integridad y versión. WAL es apropiado para la carga esperada de Brisas.

No se recomienda reducir `synchronous` a `OFF`. Si se busca rendimiento, comparar `EXTRA`, `FULL` y `NORMAL` bajo una prueba reproducible y aceptar un cambio sólo con una decisión explícita de durabilidad.

---

## 3. Hallazgos priorizados

| ID | Prioridad | Hallazgo | Tipo |
|---|---|---|---|
| R-01 | P1 | Desktop puede cargar hasta ~100.000 DTOs y duplicarlos Rust → JSON → JavaScript | Rendimiento/RAM |
| R-02 | P1 | Cola de nube se drena con una petición HTTP por elemento | Red/latencia/batería |
| R-03 | P1 | Paginador remoto trae páginas de 500 pero puede acumular todas en un `Vec` | RAM/latencia |
| R-04 | P1 | Clientes HTTP se reconstruyen en más de un camino | Pooling/TLS/latencia |
| R-05 | P2 | Fallback `Client::new()` puede perder configuración de timeout | Disponibilidad |
| R-06 | P2 | Próximo reintento calculado dinámicamente dificulta indexación eficiente | SQLite/sync |
| R-07 | P2 | Conexiones secundarias requieren política uniforme de apertura/pragmas | Consistencia/seguridad |
| R-08 | P2 | Cifrado de DB debe centralizar apertura y clave para todas las conexiones | Seguridad/arquitectura |
| R-09 | P3 | Ajustes de `synchronous`, cache y micro-queries requieren benchmark | Micro-optimización |

---

## 4. R-01 — Desktop y 100.000 DTOs

El mayor objetivo de RAM en desktop es evitar cargar decenas de miles de filas completas y luego serializarlas hacia WebView/React.

Ruta costosa conceptual:

```text
SQLite
  ↓
Vec<DTO> Rust
  ↓
serialización JSON
  ↓
IPC Tauri
  ↓
objetos JavaScript
  ↓
AG Grid
```

Esto puede duplicar o triplicar temporalmente el volumen lógico de datos.

### Recomendación

Usar datasource paginado/virtual/server-side para entregar bloques pequeños, inicialmente 100–500 filas, y medir.

La UI debe pedir sólo:

- rango visible;
- filtros activos;
- orden actual;
- columnas realmente necesarias.

No optimizar bytes individuales antes de eliminar esta duplicación estructural.

### Estado (verificado contra código, 2026-09-10)

Parcialmente mitigado: el commit `3021767` introdujo `application::CargaCompleta<T>` con
`LIMITE_CARGA_COMPLETA_MAXIMO = 20_000` filas para Historial y Auditoria, avisando con un
toast cuando el resultado real es más grande (`truncado = true`) en vez de recortar en
silencio. Esto no es la paginación server-side de 100–500 filas recomendada arriba, pero
ya elimina el peor caso (~100k sin tope). Contratistas quedó deliberadamente en
`LIMITE_LISTADO_MAXIMO_CARGA_COMPLETA = 100_000` (tabla acotada por plantilla, con
virtualización client-side), decisión consciente y no un descuido. La recomendación de
datasource paginado sigue en pie para Historial/Auditoria si el tope de 20k demuestra ser
insuficiente en uso real.

---

## 5. R-02 — Cola de sincronización

`drenar_cola` procesa operaciones remotas de forma esencialmente serial, con una petición HTTP por elemento pendiente.

Ese patrón multiplica:

- round trips;
- negociación/procesamiento HTTP;
- wakeups de red;
- consumo de batería en móvil;
- tiempo total de recuperación tras periodos offline.

### Recomendación

Introducir batch remoto, empezando por 50–200 elementos por request, manteniendo:

- idempotencia;
- orden cuando sea semánticamente necesario;
- respuesta por elemento;
- reintento sólo de fallos parciales;
- tamaño máximo de payload.

El tamaño final debe decidirse mediante medición, no por intuición.

---

## 6. R-03 — Procesar páginas y liberar

El backend puede paginar de 500 en 500, pero acumular todas las páginas en memoria elimina gran parte de la ventaja.

Preferir:

```text
fetch página
   ↓
validar
   ↓
persistir/transaccionar
   ↓
liberar Vec
   ↓
siguiente página
```

Esto estabiliza el pico de memoria y permite progreso incremental.

---

## 7. R-04/R-05 — Cliente HTTP compartido

Crear `reqwest::Client` repetidamente desperdicia pooling de conexiones y puede repetir trabajo TLS/DNS.

Recomendación: cliente de larga vida propiedad del componente de nube/AppCore o equivalente, con configuración única de:

- timeout;
- TLS;
- headers comunes;
- límites;
- política de red.

Evitar fallbacks silenciosos a `Client::new()` si eso elimina la configuración esperada. Ante fallo de construcción/configuración, devolver un error explícito o usar una configuración fallback equivalente y documentada.

---

## 8. R-06 — Reintentos indexables

Si la elegibilidad de reintento se calcula con expresiones dinámicas sobre fecha + número de intentos, SQLite tiene menos posibilidades de usar un índice simple.

Alternativa recomendada:

```text
proximo_intento_en
```

Persistir la próxima fecha elegible al registrar cada fallo. Luego la consulta puede usar un índice directo por estado/fecha.

Medir antes y después con dataset representativo.

---

## 9. R-07 — Conexiones secundarias

Toda conexión secundaria debe pasar por una fábrica común. Para conexiones de lectura, evaluar:

- flags read-only;
- `PRAGMA query_only = ON`;
- `trusted_schema = OFF`;
- `foreign_keys = ON` cuando corresponda;
- mismo `busy_timeout`/política definida para el producto.

Evitar `Connection::open()` disperso por módulos.

### Estado (verificado contra código, 2026-09-10) — más urgente de lo que sugiere la prioridad P2

Ya existe la fábrica central (`database::connection::open_database` /
`open_database_cifrada` / `aplicar_clave`, ver R-08), pero **dos rutas de producción no
la usan**: exportación de historial en hilo aparte, tanto en CLI
(`src/cli/historial_controller.rs:229`) como en TUI (`src/tui/app/historial_jobs.rs:41`).
Ambas llaman `Connection::open()` crudo y sólo configuran `busy_timeout`; no aplican la
clave de SQLCipher ni el resto de pragmas (`trusted_schema`, `foreign_keys`) que sí recibe
la conexión principal vía `initialize_database`.

Mientras la base sea SQLite plano esto es sólo inconsistencia de pragmas. Pero en cuanto
el cifrado real activado en escritorio (`docs/auditorias/...`, sección 10) se extienda a
CLI/TUI compartiendo el mismo archivo, estas dos conexiones fallarían directamente al
abrir un archivo cifrado sin clave -- no es un tema de "política" sino un punto de ruptura
concreto. Recomendación: subir esto a P1 si CLI/TUI van a tocar alguna vez una base
cifrada, o documentar explícitamente que CLI/TUI sólo operan sobre bases sin cifrar.

---

## 10. Cifrado de base de datos

Brisas está evaluando tres backends de SQLite:

```text
SQLite normal
SQLCipher
SQLite3MC
```

### Principio arquitectónico

El core no debe conocer detalles del motor. Debe existir una única fábrica de conexiones responsable de:

```text
open
 ↓
aplicar clave si corresponde
 ↓
verificar clave
 ↓
pragmas
 ↓
schema/migraciones
```

### Producción Windows

La clave de la base debe ser aleatoria de 32 bytes y protegerse con Windows DPAPI (`Current User`). No hardcodear, no guardar en claro y no derivar de Machine ID/MAC/SID.

### Android

La clave aleatoria de DB debe quedar envuelta/protegida por Android Keystore. No derivarla de `ANDROID_ID`.

### Desarrollo vs producción

Es válido desarrollar normalmente con SQLite estándar para compilación e inspección rápidas y compilar producción con un backend cifrado, siempre que CI ejecute regularmente la misma suite contra los tres motores para evitar divergencia.

Impedir por compilación activar dos motores simultáneamente.

---

## 11. Estado de las pruebas de cifrado

En Windows se confirmó previamente para SQLCipher y SQLite3MC:

- creación de DB cifrada;
- cabecera distinta de `SQLite format 3\0`;
- reapertura con clave correcta;
- rechazo de clave incorrecta;
- ejecución del flujo E2E real de Brisas.

SQLite3MC fue probado inicialmente mediante DLL/import library oficial. Esa ruta añade complejidad de distribución; antes de elegirlo como backend final conviene probar integración estática/amalgamation para un único ejecutable/paquete comparable a la integración vendorizada de SQLCipher.

SQLCipher tiene integración directa vía features de `rusqlite`, pero la versión de SQLCipher incluida por la combinación `rusqlite 0.40.2` / `libsqlite3-sys 0.38.2` observada en el laboratorio reportó `4.14.0 community`, por lo que no se recomienda congelar producción sobre ese bundle sin evaluar enlazar/actualizar a una versión actual.

---

## 12. Benchmark de tres motores

Rama de laboratorio:

`benchmark/sqlite-3way-windows-2026-09-10`

Objetivo:

- mismo runner Windows;
- SQLite normal como baseline;
- SQLCipher;
- SQLite3MC ChaCha20-Poly1305;
- mismo `AppCore` y misma carga;
- cinco rondas;
- 1.000 ciclos completos de ingreso + salida por ronda;
- mediana, promedio, mínimo, máximo y overhead contra SQLite normal.

El tiempo de compilación se excluye del benchmark de runtime.

Los resultados deben incorporarse a este documento cuando la corrida comparativa final sea válida y completa. No declarar ganador basándose en builds, KDF diferentes o runners distintos.

---

## 13. Lo que NO conviene introducir por ahora

No se justifica añadir sólo por rendimiento:

- Tokio en el core local;
- pool de conexiones general;
- ORM;
- CQRS completo;
- Event Sourcing;
- caché global;
- jerarquías genéricas de repositorios;
- más threads sin carga demostrada.

SQLite y un `AppCore` bien delimitado son suficientes para el perfil actual del producto.

---

## 14. Metodología de optimización recomendada

Orden de trabajo:

```text
1. eliminar trabajo O(N) innecesario / duplicaciones grandes
2. reducir round trips y materialización masiva
3. medir consultas/transacciones
4. optimizar milisegundos
5. sólo al final microsegundos/bytes
```

Cada optimización debe tener:

- baseline reproducible;
- dataset representativo;
- misma máquina/runner;
- al menos varias rondas;
- mediana y dispersión;
- prueba de regresión funcional.

---

## 15. Plan de implementación priorizado

### P1

1. Virtualizar/paginar el flujo desktop de listados grandes.
2. Batch para cola nube.
3. Procesamiento página-a-página sin acumular todo el remoto.
4. Cliente HTTP compartido.

### P2

5. Centralizar todas las aperturas SQLite.
6. Política uniforme de conexiones secundarias read-only.
7. Persistir `proximo_intento_en` si `EXPLAIN QUERY PLAN`/benchmark lo justifican.
8. Mantener matriz CI SQLite / SQLCipher / SQLite3MC para cambios de core/DB/sync.

### P3

9. Benchmark `synchronous=EXTRA` vs alternativas sólo si aparece como cuello de botella.
10. Micro-optimizaciones después de resolver los objetivos anteriores.

---

## 16. Criterio de cierre

El núcleo puede considerarse sano desde rendimiento cuando:

- ningún listado grande requiere materializar 100k DTOs en WebView;
- sync no hace una request por evento cuando el backend admite batch;
- paginación remota mantiene memoria acotada;
- existe un único cliente HTTP reutilizable;
- toda conexión DB pasa por fábrica central;
- los tres backends pasan la misma suite relevante;
- los cambios de durabilidad/cifrado están respaldados por benchmark;
- no se introducen capas complejas sin una necesidad medida.

## 17. Conclusión

El núcleo no necesita una arquitectura más pesada para ser rápido. Los mayores beneficios vendrán de reducir transferencias masivas, round trips y duplicación de datos. SQLite sigue siendo apropiado; el cifrado debe tratarse como una variante de backend probada continuamente, no como una implementación separada del dominio.
