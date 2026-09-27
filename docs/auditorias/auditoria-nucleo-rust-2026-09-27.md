# Auditoría del núcleo Rust: diseño, organización y rendimiento (2026-09-27)

Documento pensado para que **un agente lo aplique**. **Prioridad: antes
de seguir con la sincronización / realtime** (rama
`claude/realtime-oficial`). Buena parte de los problemas del realtime
vinieron de que la sincronización está repetida en varios lugares; hay
que pulir el núcleo primero y después montar el realtime encima.

Alcance: `src/` (crate `control_acceso`, ~36.000 líneas),
`mobile/rust-core/` (UniFFI → `Nucleo`), `desktop/src-tauri/` (comandos
Tauri → `GuiState`).

## Reglas para el agente

- Leer `AGENTS.md` y respetarlo (español; nunca el término prohibido para
  el puesto de control: usar "puesto de control", "portería" o "punto de
  acceso"; commit documentado + push por cada cambio exitoso).
- **Nunca** tocar Supabase de producción (`control-acceso-nube`). Si hace
  falta probar contra la nube, sólo staging (`control-acceso-staging`) y
  limpiar lo que se cree.
- **Sin cambio de comportamiento** salvo donde el punto lo diga. Mismos
  mensajes, mismo orden de etapas, mismos contadores en los resúmenes.
- Motor único: SQLite3MC (`SQLITE3_LIB_DIR` apuntando a
  `sqlite3mc-vendor-lib/dist`, `SQLITE3_NO_PKG_CONFIG=1`). No agregar
  `rusqlite/bundled` en ningún `Cargo.toml` (reemplaza el motor por
  unificación de features).
- Un punto = un commit o más, en el orden de la sección "Orden".

## Veredicto

El diseño **es sostenible y correcto** para una app local-first:

- `AppCore` (`src/application/mod.rs`) es la fachada única: una
  `Connection` SQLite + reloj + caché de token. Cada host lo envuelve en
  un `Mutex` (`GuiState::core()` en escritorio, `Nucleo::core_lock()` en
  móvil). SQLite admite un solo escritor, así que un candado no es un
  límite artificial; las operaciones locales son de milisegundos.
- Capas con sentido: `domain` (reglas puras, **no** depende de
  `database`/`nube`/`rusqlite`), `services`, `application`, `database`
  (repositorios/consultas), `nube`.
- Escritorio y móvil comparten las mismas reglas de negocio.

**No** hace falta pool de conexiones, actores, CQRS ni async en el
núcleo: sería sobreingeniería. Los problemas son de **uso** del diseño y
de **organización**, no del diseño en sí.

## Hallazgos

### N1 (crítica): la orquestación de la sincronización está copiada 4 veces — HECHO en `claude/nucleo-n1-n3`

Resuelto: `nube::sincronizar` / `nube::recibir` (`src/nube/orquestacion.rs`)
con `AlcanceSincronizacion` (`src/nube/alcance.rs`, traído de
`claude/realtime-oficial`) y `PerfilDispositivo` (el móvil no guarda
ningún historial del sitio: decisión del dueño, el historial vive en el
escritorio y el panel web). Eran cinco copias, no cuatro (también
`configurar_dispositivo_inicial_con_secreto` del puente). Se quitaron
`AppCore::sincronizar_con_nube`, `AppCore::refrescar_catalogo_sin_sesion`
y `Nucleo::sincronizar_con_nube`, que nadie llamaba.


La misma secuencia de ~13 etapas (`drenar_cola`, `recibir_cierres_*`,
`recibir_*_abiertos`, `recibir_catalogo_*`, `recibir_historial_*`,
`recibir_citas_del_sitio`, conflictos...) vive en:

| Dónde | Función |
|---|---|
| `src/application/nube.rs` | `AppCore::sincronizar_con_nube` (~l. 304), `refrescar_catalogo_sin_sesion` (~l. 276), `configurar_dispositivo_inicial` (~l. 393) |
| `desktop/src-tauri/src/comandos/nube.rs` | `intentar_sincronizacion` (~l. 232) |
| `mobile/rust-core/src/lib.rs` | `intentar_sincronizar_con_nube` (~l. 3009) |
| `mobile/rust-core/src/lib.rs` | `intentar_sincronizar_con_secreto` (~l. 3151) |

Cada cambio (una etapa nueva, un orden distinto, un alcance) hay que
repetirlo a mano y ya se desincronizaron entre sí. Es la causa raíz de
que el realtime fuera frágil.

**Hacer:**
1. En `src/nube/` crear **una** función, p. ej.
   `pub fn sincronizar(conexion: &Connection, contexto: &ContextoSincronizacion, alcance: AlcanceSincronizacion) -> Result<ResumenSincronizacionNube, SincronizacionError>`
   con todas las etapas en el orden actual y un resumen con todos los
   contadores que hoy usan escritorio y móvil.
2. Traer `AlcanceSincronizacion` desde la rama `claude/realtime-oficial`
   (`src/nube/alcance.rs`, con sus tests) y usarlo como parámetro;
   `AlcanceSincronizacion::completo()` = comportamiento actual.
3. Reemplazar las 4 copias por llamadas a esa función. Las diferencias
   reales entre hosts (p. ej. escritorio refresca la sesión de Supabase
   Auth; móvil omite `recibir_historial_visitas_del_sitio` a propósito,
   ver el comentario en `lib.rs` ~l. 3091) se expresan con el alcance o
   con parámetros explícitos, **no** con copias. Documentar cada
   diferencia en el doc-comment.
4. Los hosts sólo resuelven credenciales/contexto y convierten el
   resumen a su tipo (UniFFI / Tauri).

**Terminado cuando:** `grep -n "recibir_catalogo_del_sitio" -r desktop mobile src/application`
devuelve sólo la función nueva (o nada fuera de `src/nube`); tests
existentes pasan; test nuevo de la función con alcance completo y
parcial.

### N2 (alta): red con el candado del núcleo tomado (móvil) — HECHO en `claude/nucleo-n1-n3`

Resuelto: las funciones que hacían red con el candado tomado eran las
variantes "legado" del puente que reciben un directorio
(`cerrar_ingreso_remoto`, `cerrar_ingreso_proveedor_remoto`,
`cerrar_prestamo_gafete_provisional_remoto`, `sesion_realtime_nube`,
`configurar_dispositivo_inicial`, `gafete_ocupado_en_sitio`); Kotlin sólo
usaba las `_con_secreto`, que ya soltaban el candado. Se quitaron, junto
con los métodos de red de `AppCore` que quedaron sin llamadores. La regla
quedó escrita en el doc-comment de `AppCore`; única excepción documentada:
`configurar_dispositivo_inicial` del escritorio (una sola vez, base vacía).


`Nucleo::cerrar_ingreso_remoto` / `cerrar_ingreso_remoto_con_secreto`
(`mobile/rust-core/src/lib.rs` ~l. 2538-2600) y los de proveedor llaman
a `self.core_lock().cerrar_ingreso_remoto(...)`, que hace HTTP
(`src/application/nube.rs` ~l. 772-800) **con el candado tomado**. Con
red lenta la app entera se congela hasta `TIMEOUT_HTTP` (10 s,
`src/nube/cliente.rs`). Inconsistente con `gafete_ocupado_en_sitio`, que
ya lo hace bien (toma el candado sólo para autorizar y suelta antes de la
red; ver su comentario ~l. 2380).

**Hacer:** mismo patrón que `gafete_ocupado_en_sitio`: candado → autorizar
/ leer lo necesario → soltar → red con `conexion_secundaria()` o sin
conexión → candado sólo para escribir el resultado. Revisar **todo**
`AppCore` en `src/application/nube.rs` y todos los `core_lock()` /
`core()` de los hosts buscando cualquier otra llamada HTTP bajo candado
(`configurar_dispositivo_inicial` es de una sola vez: aceptable, pero
documentarlo).

**Regla a dejar escrita** en el doc-comment de `AppCore`: *nunca red con
el candado tomado*.

### N3 (alta): el historial local crece para siempre

No hay retención, ni `PRAGMA optimize`, ni `VACUUM` en `src/`. Escritorio
**y móvil** (`recibir_historial_del_sitio`, `lib.rs` ~l. 3089) guardan
el historial de todo el sitio. Con meses/años: más disco, sincronización
y búsquedas más lentas, más memoria.

**Hacer:**
1. Retención local configurable del historial **recibido de la nube**
   (p. ej. 6 meses en móvil, 12-24 en escritorio). La nube conserva todo.
   Nunca borrar filas con cambios pendientes en la cola de salida ni
   ingresos abiertos.
2. `PRAGMA optimize` al cerrar / tras sincronizaciones grandes.
3. `PRAGMA incremental_vacuum` (requiere `auto_vacuum = INCREMENTAL`;
   evaluar migración) o `VACUUM` ocasional fuera del camino caliente.

Tests: la purga respeta cola de salida y abiertos.

### N4 (media): archivos gigantes

| Archivo | Líneas | Propuesta |
|---|---|---|
| `src/nube/sincronizacion.rs` | 8.365 (93 fns) | `src/nube/sincronizacion/{cola.rs, ingresos.rs, proveedores.rs, gafetes_provisionales.rs, citas_visitas.rs, catalogo.rs, historial.rs, conflictos.rs, paginado.rs}` + `mod.rs` que re-exporte lo público (API sin cambios). |
| `src/database/schema.rs` | 4.189 | Migraciones a `src/database/migraciones/NNN_nombre.rs` (o `.sql` con `include_str!`), `schema.rs` sólo con el registro ordenado. **No** cambiar el contenido ni el orden de ninguna migración. |
| `mobile/rust-core/src/lib.rs` | 4.172 | Módulos por tema como `src/application/` (`accesos.rs`, `nube.rs`, `rutas.rs`, `proveedores.rs`...). `#[uniffi::export]` funciona en `impl` repartidos; verificar que el Kotlin generado no cambie (`uniffi-bindgen ... --language kotlin` y diff contra `mobile/android/.../uniffi/control_acceso_mobile/control_acceso_mobile.kt`). |

Sólo mover código. Hacer N4 **después** de N1 (N1 achica
`sincronizacion.rs` y `lib.rs`).

### N5 (media): SQL fuera de `database`

`rusqlite` se usa directo en 10 archivos de `src/services/` y 11 de
`src/application/`. La regla "sólo `database` toca SQL" no se cumple del
todo. No es urgente; **no** reescribir todo. Criterio: al tocar un
servicio por otro motivo, mover su SQL al repositorio/consulta
correspondiente. Nada de ORM ni capa nueva.

### N6 (media): memoria en descargas grandes

`obtener_json_paginado` junta todas las páginas en un `Vec` antes de
guardar (8 usos: catálogo ~l. 3587-3660 y catálogo de rutas ~l.
4145-4160 de `sincronizacion.rs`). Normalmente es incremental y chico,
pero la **primera** sincronización de un equipo nuevo trae todo.
**Hacer:** usar `obtener_json_paginado_con` (guarda página por página,
ya se usa en 5 lugares) donde el orden de escritura lo permita
(ojo: FKs entre empresas → contratistas; mantener el orden de tablas).

### N7 (baja): `AppCore` crece sin límite

130 métodos públicos repartidos en `src/application/*.rs`. Hoy se
aguanta porque cada tema tiene su archivo. **No** hacer nada ahora; si
pasa de ~200, agrupar en sub-fachadas (`core.rutas()`, `core.nube()`).

### N8 (baja): micro-rendimiento

- 90 `prepare()` y 0 `prepare_cached()`: cambiar en consultas de uso
  frecuente (búsquedas, listados de activos). Ganancia pequeña.
- Búsquedas `PLEGAR(col) LIKE '%x%'` recorren toda la tabla
  (`database/queries/contratistas.rs`, `usuarios.rs`, repos de rutas y
  proveedores). Irrelevante con miles de filas; si algún catálogo llega a
  decenas de miles: columna `*_plegado` indexada o FTS5.
- `synchronous = EXTRA` es deliberado (durabilidad ante cortes de luz).
  **No** cambiar.

## Lo que ya está bien (no tocar)

- `domain` sin dependencias de infraestructura.
- Escrituras de sincronización en transacciones; bajada incremental por
  `updated_at` con marca de agua.
- Un solo cliente HTTP compartido con timeout (`nube/cliente.rs`).
- Escritorio: sincronización, historial y auditoría usan
  `conexion_secundaria()` (no bloquean el candado principal). WAL permite
  leer en paralelo.
- Clave de SQLite3MC en hex crudo: abrir conexiones no paga derivación.

## Orden

1. **N1** (sincronización única): lo más importante.
2. **N2** (red fuera del candado).
3. **N3** (retención + optimize).
4. **N4** (partir archivos), 5. **N6**, 6. **N5/N8** oportunistas.
7. Recién entonces retomar realtime: rehacer `claude/realtime-oficial`
   sobre `main` ya pulido. Con N1 hecho, la sincronización por tabla es
   sólo pasar `AlcanceSincronizacion::desde_tablas(...)` a la función
   única; lo de clientes (`nubeRealtime.ts`, `NubeRealtime.kt`,
   `CambiosNube.kt`, `SincronizacionPeriodica.kt`) y el caché de token se
   pueden traer de esa rama casi sin cambios.

> **No descartar el cliente realtime en Rust** (rama
> `claude/realtime-rust-spike`, crate `lattis-realtime`). Decisión del
> dueño (2026-09-27): primero se prueba el núcleo rediseñado (N1-N3) con
> los clientes oficiales; si el realtime sigue fallando, la causa no era
> el cliente y se compara contra esa rama antes de decidir. No borrar la
> rama ni su código hasta esa prueba.

## Verificación (antes de cada push)

```bash
# Núcleo (raíz)
cargo fmt --check
cargo clippy --all-targets
cargo test
cargo clippy --all-targets --features "nube,cifrado-secreto-dispositivo"
cargo test --features "nube,cifrado-secreto-dispositivo"

# Núcleo móvil
cd mobile/rust-core && cargo clippy --all-targets && cargo test && cd -

# Escritorio (Rust)
cd desktop/src-tauri && cargo clippy --all-targets && cd -   # los lints ya son deny en Cargo.toml; CI compila para Windows

# Android (llama al Rust real)
export ANDROID_HOME=/opt/android-sdk
cd mobile/android && ./gradlew testDebugUnitTest && cd -
```

El número de tests nunca debe bajar. Si cambia la API UniFFI, regenerar
los bindings Kotlin y commitearlos junto al cambio.
