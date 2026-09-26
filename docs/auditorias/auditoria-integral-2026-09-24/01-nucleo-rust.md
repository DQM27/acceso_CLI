# Auditoría 01 — Núcleo Rust local (`control_acceso`, excepto `src/nube/`)

Fecha: 2026-09-24 · Commit auditado: `372d93d` · Modalidad: análisis estático (sin compilar ni correr pruebas), sólo lectura.
Alcance: `src/{domain,models,services,application (sin nube.rs),database,historial,lenguaje_comandos}`, `mensajes.rs`, `texto.rs`, `tiempo.rs`, `instancia.rs`, `lib.rs`, `build.rs`, `Cargo.toml`, `tests/`. Se verificaron los consumidores en `desktop/src-tauri` y `mobile/rust-core` antes de declarar código muerto o impacto.
Auditorías previas contrastadas: `docs/auditorias/*.md`, `docs/pendientes.md`, `docs/decisiones-tecnicas.md`.

## Resumen ejecutivo

| Severidad | Cantidad |
|---|---|
| Crítica | 0 |
| Alta | 1 |
| Media | 10 |
| Baja | 9 |
| Info | 1 |
| **Total** | **21** |

**Top 5**
1. **[NR-01] (Alta)** La base local de Android queda **sin cifrar**: `mobile/rust-core` abre con `AppCore::abrir_con_reloj` (sin clave) aunque el motor enlazado sea SQLCipher/SQLite3MC. La documentación afirma lo contrario, y en esa base quedan guardados los hashes Argon2 de las contraseñas de Supabase que se cachean para el login sin conexión.
2. **[NR-02] (Media)** Siete migraciones que recrean tablas con `foreign_keys = OFF` corren `PRAGMA foreign_key_check` **después** del `COMMIT`. Si el chequeo falla, la base ya quedó migrada, con referencias rotas y `user_version` subido, y en el arranque siguiente nadie vuelve a revisar.
3. **[NR-05]/[NR-07] (Media)** La autorización por rol está apagada (`puede()` siempre devuelve `true`) y siguen públicas APIs peligrosas sin uso real: fijar la contraseña de otro usuario conociendo sólo su cédula, o resetear la de ROOT sin actor ni auditoría. Además, crear usuarios y exportar datos personales no deja rastro en la auditoría.
4. **[NR-08] (Media)** El chequeo "gafete con ingreso activo" no distingue el tipo de gafete: un gafete de proveedor o de visita en uso puede darse de baja o marcarse perdido, y el préstamo KOF no valida el catálogo.
5. **[NR-10]/[NR-09] (Media)** La cédula y la placa no se normalizan en el núcleo, lo que permite duplicar a un contratista denegado cambiando el formato. Las altas por rango (gafetes/rutas) no tienen tope: bloquean la base e inundan la cola de sincronización.

---

## Hallazgos

### [NR-01] Base de datos local de Android sin cifrar pese a compilar SQLCipher/SQLite3MC (y con hashes de contraseñas globales)
- Severidad: Alta
- Categoría: Seguridad
- Ubicación: `src/application/mod.rs:108-117` (`abrir`/`abrir_con_reloj` → `open_database` sin clave), `src/application/mod.rs:119-122` (doc que afirma "en Android, del Keystore"), `src/database/connection.rs:96-98`, consumidor `mobile/rust-core/src/lib.rs:1189` (`AppCore::abrir_con_reloj`), `mobile/rust-core/src/lib.rs:2628` (conexión secundaria con `None`), `src/application/autenticacion.rs:275-285` (`cachear_password_local`), `docs/credenciales.md:58-64`.
- Estado: Nuevo. Las auditorías previas cubrieron el secreto de dispositivo en Android, no la base de datos.
- Descripción y evidencia: el núcleo expone en producción una apertura sin clave aunque el motor sea de cifrado real. Móvil usa esa apertura:
  ```rust
  // mobile/rust-core/src/lib.rs:1189
  let core = AppCore::abrir_con_reloj(&ruta_base_datos, Arc::new(RelojCorregido::nuevo()))
  ```
  `release.yml:182` compila Android con `cifrado-sqlite3mc`, pero sin `PRAGMA cipher`/`PRAGMA key` SQLite3MC deja el archivo en claro (lo dice el propio comentario en `connection.rs:119-123`). `docs/credenciales.md` afirma que "la clave de la base SQLCipher ... queda protegida por ... Android Keystore (en móvil)", y el doc-comment de `abrir_con_reloj_cifrado` dice lo mismo. Nada en el código lo respalda: `grep` de `cifrad|PRAGMA key` en `mobile/rust-core/src/lib.rs` no encuentra ninguna apertura cifrada.
