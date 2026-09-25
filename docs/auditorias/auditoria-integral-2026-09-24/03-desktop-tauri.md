# Auditoría 03 — Capa nativa de escritorio (Tauri v2)

Alcance: `desktop/src-tauri/` completo (lib.rs, main.rs, estado.rs, clave_cifrado.rs, recuperacion_local.rs, comandos/*, dto/*, pdf/*, Cargo.toml/lock, tauri.conf.json, capabilities, build.rs, .cargo/), `packaging/`, `.github/workflows/release.yml` y `build-test-desktop.yml`. Análisis estático y grep, sin compilar. Fecha: 2026-09-24.

Contra auditorías previas (`docs/auditorias/*.md`): ninguna de ellas cubre la capa de comandos Tauri desde el punto de vista de seguridad; los puntos 5.1–5.4 del plan QA (logs/Sentry) están implementados. Todos los hallazgos de abajo son **Nuevos**, salvo que se indique otra cosa.

## Resumen ejecutivo

| Severidad | Cantidad |
|---|---|
| Crítica | 0 |
| Alta | 2 |
| Media | 8 |
| Baja | 9 |
| Info | 3 |

**Los 5 principales:**
1. **DT-01 (Alta)**: 76 de los 85 comandos son síncronos. En Tauri v2 eso significa que corren en el **hilo principal (UI)**. Ahí ocurren consultas grandes, exportación XLSX (~33 s medidos), Argon2 y llamadas HTTP. Los comentarios del código afirman lo contrario.
2. **DT-02 (Alta)**: los comandos de exportación reciben del frontend una ruta arbitraria. Eso permite escribir, renombrar o sobrescribir cualquier archivo del usuario (incluido `db_key.dat`) con contenido controlado.
3. **DT-03 (Media)**: `tauri-plugin-updater 2.11.0` es vulnerable a CVE-2026-95624 / GHSA-rjc6-5hfg-grp9 (downgrade desde JS). `cargo audit` no lo detecta.
4. **DT-04 (Media)**: la sesión de Supabase en memoria no está ligada al usuario local. Una carrera entre la renovación en segundo plano y el cierre de sesión, sumada a un login local, puede hacer que `cambiar_password_supabase` cambie la contraseña de **otro** usuario.
5. **DT-06 (Media)**: la recuperación de una base dañada ignora `-wal`/`-shm` (la base está en modo WAL). El respaldo queda incompleto y el WAL viejo queda emparejado con la base nueva.

---

### [DT-01] Comandos síncronos ejecutados en el hilo principal (UI) de Tauri
- Severidad: Alta
- Categoría: Rendimiento / Mala práctica
- Ubicación: `desktop/src-tauri/src/comandos/*.rs` (76 comandos `#[tauri::command] pub fn` sin `async`); comentarios erróneos en `estado.rs:169` y `comandos/historial.rs:289`.
- Estado: Nuevo
- Descripción y evidencia: según la documentación oficial, *"Commands without the async keyword are executed on the main thread unless defined with #[tauri::command(async)]"*. En WebView2 el hilo principal es el de la ventana. Entre los comandos síncronos están:
  - `exportar_historial`: XLSX de ~33 s con 100 k filas, según su propio comentario.
  - `exportar_tabla_xlsx` y `guardar_csv`.
  - `listar_historial` (hasta 20 000 filas) y `listar_auditoria` (~750 ms, según su comentario).
  - `cambiar_mi_password`: dos pasadas de Argon2.
  - `registrar_ingreso`, `registrar_entrada_visita`, `entregar_gafete_provisional` y `registrar_ingreso_proveedor`: hacen HTTP a Supabase (`gafete_libre_en_otro_dispositivo`, `proveedor_activo_en_otro_sitio`; `comandos/ingresos.rs:148`, `comandos/proveedores.rs:164`). El último hace dos llamadas y no tiene el tope `tokio::time::timeout` que sí usan los comandos async.
  - `cerrar_ingreso_remoto`, `cerrar_ingreso_proveedor_remoto` y `cerrar_prestamo_gafete_provisional_remoto`: autenticación y cierre, ambos por red.

  Los comentarios `estado.rs:169` ("el comando ya corre en el pool de hilos bloqueantes de Tauri (no congela la ventana)") y `historial.rs:289` ("al ser un comando NO async ya corre por su cuenta en el pool de hilos bloqueantes") son incorrectos.
- Escenario de impacto: en la portería, con red lenta, pulsar "Registrar" con gafete congela la ventana hasta que vence el timeout HTTP. Una exportación de Historial grande deja la app "No responde" durante decenas de segundos y Windows puede ofrecer cerrarla, con pérdida de la operación en curso. Mientras tanto no se puede registrar ningún ingreso ni salida.
- Referencia externa: https://v2.tauri.app/develop/calling-rust/ (sección "Async Commands").
- Recomendación: marcar todos los comandos con `#[tauri::command(async)]`, o convertirlos en `async fn` y envolver el trabajo con `tauri::async_runtime::spawn_blocking`, como ya hacen `sincronizar_con_nube` y `configurar_dispositivo_inicial`. Corregir los dos comentarios. Añadir una regla de lint o revisión: ningún comando nuevo sin `async`.

### [DT-02] Escritura, renombrado y sobrescritura de archivos en rutas arbitrarias controladas por el webview
- Severidad: Alta
- Categoría: Seguridad
- Ubicación:
  - `comandos/exportacion.rs:30-36` (`guardar_csv`: `std::fs::write(&destino, con_bom(&contenido))`).
  - `comandos/exportacion.rs:46` (`exportar_tabla_xlsx`).
  - `comandos/historial.rs` (`exportar_historial` y `exportar_historial_pdf`).
  - `comandos/exportacion.rs` (`exportar_tabla_pdf`).
  - `comandos/historial.rs:214-238` (`RespaldoDestino::apartar`: `rename` del archivo existente y `remove_file` al confirmar).
- Estado: Nuevo
- Descripción y evidencia: `destino: String` llega tal cual desde `invoke()`. El backend no valida la extensión, ni el directorio, ni que la ruta provenga del diálogo "Guardar como". El diálogo (`dialog:allow-save`) solo sugiere la ruta; el comando confía en el frontend. `guardar_csv` escribe contenido 100 % controlado por el llamador, con solo un BOM al inicio. Los demás comandos apartan el archivo existente y lo borran si la exportación termina bien. Es CWE-73 / CWE-22.
- Escenario de impacto: basta cualquier ejecución de JS en el webview principal, por ejemplo por una dependencia npm comprometida dentro del bundle (AG Grid, FullCalendar, supabase-js…).
  - `invoke('guardar_csv',{destino:'%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\x.bat', contenido:'\r\ncalc'})` deja persistencia con ejecución de código al próximo inicio de sesión (el BOM solo invalida la primera línea).
  - `exportar_tabla_xlsx` con `destino = %APPDATA%\com.dqm27.lattis.desktop\db_key.dat` sustituye la clave DPAPI de la base por un XLSX. La base queda irrecuperable y se fuerza la "reconstrucción desde la nube", que destruye la auditoría local.
- Referencia externa: https://v2.tauri.app/security/ (no confiar en el frontend; los comandos son la frontera de confianza) · https://cwe.mitre.org/data/definitions/73.html · OWASP ASVS 5.0 V5.3 (File Storage).
- Recomendación: abrir el diálogo desde Rust dentro del propio comando (`app.dialog().file().blocking_save_file()` dentro de `spawn_blocking`) y no aceptar rutas del frontend; después, quitar `dialog:allow-save` de la capability. Como mínimo:
  - exigir la extensión esperada (`.csv`/`.xlsx`/`.pdf`);
  - canonicalizar el directorio padre y rechazar el árbol de datos de la app (`%APPDATA%`/`%LOCALAPPDATA%\com.dqm27.lattis.desktop`) y la carpeta Inicio;
  - no hacer `rename`/`remove` sobre archivos que no tengan esa extensión.

### [DT-03] tauri-plugin-updater 2.11.0 vulnerable a downgrade (CVE-2026-95624)
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `desktop/src-tauri/Cargo.lock` (`tauri-plugin-updater 2.11.0`), `Cargo.toml` (`tauri-plugin-updater = "2"`), `capabilities/default.json` (`"updater:default"`), `desktop/package.json` (`@tauri-apps/plugin-updater ^2.11.0`).
- Estado: Nuevo (advisory publicado el 2026-09-22)
- Descripción y evidencia: el comando IPC `check` acepta `allowDowngrades` desde JS. `updater:default` concede `allow-check` al webview `main`. Versiones afectadas: >= 2.8.0 y < 2.12.0; corregido en `updater-v2.12.0`. OSV no lo asocia a la versión de crates.io, así que `cargo audit` (job `test-gui`) no lo detecta.
- Escenario de impacto: un XSS o una dependencia comprometida invoca `check({allowDowngrades:true})` y después `downloadAndInstall()`. Así se instala una versión anterior firmada con bugs ya corregidos, por ejemplo builds que según el propio `Cargo.toml` salieron sin cifrado real (v1.5.0–v1.5.2).
- Referencia externa: https://nvd.nist.gov/vuln/detail/CVE-2026-95624 · https://github.com/tauri-apps/tauri/security/advisories/GHSA-rjc6-5hfg-grp9 · https://github.com/tauri-apps/plugins-workspace/releases/tag/updater-v2.12.0
- Recomendación: `cargo update -p tauri-plugin-updater` a >= 2.12.0 y subir `@tauri-apps/plugin-updater`. Mejor aún: mover el check/instalación a Rust (`app.updater()`) y dejar al frontend solo un comando propio "buscar/instalar actualización", retirando `updater:default` de la capability.

### [DT-04] La sesión de Supabase en memoria no está ligada al usuario local (cambio de contraseña sobre la cuenta equivocada)
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `estado.rs:191` (`iniciar_sesion` no limpia `sesion_supabase`), `estado.rs:209`, `comandos/nube.rs:243-246` (renovación en segundo plano sin compare-and-set), `comandos/autenticacion.rs:213` (login local), `comandos/autenticacion.rs:343-351` (`cambiar_password_supabase` usa el `access_token` guardado junto con la `cedula` de la sesión local). El núcleo: `src/nube/auth_supabase.rs:167-193` valida la contraseña con un `login()` cuyo token descarta, y hace `PUT /auth/v1/user` con el token recibido.
- Estado: Nuevo
- Descripción y evidencia: `SesionSupabaseCacheada` no guarda a qué usuario pertenece. `intentar_sincronizacion` lee el `refresh_token` del usuario A, hace la llamada de red y después hace `iniciar_sesion_supabase(sesion)` sin comprobar que la sesión siga siendo la misma. Si A cierra sesión durante esa ventana, la sesión de A se reinstala. Si luego B entra por el camino local (hash cacheado offline), queda `sesion = B` y `sesion_supabase = A`. `cambiar_password_supabase` valida entonces la contraseña actual de **B** y cambia la contraseña de **A**.
- Escenario de impacto: en el cambio de turno de la portería, un operador termina fijando la contraseña de la cuenta de un administrador (toma de cuenta y bloqueo del titular). La probabilidad es baja (depende de una carrera con la sincronización, que corre cada 2 minutos), pero el impacto es de escalada de privilegios.
- Referencia externa: OWASP ASVS 5.0 V7 (Session Management) · CWE-362 / CWE-639.
- Recomendación:
  - guardar `usuario_id`/`cedula` en `SesionSupabaseCacheada` y exigir que coincida en `access_token_supabase_vigente`;
  - limpiar `sesion_supabase` en `iniciar_sesion`;
  - hacer la renovación con compare-and-set (solo guardar si el `refresh_token` vigente sigue siendo el leído);
  - en el núcleo, usar el `access_token` que devuelve el `login()` de revalidación, no uno externo.

### [DT-05] Sesión local sin expiración ni cierre por inactividad; el "tope de 12 h" no protege nada
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `estado.rs:185` (`sesion_activa` sin vencimiento), `estado.rs:51` (`TOPE_PRESENCIA_SUPABASE`), `comandos/nube.rs:243-246`.
- Estado: Nuevo
- Descripción y evidencia: la `UsuarioSesion` vive en memoria hasta que alguien pulsa cerrar sesión, se cierra la app o el usuario es dado de baja remotamente. No hay timeout de inactividad ni en backend ni en frontend (grep sin resultados). El tope de 12 h solo afecta a `access_token_supabase_vigente`, que únicamente usa `cambiar_password_supabase`. Además, cada sincronización (cada 2 min) renueva `confirmada_en`, así que el tope nunca se alcanza mientras haya red.
- Escenario de impacto: en un PC compartido de portería, el guardia saliente deja la sesión abierta y el entrante registra ingresos y salidas que la auditoría atribuye al anterior. Eso afecta el no repudio de los registros de acceso.
- Referencia externa: OWASP ASVS 5.0 V7.3 (Session Timeout) · https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- Recomendación: registrar la última actividad en `GuiState` y hacer que `sesion_activa()` falle tras N minutos de inactividad (configurable) y tras un tope absoluto. En el frontend, bloquear la pantalla. Renovar la sesión de Supabase solo cuando esté cerca de vencer, no en cada sincronización.

### [DT-06] La recuperación de una base dañada ignora `-wal`/`-shm` (la base está en modo WAL)
- Severidad: Media
- Categoría: Mala práctica / Integridad de datos
- Ubicación: `recuperacion_local.rs:56-75` (solo borra `control_acceso.db` y `db_key.dat`), `recuperacion_local.rs:95` y `:106` (solo copia esos dos archivos); `src/database/schema.rs:60` (`PRAGMA journal_mode = WAL`).
- Estado: Nuevo
- Descripción y evidencia: la cuarentena copia la base sin su `-wal`, así que las transacciones que aún no pasaron a la base se pierden en el respaldo "para rescate". Además borra la base pero deja el `-wal`/`-shm` viejos junto a la base nueva, que se crea con otra clave. SQLite documenta ambas prácticas como causa de corrupción: *"Copying a database file without also copying its journal"* y *"Overwriting a database file with another without also deleting any hot journal associated with the original database"*.
- Escenario de impacto: tras aceptar "Reconstruir desde la nube", la base nueva puede abrirse con un WAL ajeno, cifrado con la clave anterior. Puede volver a fallar ("file is not a database"), lo que abre de nuevo el diálogo y provoca un ciclo de reconstrucción, o quedar corrupta. El respaldo en `respaldos-corruptos/` además está incompleto.
- Referencia externa: https://www.sqlite.org/howtocorrupt.html §1.3–1.4
- Recomendación: incluir `control_acceso.db-wal`, `-shm` y `-journal` tanto en la copia de cuarentena como en el borrado, y añadir un test que cree esos archivos.

### [DT-07] Sentry recibe un evento cada 2 minutos mientras no hay sesión (contradice su doc-comment) y rota los logs útiles
- Severidad: Media
- Categoría: Mala práctica / Observabilidad
- Ubicación: `lib.rs:78-119` (`iniciar_sincronizacion_automatica`), `lib.rs:43-65` (doc-comment), `comandos/nube.rs:148`.
- Estado: Nuevo (regresión introducida con el punto 5.4 del plan QA)
- Descripción y evidencia: el doc dice que "sin sesión activa… no son errores acá… Silencioso". Sin embargo `autenticar()` devuelve `Err("No hay una sesión activa")` o `"Todavía no se guardó el secreto…"`, y el bucle hace `log::warn!` y `sentry::capture_message(…, Warning)` para todo `Ok(Err(_))`. Además, el bloque de doc de `iniciar_sincronizacion_automatica` quedó pegado al de `precargar_catalogo_durante_splash`, así que la primera función no tiene documentación.
- Escenario de impacto: un PC que pasa la noche en la pantalla de login envía unos 30 eventos por hora a Sentry, lo que agota la cuota del plan gratuito y esconde los errores reales. El archivo de log (tope por defecto de 40 KB, `KeepOne`) se llena de este ruido y rota antes de conservar los fallos útiles.
- Referencia externa: https://docs.sentry.io/product/accounts/quotas/ · OWASP ASVS 5.0 V16 (Security Logging).
- Recomendación: devolver un error tipado (`SinSesion`, `SinSecreto`) y tratarlo como un `continue` silencioso. Reportar a Sentry solo los fallos de red o de servidor, con deduplicación o muestreo. Separar los dos doc-comments.

### [DT-08] Login local sin freno contra fuerza bruta y con enumeración de cédulas
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `comandos/autenticacion.rs:169-180` (`login` → `intentar_login_local`); núcleo: `src/services/autenticacion_service.rs:82-118` y `src/mensajes.rs:31-47`.
- Estado: Nuevo en esta capa (el núcleo lo comparte; ver el informe del núcleo)
- Descripción y evidencia: no hay contador de intentos ni bloqueo, ni en `GuiState` ni en el núcleo. El camino de hash cacheado offline no pasa por los límites de Supabase. `buscar_candidato` responde "Usuario inactivo" antes de verificar la contraseña, y una cédula inexistente responde sin ejecutar Argon2 (diferencia de tiempo). Ambas cosas permiten enumerar cédulas.
- Escenario de impacto: con acceso físico al PC de portería desatendido, se puede automatizar la prueba de contraseñas con un emulador de teclado USB (Argon2 solo limita a unos pocos intentos por segundo) y descubrir qué cédulas existen o están inactivas.
- Referencia externa: OWASP ASVS 5.0 V6.3 (anti-automatización / credential stuffing) · CWE-307 · CWE-204.
- Recomendación: añadir en `GuiState` un contador por cédula y global con retardo exponencial y bloqueo temporal. Unificar los mensajes ("Credenciales inválidas") y verificar contra un hash ficticio cuando la cédula no existe.

### [DT-09] `exportar_historial_pdf` retiene el `Mutex<AppCore>` durante la consulta completa
- Severidad: Media
- Categoría: Rendimiento / Mala práctica
- Ubicación: `comandos/historial.rs:293-305` (`let core = state.core(); … core.movimientos_completos(...) / movimientos_en_orden(...)`).
- Estado: Nuevo
- Descripción y evidencia: la variante Excel usa `conexion_secundaria` precisamente para no bloquear el núcleo, pero la del PDF toma el candado compartido. El camino con `uuids` recorre todo el historial página a página, sin el tope `LIMITE_CARGA_COMPLETA_MAXIMO`. El núcleo ya expone `movimientos_completos_con_conexion` y `movimientos_en_orden_con_conexion` (`src/application/historial.rs:167,217`).
- Escenario de impacto: mientras se genera un PDF grande, cualquier `registrar_ingreso` o `registrar_salida` queda esperando el mutex. Combinado con DT-01, además congela la UI.
- Referencia externa: https://tokio.rs/tokio/tutorial/shared-state (no retener candados durante trabajo largo).
- Recomendación: usar `state.conexion_secundaria()` junto con las variantes `_con_conexion`, como hace `exportar_historial`.

### [DT-10] Ejecutables e instaladores de Windows sin firma Authenticode
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `tauri.conf.json` (`bundle.windows` sin `certificateThumbprint`/`signCommand`), `.github/workflows/release.yml:82-98`.
- Estado: Nuevo
- Descripción y evidencia: solo existe la firma minisign del updater. El `.msi`/`.exe` publicados en GitHub Releases no llevan firma de código de Windows.
- Escenario de impacto: SmartScreen muestra "editor desconocido" y los sitios se acostumbran a pulsar "Ejecutar de todas formas". Un instalador troyanizado enviado por correo es indistinguible del legítimo en la primera instalación, que el updater no protege.
- Referencia externa: https://v2.tauri.app/distribute/sign/windows/
- Recomendación: firmar con Azure Trusted Signing o un certificado OV/EV mediante `bundle.windows.signCommand` en el job de release, con el secreto en un Environment protegido (ver DT-11).

### [DT-11] Release: firma del updater accesible a builds sin revisar y a scripts de dependencias
- Severidad: Baja
- Categoría: Seguridad (cadena de suministro)
- Ubicación: `.github/workflows/release.yml:12-16` (`workflow_dispatch` sobre cualquier rama), `:82-87` (claves `TAURI_SIGNING_PRIVATE_KEY*` en el mismo paso que ejecuta `npm run build`/`cargo build`), `:113-117` (secretos del keystore Android en el `env` de todo el job), `:160` (`cargo install cargo-ndk` sin versión ni `--locked`).
- Estado: Nuevo
- Descripción y evidencia: sin `environment:` con revisores obligatorios, cualquier persona con permiso de escritura puede producir artefactos con firma válida del updater desde una rama no revisada. Un borrador que se publique después se distribuye a todos los sitios por el auto-update. Los `build.rs`, plugins de Vite y `postinstall` de dependencias se ejecutan con la clave privada en el entorno. Los secretos de Android están disponibles también durante `cargo install` y la compilación con gradle.
- Escenario de impacto: una dependencia npm o crate comprometida exfiltra `TAURI_SIGNING_PRIVATE_KEY` y su contraseña, y con eso puede firmar actualizaciones maliciosas que el updater acepta.
- Referencia externa: https://docs.github.com/actions/deployment/targeting-different-environments/using-environments-for-deployment · https://v2.tauri.app/plugin/updater/#signing-updates · https://github.com/tauri-apps/tauri/security/advisories/GHSA-2rcp-jvr4-r259 (precedente de filtración de la clave del updater por variables de entorno).
- Recomendación:
  - usar un Environment `release` con revisores obligatorios y restricción a tags `v*`;
  - separar la compilación (sin secretos) de la firma (`tauri signer sign` sobre los artefactos en otro job);
  - pasar los secretos de Android solo a los pasos de gradle;
  - usar `cargo install cargo-ndk --locked --version X`.

### [DT-12] Capabilities y ACL de comandos más amplias de lo necesario
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `capabilities/default.json` (`core:default`), `build.rs:1-3` (sin `AppManifest`), `tauri.conf.json` (ventana `splashscreen`), `pdf/generador.rs:66` (ventana `exportar-pdf-N`).
- Estado: Nuevo
- Descripción y evidencia: el frontend solo usa `getVersion`, `listen` e `invoke` (grep de `@tauri-apps/api`). Sin embargo, `core:default` concede además los permisos de window/webview/menu/tray/image/path/resources y `event:allow-emit`. Como `build.rs` no declara `AppManifest::commands`, los 85 comandos propios están permitidos en **todas** las ventanas: el splash y la ventana oculta del PDF, que carga un `file://`.
- Escenario de impacto: amplía la superficie si alguna vez se inyecta marcado en otra ventana. Hoy lo mitiga el escape HTML correcto de DT-15.
- Referencia externa: https://v2.tauri.app/security/capabilities/ (*"all commands that you registered in your app are allowed to be used by all the windows… use AppManifest::commands()"*) · https://v2.tauri.app/security/permissions/
- Recomendación: sustituir `core:default` por `core:app:allow-version`, `core:event:allow-listen` y `core:event:allow-unlisten`. Declarar `tauri_build::Attributes::new().app_manifest(AppManifest::new().commands(&[...]))` y crear una capability `main` que liste explícitamente `allow-<comando>`.

### [DT-13] CSP: `unsafe-inline` en estilos, proyecto de staging en producción y directivas sin respaldo
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `tauri.conf.json` (`app.security.csp`).
- Estado: Nuevo
- Descripción y evidencia:
  - `style-src 'self' 'unsafe-inline'`.
  - `connect-src` incluye `pmrytjktlyiuikxuuxpr.supabase.co`, que es el proyecto de staging según `docs/recuperacion-sitio-staging.md:10`.
  - Faltan `base-uri`, `form-action` y `frame-ancestors`, que no heredan de `default-src`.
  - No se usa `freezePrototype` ni el isolation pattern, aunque el bundle incluye muchas dependencias npm.
- Escenario de impacto: si hubiera un XSS, podría exfiltrar datos hacia un proyecto Supabase que no es el de producción, e inyectar `<base>` o formularios.
- Referencia externa: https://v2.tauri.app/security/csp/ · https://v2.tauri.app/concept/inter-process-communication/isolation/ · https://developer.mozilla.org/docs/Web/HTTP/Headers/Content-Security-Policy/base-uri
- Recomendación: generar la CSP por entorno (staging solo en builds de staging) y añadir `base-uri 'none'; form-action 'none'; object-src 'none'; frame-ancestors 'none'`. Evaluar `"freezePrototype": true` y el isolation pattern. Medir si `unsafe-inline` puede sustituirse por hashes o nonces.

### [DT-14] URL de la nube sobrescribible en tiempo de ejecución en builds de producción
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `src/nube/mod.rs:39-62` (`std::env::var("CONTROL_ACCESO_SUPABASE_URL")`), usado por todos los comandos de nube y por `login_supabase` en el escritorio.
- Estado: Nuevo (compartido con el núcleo)
- Descripción y evidencia: no hay validación de `https://` ni lista permitida, y el override no está limitado a builds de depuración o staging. La CSP del webview no aplica al backend Rust.
- Escenario de impacto: un proceso local que fije la variable en `HKCU\Environment` redirige el secreto de dispositivo y las contraseñas de login (`nube::login`) a un servidor ajeno, incluso por HTTP en claro.
- Referencia externa: CWE-15 (External Control of System or Configuration Setting).
- Recomendación: resolver la URL en compilación (`option_env!`) en release, o exigir `https://*.supabase.co` más una lista permitida.

### [DT-15] Clave de la base: copias en claro sin borrar
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `clave_cifrado.rs:125-137` (`plano: Vec<u8>` se libera sin `zeroize`), `clave_cifrado.rs:197-211` (`LocalFree` sin `SecureZeroMemory` del buffer que devuelve `CryptUnprotectData`); núcleo `src/database/connection.rs:126` (`format!("x'{}'", clave_a_hex(..))` en cada `conexion_secundaria()`, es decir, en cada lectura de historial o sincronización).
- Estado: Nuevo
- Descripción y evidencia: `Zeroizing<[u8;32]>` solo protege la copia final. Quedan en el heap liberado otras dos copias, más una en hexadecimal por cada conexión secundaria. `estado.rs` además guarda el secreto de dispositivo en `TokenCacheado.secreto: String` sin borrar.
- Escenario de impacto: un volcado de memoria (crash dump enviado a soporte, malware del mismo usuario) contiene la clave de la base.
- Referencia externa: https://learn.microsoft.com/windows/win32/api/dpapi/nf-dpapi-cryptunprotectdata · https://docs.rs/zeroize
- Recomendación: devolver `Zeroizing<Vec<u8>>` desde `copiar_y_liberar`, poner a cero `pbData` antes de `LocalFree` y construir el PRAGMA en un `Zeroizing<String>`. En `TokenCacheado`, guardar un hash del secreto en vez del secreto.

### [DT-16] El HTML del PDF (con PII) se escribe en %TEMP% y se carga como `file://` sin CSP
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `pdf/generador.rs:47` (`std::env::temp_dir().join("exportar-pdf-N.html")`), `pdf/generador.rs:66` (`WebviewUrl::External(file://…)`).
- Estado: Nuevo
- Descripción y evidencia: el HTML con cédulas, nombres y placas queda en claro en `%TEMP%`, con nombre predecible, fuera del cifrado de la base. Si el proceso muere o hay un panic antes del `remove_file`, el archivo persiste. La CSP de la app no se aplica a `file://`. El escape en `pdf/html.rs:107-113` es correcto (ver "Aspectos bien resueltos"), pero no hay una segunda capa de defensa.
- Escenario de impacto: queda PII residual en disco sin cifrar, y un error futuro de escape se ejecutaría en una ventana con acceso a los 85 comandos (DT-12).
- Referencia externa: OWASP ASVS 5.0 V14.2 (datos sensibles en archivos temporales) · CWE-377.
- Recomendación: cargar el HTML en memoria (`WebviewUrl::App` con un protocolo propio, o `navigate_to_string`/data URL) o, como mínimo, crear el archivo con `tempfile` y borrarlo en un `Drop`. Añadir `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'">` al documento.

### [DT-17] Mensajes de error técnicos crudos al usuario y política inconsistente
- Severidad: Baja
- Categoría: Seguridad / Mala práctica
- Ubicación: `comandos/mod.rs:34-38` (`mensaje_generico` devuelve `error.to_string()`), `comandos/autenticacion.rs` (`From<AuthSupabaseError> for ErrorLogin` → `to_string()`, que incluye `Red(String)` con la URL de reqwest y no se registra en el log), frente a `comandos/citas.rs` y `rutas.rs`, que descartan el error con `map_err(|_| "...")` **sin registrarlo**.
- Estado: Nuevo (el plan QA 5.1 registra en el log, pero sigue exponiendo el `Display`)
- Descripción y evidencia: el propio `citas.rs` dice "no exponer su Display (interpola el error crudo de SQLite)", mientras que `mensaje_generico` lo expone en unos 34 sitios, rutas de archivo incluidas (errores de `conexion_secundaria` y de `RespaldoDestino`). Las variantes `map_err(|_| …)` pierden el contexto sin dejar rastro.
- Escenario de impacto: el operador ve rutas y mensajes de SQLite. Los fallos de login por red y de listados no quedan en el log ni en Sentry.
- Referencia externa: CWE-209 · OWASP ASVS 5.0 V16.5.
- Recomendación: que `mensaje_generico` registre el error completo y devuelva un texto genérico. Sustituir los `map_err(|_| …)` por una variante que registre. Registrar en el log los `AuthSupabaseError::Red`.

### [DT-18] Sentry: nombre de equipo y rutas de usuario en los eventos
- Severidad: Baja
- Categoría: Seguridad (privacidad)
- Ubicación: `lib.rs:458-475` (`inicializar_sentry` sin `server_name` ni `before_send`), `lib.rs:160-163` y `262-266` (mensajes con `ruta.display()` que incluye `C:\Users\<usuario>\…`).
- Estado: Nuevo
- Descripción y evidencia: la `ContextIntegration` por defecto asigna `server_name` = hostname (*"if options.server_name.is_none() { options.server_name = server_name() }"*). Los errores de `ErrorClaveBaseDatos` interpolan rutas con el nombre de usuario de Windows.
- Escenario de impacto: se envían a un tercero datos identificables del equipo y del empleado.
- Referencia externa: https://docs.rs/sentry-contexts/latest/src/sentry_contexts/integration.rs.html · https://docs.sentry.io/platforms/rust/data-management/sensitive-data/
- Recomendación: `.server_name("")` o un identificador anónimo del sitio, y un `before_send` que reemplace `C:\Users\[^\\]+` por `%USERPROFILE%`.

### [DT-19] Lógica y SQL de negocio en la capa de comandos; duplicación que ya produjo un bug
- Severidad: Baja
- Categoría: Acoplamiento
- Ubicación:
  - SQL directo: `comandos/historial.rs:97-144`, `comandos/nube.rs:428-584`, `comandos/citas.rs` (`listar_historial_visitas_sitio`), `comandos/proveedores.rs` (`listar_historial_ingresos_proveedor_sitio`), `comandos/gafetes_provisionales.rs` (`listar_gafetes_provisionales_historial_sitio`).
  - Orquestación duplicada de la sincronización: `comandos/nube.rs:226-325` frente a `AppCore::sincronizar_con_nube`; el comentario de `nube.rs:270-274` documenta el bug del 2026-09-22 causado por esa divergencia.
  - La construcción de `nube::ContextoSincronizacion` se repite 11 veces y los chequeos `*_libre_en_otro_dispositivo` / `*_activo_en_otro_sitio` aparecen 4 veces casi idénticos (ingresos, citas, proveedores, gafetes_provisionales).
- Estado: Nuevo
- Descripción y evidencia: contradice la convención 4 de `comandos/mod.rs:26-28` ("Sin lógica propia… esa lógica va al núcleo"). `ResumenSincronizacion` (`nube.rs:35-83`) duplica el tipo del núcleo con campos que se rellenan a mano, a veces con `0` (`nube.rs:354-359`).
- Escenario de impacto: cada tabla nueva de sincronización hay que añadirla en dos sitios, y ya hubo una regresión real por eso. Los tests del núcleo no cubren estas consultas.
- Referencia externa: https://v2.tauri.app/develop/calling-rust/ (los comandos como capa fina) · principio de responsabilidad única.
- Recomendación: mover las consultas a `src/database/queries/*` y exponerlas vía `AppCore` o `*_con_conexion`. Hacer que el escritorio use una única función del núcleo `sincronizar_con_conexion(&conexion, &contexto)`. Crear un helper `contexto_nube(state) -> (TokenDispositivo, ContextoSincronizacion)` y una función genérica de chequeo remoto.

### [DT-20] Bloqueos síncronos dentro de comandos `async`
- Severidad: Baja
- Categoría: Mala práctica / Rendimiento
- Ubicación:
  - `comandos/autenticacion.rs:169` y `180`: Argon2 y el `std::sync::Mutex` de `core` directamente en `async fn login`.
  - `comandos/autenticacion.rs` (`login_supabase`): `state.core()` inline.
  - `comandos/ingresos.rs` (`preparar_ingreso`) y `comandos/citas.rs` (`verificar_check_in_visita`): `state.core()` inline.
  - `comandos/exportacion.rs` (`exportar_tabla_pdf`) y `pdf/generador.rs:47`: generación de HTML grande y `std::fs::write` inline.
- Estado: Nuevo
- Descripción y evidencia: estas operaciones bloquean un worker del runtime tokio de Tauri. Si otro comando retiene `core` (DT-09), el worker queda esperando.
- Escenario de impacto: con varios comandos async concurrentes (login, sincronización, realtime), el runtime se satura y las respuestas se retrasan.
- Referencia externa: https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
- Recomendación: envolver en `spawn_blocking` todo acceso a `core()`, Argon2 y E/S de archivos dentro de funciones `async`.

### [DT-21] Código muerto
- Severidad: Info
- Categoría: Código muerto
- Ubicación y evidencia:
  - `comandos/gafetes_provisionales.rs:53` `buscar_encargados_ruta_provisional`: registrado en `lib.rs:556` pero **nunca invocado** desde `desktop/src`. Es el único de los 85 comandos sin uso; no hay comandos definidos sin registrar ni invocaciones sin comando.
  - `Cargo.toml`: la dependencia `serde_json = "1.0"` no se usa en ningún archivo del crate.
  - Ramas `#[cfg(not(windows))]` en `lib.rs:174-177`, `237-240` y `251-255`, y en `pdf/generador.rs` (`lanzar_print_to_pdf`): no compilan, porque `intentar_abrir_nucleo` (`lib.rs:186`) es `#[cfg(windows)]` y `zeroize` solo se declara en `[target.'cfg(windows)'.dependencies]` pero se importa sin condición (`lib.rs:10`, `estado.rs`). Dan una falsa sensación de portabilidad.
  - `#[cfg_attr(mobile, tauri::mobile_entry_point)]` (`lib.rs:488`) con `crate-type` sin `cdylib`.
  - `packaging/msix/`: empaqueta `control_acceso.exe`, un binario que ya no existe (el commit `a98d7d9` retiró la CLI; `7daf456` borró `packaging/alacritty` por el mismo motivo). Además usa la misma marca "Lattis" que la app Tauri y la versión `1.0.0.0`.
  - `dto/proveedores.rs`: `DatosIngresoProveedor` solo reempaqueta argumentos. `DatosContratistaEntrada` es una tercera copia de dos structs idénticas del núcleo.
- Referencia externa: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html · `cargo machete` / `cargo udeps`.
- Recomendación: eliminar el comando y la dependencia; eliminar las ramas no-Windows o hacer que el crate compile de verdad en Linux; borrar o actualizar `packaging/msix`.

### [DT-22] El token de dispositivo se expone al webview
- Severidad: Info
- Categoría: Seguridad
- Ubicación: `comandos/nube.rs:389-409` (`sesion_realtime_nube` devuelve `access_token`, `apikey` y `sitio_id`).
- Estado: Nuevo
- Descripción y evidencia: es necesario para Realtime en `nubeRealtime.ts`, que ya usa `persistSession:false` (bien). Con un XSS, el token del dispositivo (identidad de todo el sitio ante RLS) sería exfiltrable hasta que venza.
- Referencia externa: https://v2.tauri.app/security/ · https://supabase.com/docs/guides/realtime/authorization
- Recomendación: a medio plazo, suscribirse a Realtime desde Rust y reenviar eventos al frontend con `emit`; como alternativa, emitir un token con alcance solo de Realtime y vida corta.

### [DT-23] Varios menores de la capa nativa
- Severidad: Info
- Categoría: Mala práctica
- Descripción y evidencia:
  - Instancia única: `InstanciaGuard` convierte el segundo arranque en un "error fatal", con un evento Sentry `Fatal` (`lib.rs:392-397`), en vez de enfocar la ventana existente (`tauri-plugin-single-instance`).
  - El login no aplica `debe_cambiar_password` en el backend: solo el frontend fuerza el cambio y los comandos quedan operativos con la contraseña temporal.
  - `cerrar_sesion` no revoca el refresh token en Supabase (`/auth/v1/logout`).
  - Hay dos `reqwest` en el árbol (0.12 del núcleo y 0.13 de tauri/sentry/updater) y `native-tls`/`openssl-src` vendorizado, lo que alarga el build y la superficie.
  - `build-test-desktop.yml` compila con `createUpdaterArtifacts: true` sin `TAURI_SIGNING_PRIVATE_KEY`: verificar que `tauri build` no falle en el paso de firma del updater, porque el CLI v2 exige la clave privada cuando hay `pubkey`.
- Referencia externa: https://v2.tauri.app/plugin/single-instance/ · https://supabase.com/docs/reference/api/auth-logout
- Recomendación: tratarlos en el backlog de higiene.

---

## Aspectos bien resueltos
- **Autorización en backend**: los 85 comandos fueron revisados uno a uno. Todos exigen `sesion_activa()` salvo los que no pueden hacerlo (`requiere_configuracion_inicial`, `login`, `cerrar_sesion`, `mostrar_ventana_principal`, y `configurar_dispositivo_inicial`, que el núcleo protege con `YaConfigurado`). En las escrituras, el núcleo vuelve a leer `activo` y el rol desde la base dentro de la transacción (`verificar_actor_activo`), y `autorizar_uso_nube` hace lo mismo. No se confía en el rol que envía el frontend, y `generado_por` del PDF sale de la sesión.
- **Escape HTML del PDF** (`pdf/html.rs:107-113`): se escapan `& < > "` en todos los valores, que solo aparecen en contexto de texto. Tiene tests de inyección.
- **Exportación XLSX** con `write_string_with_format`: sin inyección de fórmulas.
- **Updater**: `pubkey` minisign, endpoint HTTPS de GitHub `releases/latest` (los borradores no se distribuyen). Sin `withGlobalTauri` y sin la feature `devtools` en release.
- **Clave de base de datos**: 32 bytes de `OsRng` protegidos con DPAPI de ámbito usuario (nunca `LOCAL_MACHINE`). Escritura atómica. No se genera una clave nueva sobre una base existente. Tests del flujo.
- **Sesión y tokens** solo en memoria (nunca en disco ni en `localStorage`; Realtime con `persistSession:false`). Mutex con recuperación de envenenamiento.
- **CI**: acciones fijadas por hash, `persist-credentials: false`, `cargo audit` para el crate de escritorio, sin cachés en release y `inputs.version` pasado por `env`/`with` (sin inyección de script).