- Escenario de impacto: una tablet del punto de acceso, desatendida, rooteada o sometida a extracción forense, entrega cédulas y nombres de todos los contratistas, el historial completo de ingresos, la cola de sincronización y, lo más grave, los hashes Argon2id de las contraseñas **de Supabase** de administradores y operadores. `cachear_password_local` las guarda tras cada login en línea (mobile `lib.rs:2761`), y el hash vencido nunca se borra: `autenticacion_service.rs:112-118` sólo lo ignora. Con esos hashes se puede montar un ataque de fuerza bruta sin conexión contra credenciales que también sirven en el panel web.
- Referencia externa: OWASP MASVS-STORAGE-1 (https://mas.owasp.org/MASVS/05-MASVS-STORAGE/), CWE-311 (https://cwe.mitre.org/data/definitions/311.html), SQLCipher API "PRAGMA key" (https://www.zetetic.net/sqlcipher/sqlcipher-api/).
- Recomendación: generar en Android una clave aleatoria de 32 bytes protegida con Keystore (el mismo patrón que `SecretoDispositivoStore.kt`) y abrir con `abrir_con_reloj_cifrado`, migrando la base en claro con `sqlcipher_export`/`ATTACH ... KEY`. En el núcleo conviene quitar `abrir`/`abrir_con_reloj` fuera de `#[cfg(test)]` o de la feature `sqlite-plano` para que no se pueda abrir sin clave por accidente. Además, purgar (`password_hash = SIN_PASSWORD_LOCAL`) los hashes cacheados vencidos y corregir `docs/credenciales.md`.

### [NR-02] `PRAGMA foreign_key_check` se ejecuta después del `COMMIT` en las migraciones con `foreign_keys = OFF`
- Severidad: Media
- Categoría: Seguridad (integridad de datos) / Mala práctica
- Ubicación: `src/database/schema.rs:595-604` (M15), `571-580` (M35), `642-651` (M39), `676-685` (M41), `718-727` (M44), `757-766` (M46), `825-834` (M49).
- Estado: Nuevo.
- Descripción y evidencia:
  ```rust
  transaction.execute_batch(MIGRACION_49)?;
  transaction.execute_batch("PRAGMA user_version = 49")?;
  transaction.commit()?;
  if connection.prepare("PRAGMA foreign_key_check")?.exists([])? {
      return Err(SchemaError::MigracionStrictReferenciasInvalidas);
  }
  ```
  Cuando la validación falla, el error se devuelve pero los cambios ya están confirmados y `user_version` ya subió. En el arranque siguiente esa migración no se vuelve a ejecutar y la base abre "sana", con filas huérfanas.
- Escenario de impacto: una base con datos inconsistentes (por ejemplo, después de una sincronización parcial) queda migrada con referencias rotas de forma permanente. El único aviso es un error en el primer arranque, que en escritorio además dispara el diálogo de "base dañada" (ver NR-03).
- Referencia externa: SQLite, procedimiento de 12 pasos: el paso 10 (`foreign_key_check`) va **antes** del paso 11 (`COMMIT`) — https://www.sqlite.org/lang_altertable.html#otheralter
- Recomendación: mover el `foreign_key_check` antes de `transaction.commit()` (dentro de la transacción) para que un fallo revierta la migración. Agregar una prueba con datos huérfanos que verifique que `user_version` no cambia.

### [NR-03] Los errores de apertura no se clasifican: una base más nueva o una migración rechazada a propósito se tratan como "corrupción"
- Severidad: Media
- Categoría: Seguridad (disponibilidad y pérdida de datos) / Mala práctica
- Ubicación: `src/database/schema.rs:198-206` (versión mayor que `SCHEMA_VERSION` → `VersionInesperadaTrasMigrar` con el texto "Error interno"), `schema.rs:780-793` (M47 falla a propósito si hay nombres duplicados), `src/application/mod.rs:59-63` (`BootstrapError` con una sola variante). Consumidor: `desktop/src-tauri/src/lib.rs:185-205,247-285`.
- Estado: Nuevo.
- Descripción y evidencia: una base con `user_version` 51 (instalador viejo sobre datos nuevos) no entra en ninguna rama `if version == N` y termina en `VersionInesperadaTrasMigrar`. El escritorio convierte cualquier `BootstrapError` en `String` y ofrece "Base de datos dañada — ¿Reconstruirla desde la nube?", avisando que "se va a perder, de forma permanente, el historial de auditoría y de incidentes". Lo mismo pasa si M47 aborta por dos empresas "Dos Pinos"/"DOS PINOS", o si el archivo está bloqueado momentáneamente (`SQLITE_BUSY`).
- Escenario de impacto: el operador de la portería vuelve a una versión anterior y la app le ofrece destruir la auditoría local y la cola pendiente por un caso que no es corrupción y que se resolvía reinstalando la versión correcta.
- Referencia externa: Rust API Guidelines C-GOOD-ERR (https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err); CWE-755 (https://cwe.mitre.org/data/definitions/755.html).
- Recomendación: agregar a `SchemaError` las variantes `BaseMasNueva { encontrada }`, `MigracionRechazada { version, motivo }` y `ClaveIncorrecta` (código `SQLITE_NOTADB` tras `PRAGMA key`), exponerlas sin aplanar en `BootstrapError`, y ofrecer la reconstrucción sólo para `IntegridadInvalida`/`NOTADB`.

### [NR-04] Nada verifica en runtime ni en CI que el motor enlazado cifre de verdad
- Severidad: Media
- Categoría: Seguridad / Pruebas
- Ubicación: `src/database/connection.rs:118-127` (`aplicar_clave`), `connection.rs:314-351` (la única prueba de cifrado real, detrás de `#[cfg(feature = "cifrado-sqlite3mc")]`), `.github/workflows/ci.yml:95-106` (el núcleo sólo se prueba con el `default` = SQLCipher; nunca con SQLite3MC, que es el motor de producción de escritorio y Android).
- Estado: Pendiente de `docs/pendientes.md:52-60` ("run() no valida en tiempo de ejecución qué motor quedó enlazado"), que sigue abierto. El hueco de pruebas es nuevo.
- Descripción y evidencia: con SQLite plano, `PRAGMA key` se ignora en silencio. Así se publicaron sin cifrar las versiones v1.5.0 a v1.5.2 (según `pendientes.md`). `aplicar_clave` no comprueba nada después de aplicar la clave, y el único test que abre un archivo cifrado y valida que el encabezado no sea `SQLite format 3\0` nunca corre en CI.
- Escenario de impacto: cualquier cambio de features (Dependabot, alias `-plano` en un release) vuelve a publicar binarios con la base en claro sin que falle ninguna prueba.
- Referencia externa: SQLCipher API (`PRAGMA cipher_version`, "PRAGMA key should generally be called as the first operation") — https://www.zetetic.net/sqlcipher/sqlcipher-api/
- Recomendación: en `aplicar_clave`, cuando `cfg(any(cifrado-sqlcipher, cifrado-sqlite3mc))`, consultar `PRAGMA cipher_version` (SQLCipher) o `PRAGMA cipher` (SQLite3MC) y fallar si viene vacío. Llevar la prueba de encabezado cifrado a los tres motores y agregar a `ci.yml` un job `cargo test-3mc` para el núcleo.

### [NR-05] Autorización por rol desactivada: el código aparenta proteger; crear usuarios y exportar datos personales no deja rastro
- Severidad: Media
- Categoría: Seguridad / Código muerto
- Ubicación: `src/domain/autorizacion.rs:27-31` (`puede()` → `true`), unos 30 sitios `if !rol.puede(..)` (por ejemplo `application/usuarios.rs:30`, `catalogos.rs:37,129,177,183,300`), `application/usuarios.rs:76-88` (`crear_usuario` sin `SqliteAuditoria`), `application/historial.rs:258-470` y `catalogos.rs:76-82` (búsquedas y exportaciones sin actor), `database/queries/usuarios.rs:39-41` y `services/usuario_service.rs:29-34` (valores por defecto que actúan como ROOT). Comentario falso en el consumidor: `mobile/rust-core/src/lib.rs:1806-1810`.
- Estado: Parcialmente pendiente. La decisión "aplanado de roles" (`docs/decisiones-tecnicas.md`, 2026-09-11) dejó el código muerto como "limpieza futura". La falta de auditoría de altas y exportaciones es nueva.
- Descripción y evidencia: la decisión supone que "quien tiene una sesión válida ya pasó el filtro del panel". Sin embargo, `AppCore::crear_usuario`, expuesto por UniFFI como `Nucleo::crear_usuario`, crea usuarios **locales** con hash permanente (`password_hash_confirmado_en = None`) que nunca pasan por el panel, y no lo registra en `auditoria_cambios`. El doc de móvil sostiene que "Rust ya rechaza a un actor sin `Operacion::GestionarUsuarios`", lo cual es falso. Las exportaciones masivas de historial (cédulas y nombres) no reciben actor, así que no se puede saber quién exportó qué.
- Escenario de impacto: un operador (o una pantalla futura) crea una cuenta local persistente que sigue funcionando sin conexión aunque el panel la revoque. Ante una fuga de un XLSX no hay forma de identificar quién lo generó.
- Referencia externa: OWASP ASVS 4.0.3 V4.1.3 (mínimo privilegio) y V7.1/V7.2 (registrar eventos de seguridad) — https://github.com/OWASP/ASVS/blob/v4.0.3/4.0/en/0x12-V4-Access-Control.md, https://github.com/OWASP/ASVS/blob/v4.0.3/4.0/en/0x15-V7-Error-Logging.md
- Recomendación: o se elimina `Operacion`/`puede()` y sus sitios de uso (para que ningún comentario sugiera un control inexistente), o se vuelve a activar un gate mínimo para `GestionarUsuarios`. Auditar `crear_usuario`, el inicio y fin de sesión y cada exportación (actor, rango, filas). Quitar los valores por defecto "como ROOT" de `UsuariosQuery::buscar`/`buscar_para_tabla`.

### [NR-06] Login local sin límite de intentos, con enumeración de usuarios y sin registro de fallos
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `src/services/autenticacion_service.rs:82-129` (salida temprana sin Argon2 si la cédula no existe; `UsuarioInactivo`/`SinPasswordLocal` antes de verificar la contraseña), `src/mensajes.rs:31-46` (mensajes distintos), `src/application/autenticacion.rs:219-226`.
- Estado: La parte de tiempo es un pendiente abierto (`docs/pendientes.md:78-80`, "Mitigar timing attack en login local", sin cambios). La falta de límite de intentos y de registro de fallos, y los mensajes distintos, son nuevos.
- Descripción y evidencia: "Credenciales inválidas", "Usuario inactivo" y "Todavía no tenés contraseña en este dispositivo" revelan si la cédula existe y en qué estado está. No hay contador de intentos (búsqueda de `intentos|lockout|throttl` sin resultados en el núcleo, escritorio y móvil) ni ninguna escritura en auditoría ante un fallo.
- Escenario de impacto: desde la pantalla del punto de acceso se puede probar sin límite contra la cuenta ROOT (hash local permanente) o contra un hash cacheado de 24 h; cada intento cuesta alrededor de 50 ms de Argon2 y no queda registro.
- Referencia externa: OWASP Authentication Cheat Sheet (mensajes genéricos, tiempos uniformes, bloqueo por cuenta) — https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html; ASVS V2.2.1.
- Recomendación: devolver el mismo error y mensaje para inexistente, inactivo o contraseña incorrecta. Verificar contra un hash ficticio cuando no hay usuario. Aplicar un retardo exponencial o bloqueo por cédula (persistido) y registrar los fallos.

### [NR-07] APIs públicas sin uso real, con semántica insegura (no exigen actor ni la contraseña actual, o no auditan)
- Severidad: Media
- Categoría: Seguridad / Código muerto
- Ubicación: `src/application/autenticacion.rs:293-315` (`fijar_password_inicial`: fija la contraseña de cualquier usuario `SIN_PASSWORD_LOCAL` conociendo **sólo la cédula**, sin auditoría), `src/application/usuarios.rs:56-74` (`resetear_password_root`: sin actor, sin auditoría; el comentario remite a un `main.rs --reset-root` que ya no existe), `usuarios.rs:290-336` (`preparar_cambio_password_propio` entrega el `password_hash` al llamador y `cambiar_mi_password_con_hash` acepta el cambio si el llamador devuelve ese mismo hash, sin exigir la contraseña actual), `usuarios.rs:102-124,215-236` (`*_con_hash`: aceptan un hash arbitrario y se saltan la política de largo mínimo).
- Estado: Nuevo.
- Descripción y evidencia: un `grep` en `desktop/src-tauri/src` y `mobile/rust-core/src` no encuentra llamadas a ninguna de ellas (escritorio sólo usa `cambiar_mi_password`). Sólo las usan las pruebas, y `tests/bootstrap_password_usuario_global.rs:102` documenta falsamente que es el "camino real que usan escritorio/móvil".
- Escenario de impacto: cualquier comando nuevo de Tauri o UniFFI que las exponga (o una revisión que confíe en su nombre) abre la toma de cuentas de usuarios globales por cédula (dato semipúblico en Costa Rica) o el reseteo de ROOT sin rastro.
- Referencia externa: OWASP ASVS V2.5 (recuperación de credenciales) y V7.2.1 — https://github.com/OWASP/ASVS/blob/v4.0.3/4.0/en/0x11-V2-Authentication.md; CWE-620 (https://cwe.mitre.org/data/definitions/620.html).
- Recomendación: eliminarlas (el código está en la rama `archive/cli-tui-2026-09-12`) o marcarlas `pub(crate)`/`#[cfg(test)]`. Si alguna se conserva, que exija actor y verificación fuerte, y que registre en auditoría.

### [NR-08] El chequeo "gafete con ingreso activo" no distingue el tipo de gafete; el préstamo KOF no valida el catálogo
- Severidad: Media
- Categoría: Seguridad (integridad del inventario de credenciales físicas)
- Ubicación: `src/services/gafete_service.rs:81-97` (`dar_de_baja`) y `:103-128` (`marcar_perdido`), que consultan `RegistroIngresoRepository::buscar_ingreso_activo_por_gafete(gafete.numero)` (sólo `registro_ingresos`, sin tipo); `src/services/gafete_provisional_service.rs` (`entregar`: no consulta `gafetes` de tipo `PROVISIONAL_KOF`).
- Estado: Nuevo.
- Descripción y evidencia: desde M35 la unicidad es `(numero, tipo)`, pero el guard sólo mira los ingresos de **contratistas** por número:
  ```rust
  if registros.buscar_ingreso_activo_por_gafete(gafete.numero)?.is_some() {
      return Err(GafeteServiceError::GafeteConIngresoActivo);
  }
  ```
  El gafete PROVEEDOR #7 en uso por un proveedor puede darse de baja o marcarse perdido (falso negativo), mientras que el VISITA #7 queda bloqueado si un contratista tiene el CONTRATISTA #7 (falso positivo). `entregar` de KOF presta números dados de baja, perdidos o inexistentes en el catálogo.
- Escenario de impacto: el inventario contradice lo que está pasando en el sitio. Se registra como perdido un gafete que alguien sigue usando y se prestan gafetes dados de baja.
- Referencia externa: CWE-841 (Improper Enforcement of Behavioral Workflow) — https://cwe.mitre.org/data/definitions/841.html
- Recomendación: que la verificación de ocupación dependa de `gafete.tipo` (registro_ingresos, movimientos_visita, registro_ingresos_proveedor o prestamos_gafete_provisional). En `entregar`, validar con `validar_para_asignar(buscar_por_numero(n, ProvisionalKof))`. Agregar pruebas por tipo: hoy no hay ninguna prueba de `GafeteConIngresoActivo` en `tests/`.

### [NR-09] Altas por rango sin tope (gafetes y rutas): transacción `IMMEDIATE` ilimitada e inundación de `cola_salida`
- Severidad: Media
- Categoría: Seguridad (disponibilidad) / Rendimiento
- Ubicación: `src/services/gafete_service.rs:67-79`, `src/services/ruta_service.rs:259-270`, `src/application/gafetes.rs:72-89`, `application/rutas.rs:236-255`, `src/database/repositories/gafete_repository.rs:93-106` (cada alta encola una fila en `cola_salida`).
- Estado: Nuevo.
- Descripción y evidencia: sólo se valida `desde > 0 && hasta >= desde`, y `(desde..=hasta).map(crear_uno).collect()` no tiene límite. Ni el comando de Tauri (`desktop/src-tauri/src/comandos/gafetes.rs:52-63`) ni el núcleo ponen un máximo.
- Escenario de impacto: un error de tipeo (1 a 1 000 000) retiene el candado de escritura y el `Mutex<AppCore>` durante minutos, hace crecer el WAL, carga un millón de filas en memoria y un millón de envíos a Supabase.
- Referencia externa: CWE-770 (Allocation of Resources Without Limits) — https://cwe.mitre.org/data/definitions/770.html
- Recomendación: definir una constante `MAX_RANGO_ALTA` (por ejemplo 500) en el servicio, devolver `RangoDemasiadoGrande` si se supera y agregar una prueba.

### [NR-10] El núcleo no valida ni normaliza cédulas ni placas (sólo `trim`), aunque el frontend dice que "la validación real vive en el núcleo"
- Severidad: Media
- Categoría: Seguridad (lógica de control de acceso)
- Ubicación: `src/services/contratista_service.rs` (`construir_contratista`/`construir_actualizacion`: `datos.cedula.trim()`), `services/usuario_service.rs:420-429` (`normalizar_requerido`), `services/ingreso_proveedor_service.rs:62-65`, `services/ruta_service.rs:100-130` (placa), `services/registro_ingreso_service.rs:258-266` (placa sin normalizar ni largo máximo); esquema `cedula TEXT NOT NULL UNIQUE` (igualdad exacta). Contraste: `desktop/src/validacion.ts:5-8`.
- Estado: Nuevo. Para nombres de empresa ya se corrigió con M47 (`PLEGAR`), pero no para cédulas ni placas.
- Descripción y evidencia: `"1-1234-0567"`, `"112340567"` y `"0112340567"` cuentan como tres contratistas distintos (el último incluso pasa la regex `^\d+$` del frontend). Móvil y la sincronización no pasan por esa regex. Con las placas pasa lo mismo: `buscar_activa_por_placa` compara el texto exacto, así que `"C12345"` y `"c-12345"` evaden `VehiculoYaEnRuta`.
- Escenario de impacto: a un contratista denegado (`tiene_acceso = 0` o PRAIND vencido) se lo vuelve a dar de alta con otro formato de cédula y se le registra el ingreso. Un mismo vehículo abre dos salidas de ruta.
- Referencia externa: CWE-1289 (Improper Validation of Unsafe Equivalence in Input) — https://cwe.mitre.org/data/definitions/1289.html; OWASP Input Validation Cheat Sheet — https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html
- Recomendación: crear un tipo `Cedula` (y `Placa`) con constructor que normalice (quitar separadores y mayúsculas, formato canónico, largo máximo) y usarlo en todos los servicios y en la sincronización. Agregar un índice único sobre el valor normalizado, con una migración que detecte duplicados existentes.

### [NR-11] La exportación a PDF trunca en silencio los movimientos locales a 20 000 y mezcla sin tope los remotos
- Severidad: Media
- Categoría: Mala práctica (error tragado / integridad de reportes)
- Ubicación: `src/application/historial.rs:217-236` (`movimientos_completos_con_conexion`: `buscar_historial_completo_con_conexion(..)?.items` descarta `truncado`); consumidor `desktop/src-tauri/src/comandos/historial.rs:295-300`.
- Estado: Nuevo. R-01 de la auditoría de rendimiento introdujo `CargaCompleta::truncado` precisamente para no recortar en silencio, pero este camino lo ignora.
- Descripción y evidencia:
  ```rust
  let locales = buscar_historial_completo_con_conexion(connection, filtro)?.items;
  ...
  movimientos.extend(movimientos_del_sitio_con_conexion(connection, filtro, &uuids_locales)?);
  ```
- Escenario de impacto: un PDF de "historial del rango X" entregado a seguridad o a un cliente omite los movimientos locales más viejos del rango (mantiene los 20 000 más recientes) e incluye todos los remotos, sin ningún aviso. Es un reporte formalmente incorrecto.
- Referencia externa: CWE-392 (Missing Report of Error Condition) — https://cwe.mitre.org/data/definitions/392.html
- Recomendación: devolver `CargaCompleta<MovimientoExportable>` (o un error `DemasiadasFilasPdf`), aplicar el mismo tope a los remotos y mostrarlo en el PDF y en la interfaz.

### [NR-12] Guardia de "reloj retrocedido" inconsistente entre dominios, repetida en tres copias y con una variante muerta
- Severidad: Baja
- Categoría: Mala práctica / Seguridad (integridad temporal)
- Ubicación: `src/application/accesos.rs:65-82,135-147` (sólo mira `registro_ingresos`), `application/citas.rs:48-65,128-159` (ingresos y visitas), `application/rutas.rs:300-318,384-420` (ingresos, visitas y rutas), `application/proveedores.rs:135-195` y `gafetes_provisionales.rs:21-72` (**sin guardia**), `services/error.rs:378` (`IngresoProveedorServiceError::RelojRetrocedido`, nunca se construye; `mensajes.rs:374` la traduce igual).
- Estado: Nuevo.
- Descripción y evidencia: la misma regla ("el reloj no retrocedió respecto al último movimiento") se escribe tres veces con alcances distintos y falta en dos dominios. Además, una sola marca futura (reloj adelantado, o un `RelojCorregido` con un desfase sin cota en `tiempo.rs:70-78`) bloquea todos los ingresos hasta que la hora real la alcanza, y no hay forma de recuperarse.
- Escenario de impacto: con el reloj atrasado se pueden registrar préstamos KOF e ingresos de proveedores con hora anterior a la real. Un reloj que estuvo adelantado deja la portería sin poder registrar movimientos.
- Referencia externa: Rust API Guidelines, genéricos para evitar duplicación — https://rust-lang.github.io/api-guidelines/flexibility.html
- Recomendación: un único `fn en_transaccion_operativa<T, E: From<DatabaseError> + From<ErrorOperativo>>` que calcule el máximo de todas las tablas de movimientos, usado por los cinco dominios. Acotar el desfase aceptado en `RelojCorregido` y definir un procedimiento de recuperación (auditado) para marcas futuras.

### [NR-13] `SalidaRegistroIngreso` promete "fecha y usuario juntos", pero el esquema y la sincronización permiten `usuario_salida_id` NULL
- Severidad: Baja
- Categoría: Mala práctica (invariante del tipo que no se cumple)
- Ubicación: `src/models/registro_ingreso.rs:61-69`, `src/database/repositories/registro_ingreso_repository.rs:110-120` (`zip` descarta la salida si el id es NULL), `registro_ingreso_proveedor_repository.rs:~90-95` (mismo patrón), esquema M44/M49 (`CHECK` relajado), `src/nube/sincronizacion.rs:1858-1863` (`usuario_salida_id = NULL`).
- Estado: Nuevo.
- Descripción y evidencia: un ingreso cerrado desde otro dispositivo se lee con `salida: None` ("activo"). Hoy no genera un error visible porque el `UPDATE ... WHERE fecha_hora_salida IS NULL` lo frena, pero el modelo de dominio queda mintiendo y el comentario ("el CHECK garantiza que vienen juntos") está desactualizado.
- Escenario de impacto: cualquier consumidor futuro de `RegistroIngreso.salida` (reportes, reglas) tratará como abiertos ingresos ya cerrados.
- Referencia externa: Rust API Guidelines C-NEWTYPE / hacer imposibles los estados inválidos — https://rust-lang.github.io/api-guidelines/type-safety.html
- Recomendación: `SalidaRegistroIngreso { fecha_hora, usuario: UsuarioSalida }`, con `enum UsuarioSalida { Local(i64), Remoto(String) }`, leyendo `usuario_salida_nombre`.

### [NR-14] Rendimiento: `quick_check` completo dos veces en cada apertura y paginación `OFFSET` + `COUNT(*)` por página en las cargas completas
- Severidad: Baja
- Categoría: Rendimiento
- Ubicación: `src/database/connection.rs:137-138` (`verificar_archivo_propio` y luego `initialize_database`, que repite `rechazar_archivo_ajeno` y `verificar_integridad_rapida` en `schema.rs:105-106`); `src/database/queries/ingresos/historial.rs:139-189` (cada página ejecuta `COUNT(*)` y `LIMIT 200 OFFSET n`), llamada en bucle por `application/historial.rs:29-53,184-199,366-395`.
- Estado: Nuevo. La auditoría de rendimiento (R-01) midió unos 33 s para exportar 100 000 filas, pero no identificó esta causa.
- Descripción y evidencia: `PRAGMA quick_check` es O(N) sobre todo el archivo (con descifrado de cada página en SQLCipher) y se ejecuta dos veces al arrancar. Una exportación de N filas hace N/200 `COUNT(*)` y `OFFSET` crecientes, es decir O(N²) filas recorridas.
- Escenario de impacto: con varios años de historial, el arranque en frío y la exportación crecen de forma cuadrática en tablets y PCs modestas.
- Referencia externa: SQLite `PRAGMA quick_check` ("O(N)") — https://www.sqlite.org/pragma.html#pragma_quick_check; paginación por clave (keyset) — https://use-the-index-luke.com/no-offset
- Recomendación: ejecutar el chequeo una sola vez (un parámetro `ya_verificado` o separar `initialize_database` en dos partes). En los bucles internos, paginar por clave (`(fecha_hora_ingreso, id) < (?, ?)`), que `corte_id` ya permite, y calcular el `COUNT` una única vez.

### [NR-15] Copias de la clave de cifrado en memoria que nunca se borran
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `src/database/connection.rs:126` y `225-233` (`clave_a_hex` → `String`; `pragma_update` arma otro `String` SQL con la clave); no se activa `PRAGMA cipher_memory_security`.
- Estado: Nuevo. Escritorio guarda la clave en `Zeroizing<[u8;32]>`, pero el núcleo la copia sin borrarla.
- Descripción y evidencia: `format!("x'{}'", clave_a_hex(clave))` deja la clave en hexadecimal en memoria del heap ya liberada.
- Escenario de impacto: un volcado de memoria o un archivo de paginación de la PC del punto de acceso puede contener la clave de la base.
- Referencia externa: CWE-316 — https://cwe.mitre.org/data/definitions/316.html; SQLCipher `cipher_memory_security` — https://www.zetetic.net/sqlcipher/sqlcipher-api/; crate `zeroize` (1.9.0 ya está en `Cargo.lock`).
- Recomendación: construir el literal en un `Zeroizing<String>` y ejecutar `execute_batch` con él. Evaluar `PRAGMA cipher_memory_security = ON` en escritorio.

### [NR-16] `LIKE` sin escapar `%`/`_` y `modo` como número mágico
- Severidad: Baja
- Categoría: Mala práctica
- Ubicación: `src/database/search.rs:1-31` (`modo: i64` con valores 0, 1 y 2; `format!("%{texto}%")`), `database/queries/ingresos/historial.rs:340-342` (`patron_like`), usos en `queries/contratistas.rs:143-165`, `queries/usuarios.rs:77-127`.
- Estado: Nuevo. No es inyección SQL: los valores van como parámetros.
- Descripción y evidencia: buscar `"_"` o `"%"` devuelve todo; `"a_b"` también encuentra `"axb"`. El `match busqueda.modo { 1 => .., 2 => .., _ => {} }` acepta cualquier valor sin que el compilador lo impida.
- Escenario de impacto: búsquedas con resultados inesperados; un nuevo modo que se agregue se ignora en silencio.
- Referencia externa: SQLite `LIKE ... ESCAPE` — https://www.sqlite.org/lang_expr.html#like
- Recomendación: escapar `\`, `%` y `_` con `ESCAPE '\'`, y reemplazar `modo` por `enum ModoBusqueda { Todos, Corto(String), Fts(String) }`.

### [NR-17] Código muerto verificado en todo el repositorio
- Severidad: Baja
- Categoría: Código muerto
- Ubicación y evidencia (sin referencias en `src`, `desktop/src-tauri/src` ni `mobile/rust-core/src`, fuera de su propia definición y de las pruebas):
  - `src/lenguaje_comandos/` (unas 1 400 líneas; `lib.rs:6-10` admite que está "huérfano", el código ya vive en la rama `archive/cli-tui-2026-09-12`; los doc-comments remiten a `application/comandos.rs` y `comandos/`, que no existen; comentario suelto en `application/mod.rs:19-20`).
  - `build.rs:8` depende de `CARGO_FEATURE_TERMINAL_UI`, una feature que no existe en `Cargo.toml`: la rama nunca se ejecuta y la dependencia de build `winresource` se compila en cada build del núcleo, escritorio y móvil sin usarse.
  - Feature `dev-auth` y `services/dev_auth.rs`: sólo `tests/root_inicial.rs:411` y ningún job de CI activa la feature, así que se deteriora sin que nadie lo note.
  - `tiempo.rs:81-83,116-118` (`ahora_costa_rica`, `hora_actual_texto`), `domain/cita.rs:22` (`VERSION_REGLAS_VISITA`), `services/registro_ingreso_service.rs:398-406` (`registrar_salida_por_gafete`), `application/usuarios.rs:39-46,92-100` (`listar_roots_activos`, `validar_datos_para_crear_usuario`), variantes no auditadas `UsuarioService::{actualizar_administracion, activar, desactivar}` (`usuario_service.rs:151-229`), `UsuarioRepository::contar_roots_activos` (sólo en pruebas), `IngresoProveedorServiceError::RelojRetrocedido` (NR-12).
  - `database/connection.rs:199-206` (`abrir_conexion_secundaria`, la de sólo lectura, sin consumidores). En consecuencia, las lecturas largas de escritorio (exportar, auditoría completa) usan la conexión **de escritura** (`desktop/src-tauri/src/estado.rs:178-181`) y se pierde el refuerzo de `query_only`.
- Estado: Nuevo. La auditoría de calidad de 2026-09 decía que no había código muerto; el comentario de `profile.production` en `Cargo.toml` todavía menciona `formulario.rs`, que no existe.
- Escenario de impacto: más superficie que revisar y mantener, documentación que describe módulos inexistentes y tiempo de compilación perdido.
- Referencia externa: Rust API Guidelines, "Future proofing" — https://rust-lang.github.io/api-guidelines/future-proofing.html
- Recomendación: borrar `lenguaje_comandos`, la rama de `build.rs` y `winresource`, `dev-auth`, las funciones listadas y las variantes no auditadas. Pasar las lecturas de escritorio a `abrir_conexion_secundaria` (sólo lectura) o eliminarla. Actualizar el comentario de `profile.production`.

### [NR-18] Acoplamiento entre capas: ciclo domain↔models, modelos de lectura en la capa de persistencia y reglas duplicadas en SQL
- Severidad: Baja
- Categoría: Acoplamiento
- Ubicación: `models/contratista.rs:101-109` → `domain::contratista`, y `domain/acceso.rs:4` → `models::contratista` (ciclo); `models/registro_ingreso.rs:9` reexporta `domain::acceso`; los DTO que serializa la interfaz viven en `database::queries::*` (`ContratistaResumen`, `UsuarioResumen`, `CambioAuditado`, `FiltroHistorial`) y los importan `services`, `historial::exportacion` y los comandos de Tauri; la regla PRAIND está duplicada en SQL (`queries/contratistas.rs:195,206`, frente a `domain/contratista.rs:71-77`); un servicio fabrica un error de rusqlite (`registro_ingreso_service.rs:211-217`, `Sqlite(rusqlite::Error::InvalidQuery)` para "empresa inexistente"); la conversión de rol está duplicada (`usuario_repository.rs:52-86`, `queries/usuarios.rs:147-167`); `Contratista::reconstruir` es `pub` y anula la privacidad de `empresa_activa`.
- Estado: Parcialmente pendiente (`docs/pendientes.md`, "Agregados de dominio con constructores privados diferidos a V3"). El resto es nuevo.
- Escenario de impacto: si cambia la regla PRAIND en el dominio y no en el SQL, el filtro "vencido/próximo a vencer" de la grilla contradice la decisión real de acceso. Los errores de dominio aparecen como "Error de SQLite".
- Referencia externa: Rust API Guidelines C-GOOD-ERR — https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err
- Recomendación: reunir `models` y `domain` en entidades con constructores validados; mover filtros y resúmenes a `application::dto`; generar el SQL de PRAIND desde una constante del dominio o probar su equivalencia con una prueba basada en propiedades; agregar `RegistroIngresoServiceError::EmpresaNoEncontrada`; crear `RolUsuario::as_str_sql`/`from_str_sql`.

### [NR-19] Migraciones: 35 funciones casi idénticas y documentación de atomicidad inexacta
- Severidad: Baja
- Categoría: Mala práctica
- Ubicación: `src/database/schema.rs:86-405` (cadenas de `if version == N`), `407-855` (`aplicar_migracion_16..50`, mismo cuerpo de 5 líneas); `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md:167-174` y `docs/pendientes.md:639-641` afirman que una migración fallida "revierte sola ... todas", pero `tests/migraciones.rs:1316` sólo cubre 1 a 14 (una transacción común). Desde la 15, cada migración confirma por separado.
- Estado: Nuevo.
- Escenario de impacto: al agregar una migración es fácil olvidarse del `if` o de la función (es justamente el error que previene `VersionInesperadaTrasMigrar`). La documentación da una garantía más fuerte que la real.
- Referencia externa: SQLite `PRAGMA user_version` — https://www.sqlite.org/pragma.html#pragma_user_version
- Recomendación: una tabla `const MIGRACIONES: &[(i64, &str, RequiereFkOff)]` recorrida en un solo bucle, con el `foreign_key_check` dentro de la transacción (NR-02). Corregir la documentación ("cada migración es atómica y reanudable").

### [NR-20] Huecos de cobertura en reglas críticas
- Severidad: Baja
- Categoría: Pruebas
- Ubicación y evidencia (`grep` en `tests/` y en los `mod tests`):
  - Ninguna prueba ejecutada en CI verifica que la base quede cifrada con SQLCipher; la de SQLite3MC nunca corre (NR-04).
  - `GafeteConIngresoActivo`: 0 pruebas en `tests/`; ninguna combina tipos de gafete (NR-08).
  - `crear_gafetes_rango`, `entregar_gafete_provisional`, `registrar_ingreso_proveedor` y `marcar_gafete_perdido_*` no tienen pruebas de integración en `AppCore` (sólo unitarias de servicio).
  - `VersionInesperadaTrasMigrar`, `MigracionStrictReferenciasInvalidas` e `IntegridadInvalida`: 0 pruebas.
  - `cachear_password_local` (escritura del hash cacheado con marca de tiempo): 0 pruebas directas.
  - `#[cfg(feature = "dev-auth")]`: pruebas que ningún job ejecuta.
  - Documentación de prueba falsa: `tests/bootstrap_password_usuario_global.rs:102` ("camino real que usan escritorio/móvil").
- Estado: Nuevo.
- Escenario de impacto: las regresiones en cifrado, inventario de gafetes y migraciones llegarían a producción sin que CI las detecte.
- Referencia externa: OWASP ASVS V1.14 / V14.2 (verificación de configuración de build) — https://github.com/OWASP/ASVS/blob/v4.0.3/4.0/en/0x22-V14-Config.md
- Recomendación: agregar las pruebas listadas, un job `test-3mc` del núcleo en CI y activar o eliminar `dev-auth`.

### [NR-21] `unreachable!` en rutas de producción para invariantes que el tipo no expresa
- Severidad: Info
- Categoría: Mala práctica
- Ubicación: `src/mensajes.rs:185-187` y `218-220` (`GafeteNoDisponible(EstadoGafete::Disponible) => unreachable!`), `services/registro_ingreso_service.rs:330`, `database/schema.rs:83`.
- Estado: Nuevo. La auditoría de calidad sólo contabilizó `unwrap`/`expect`.
- Descripción y evidencia: hoy son lógicamente inalcanzables (validado). Con `panic = "abort"` en `profile.production`, cualquier cambio futuro que construya esa variante cerraría la app de golpe en lugar de mostrar un mensaje.
- Referencia externa: Rust API Guidelines, tipos para los invariantes — https://rust-lang.github.io/api-guidelines/type-safety.html
- Recomendación: `enum EstadoNoAsignable { Perdido, DeBaja }` para `NoDisponible`, y `let ResultadoAcceso::... else` antes de construir `resultado_registrado`.

---

## Aspectos bien resueltos

- **Sin inyección SQL:** todo `format!` en SQL arma fragmentos estáticos y los valores van como parámetros. En FTS5 se escapan las comillas dobles de cada palabra (`search.rs:44-48`).
- **Exportación XLSX sin inyección de fórmulas:** toda celda de texto usa `write_string_with_format`, que produce celdas de texto y no fórmulas. El destino se publica con `persist_noclobber`, sin carrera entre comprobar y escribir ni sobrescritura de archivos.
- **Transacciones `IMMEDIATE` con revalidación del actor y de las reglas dentro de la misma transacción** en ingresos, salidas, gafetes y usuarios; carrera del "último ROOT" cerrada en el repositorio, con prueba.
- **Historial inmutable reforzado en la base:** triggers de no borrado, entrada inmutable y salida única, `CHECK` de formato UTC, índices únicos parciales para evitar dos ingresos activos o el mismo gafete dos veces.
- **Argon2id con parámetros por defecto de `argon2` 0.6** (m=19 MiB, t=2, p=1, mínimo recomendado por OWASP) y verificación mediante la API de `password-hash`; `Debug` de `CandidatoAutenticacion` oculta el hash.
- **Prácticamente sin `unwrap`/`expect` en producción** dentro del alcance (sólo los 4 `unreachable!` de NR-21); conversiones numéricas con `try_from` y lints estrictos de clippy.
- **Pragmas de endurecimiento** (`trusted_schema=OFF`, `secure_delete=FAST`, `foreign_keys=ON`) centralizados, también en las conexiones secundarias; `PLEGAR` marcada `SQLITE_INNOCUOUS` a conciencia.
- **Dependencias al día:** `rustls 0.23.45` ya incluye la corrección de RUSTSEC-2026-0285; no hay avisos abiertos conocidos para `rusqlite 0.40.2` (https://rustsec.org/packages/rusqlite.html), y `cargo audit` corre en CI.
