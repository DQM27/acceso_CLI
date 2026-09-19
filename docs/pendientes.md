# Pendientes consolidados

Documento único de trabajo pendiente del repo. Reemplaza las listas paralelas y notas
viejas de auditoría, nube, móvil, escritorio, OCR, empaquetado y planes de producto.

## Regla del archivo

Al terminar una tarea de aquí, se marca `[x]` en el mismo commit o en el siguiente. Si
una tarea se descarta, también se marca `[x]` con una nota corta de por qué. Los planes
históricos pueden seguir existiendo como contexto, pero esta lista manda.

## Fuentes consolidadas

- `docs/auditorias/auditoria-calidad-2026-09.md`
- `docs/planes-implementados/plan-persistencia-nube.md`
- `docs/planes-implementados/plan-panel-administrativo-web.md`
- `docs/planes-implementados/plan-ocr-escaneo-documentos.md`
- `docs/arquitectura/fixtures-ocr-sinteticos.md`
- `docs/features-futuras/idea-lector-placas-vehiculares.md`
- `mobile/android/arquitectura.md`
- `mobile/README.md`
- `docs/features-futuras/plan-sesion-unica-dispositivos.md`
- tracker anterior de escritorio (absorbido aquí; ya no existe como lista aparte)
- `README.md`, `packaging/msix/README.md`, `docs/recuperacion-supabase.md`,
  `docs/auditorias/realtime-verificado.md`

---

## Seguridad y nube

- [x] **El pipeline de release real (no sólo `cargo tauri dev`) publicaba
  builds sin cifrar de verdad, en silencio (hallazgo 2026-09-12, resuelto
  2026-09-12).** `desktop/src-tauri` compilaba con `sqlite-plano` como
  motor por defecto (a propósito en su momento, mientras
  `bench/sqlite-3way-*` evaluaba los tres motores). El problema:
  `clave_cifrado::resolver_clave` + `AppCore::abrir_con_reloj_cifrado`
  corren SIEMPRE en `lib.rs::run()`, sin chequear qué motor está realmente
  enlazado -- generan y guardan `db_key.dat` con DPAPI igual, y llaman
  `PRAGMA key` igual, pero con SQLite plano ese `PRAGMA` no hace nada (no
  es un error, simplemente se ignora). **No era sólo un riesgo de
  `cargo tauri dev` local:** `.github/workflows/release.yml` invocaba
  `tauri-apps/tauri-action` sin ningún `--features`/override -- heredaba el
  mismo default. Confirmado que **v1.5.0, v1.5.1 y v1.5.2 (los releases
  reales publicados en GitHub) se compilaron sin cifrado real de la base
  local**.

  **Fix:** `desktop/src-tauri/Cargo.toml` cambia su `default` a
  `cifrado-sqlite3mc` (motor real, compila en segundos con
  `sqlite3mc-vendor-lib/` ya construida -- ver `docs/decisiones-tecnicas.md`)
  en vez de `sqlite-plano`. `release.yml` agrega el paso obligatorio que
  compila `sqlite3mc-vendor-lib/` antes de tests/build. Esto cierra el
  riesgo de "ejecutable sin `--features` explícito termina sin cifrar" en
  los dos caminos de entrada (dev local y pipeline de release).

  **Sigue pendiente, más chico:** `run()` no valida en tiempo de ejecución
  qué motor quedó enlazado -- si alguien pide `sqlite-plano` a propósito
  (features siguen siendo compile-time, `sqlite-plano` sigue existiendo
  para iterar rápido) y ese build termina en un sitio real, el silencio se
  repite. Falta decidir si `run()` debe negarse a arrancar con
  `sqlite-plano` fuera de un build de desarrollo explícito, o al menos
  avisar fuerte en la UI que la base no está cifrada de verdad.
- [x] **Android: proteger el secreto del dispositivo con Keystore.** El secreto móvil
  se guarda desde Kotlin con Android Keystore (`AES/GCM/NoPadding`) y el núcleo móvil recibe
  el secreto descifrado sólo en memoria para autenticarse/sincronizar. Incluye migración
  suave del archivo legado administrado por Rust; desktop mantiene su cifrado actual.
- [x] **Redactar `Debug` de credenciales de nube.** `TokenDispositivo`
  (`src/nube/cliente.rs`) y `SesionRealtimeNube` (`src/application/nube.rs`) tienen
  `Debug` manual con `access_token`/`apikey` redactados, cubierto por pruebas.
- [ ] **`cerrar_ingreso_remoto` manda `hora_salida` sin corregir desfase de
  reloj.** A diferencia de otros caminos, calcula la hora con
  `chrono::Utc::now()` crudo del dispositivo, no con el reloj corregido
  contra el servidor. Encontrado en vivo (2026-09-08): una PC con el reloj
  atrasado no llegó a mandar una hora mala a la nube porque el guardia
  local (`RelojRetrocedido`) frenó antes, comparando contra el último
  movimiento local -- pero ese guardia es local, no protege este camino
  remoto. Sin reproducir todavía; anotado para revisar si vale la pena
  aplicar la misma corrección de desfase acá.
- [ ] **Mitigar timing attack en login local.** Si la cédula no existe,
  `AutenticacionService::buscar_candidato` rechaza sin correr Argon2; usar un hash dummy
  reduciría la diferencia de tiempo. Riesgo bajo, pero confirmado.
- [ ] **Activación de dispositivo con verificación por correo.** Hoy el secreto correcto
  activa el dispositivo. El flujo diseñado agrega un código por correo en la primera
  activación del secreto, con estado intermedio antes de emitir el JWT final.
- [ ] **Sesión única por dispositivo y presencia en tiempo real.** Ver
  `docs/features-futuras/plan-sesion-unica-dispositivos.md`. El mismo secreto hoy activa más de un
  dispositivo sin límite. Plan: secreto de un solo uso, identidad canónica del
  dispositivo en Supabase, sesión propia desacoplada del secreto, panel de presencia,
  y regla de desempate por fecha de alta + expulsión automática para conflictos
  detectados offline.
  - [x] Presencia en tiempo real ya funcionando (2026-09-08): Dispositivos.tsx y
    Usuarios.tsx muestran en vivo quién/qué está conectado y desde dónde.
  - [ ] El resto (secreto de un solo uso, identidad canónica, desempate offline)
    sigue sin implementar.
- [ ] **Sesión única por SITIO, no por dispositivo ni global (decisión
  refinada 2026-09-12).** Ver `docs/features-futuras/plan-sesion-unica-dispositivos.md`,
  sección 7 -- reemplaza el planteo anterior de esta entrada. Política
  aclarada con el usuario: un mismo operador SÍ puede tener sesión abierta
  en más de un dispositivo del MISMO sitio a la vez (PC + celular en
  Brisas, uso normal), pero NO en dos sitios distintos al mismo tiempo
  (logueado en Cartago no debería poder tener sesión viva en Brisas). El
  primer diseño (chequear presencia del mismo sitio) ya no aplica tal cual
  porque el disparador es "sitio", no "dispositivo" ni "global puro" --
  hoy es más plausible que antes porque la identidad ya vive centralizada
  en Supabase Auth (desktop y mobile migrados, ver
  `docs/planes-implementados/plan-autenticacion-supabase-auth.md`), no repartida por dispositivo.

  **Diseño propuesto, sin implementar:**
  1. `usuarios` suma `sesion_sitio_id` (uuid, nullable, referencia
     `sitios`) + `sesion_iniciada_en` (timestamptz).
  2. Función `security definer` nueva (`marcar_sesion_activa`, mismo
     patrón que `es_admin_global`/Edge Functions de dispositivos): la
     llama el dispositivo (con su propio JWT, que ya trae `sitio_id`)
     justo después de un login de persona exitoso. Si `sesion_sitio_id`
     está `null` o ya es el mismo sitio, sólo actualiza el timestamp. Si
     apunta a OTRO sitio, lo pisa con el nuevo (último login gana, mismo
     criterio ya aceptado en otras partes del sistema para "bloqueo hasta
     reconectar") y marca que hubo conflicto en la respuesta.
  3. **Kick en vivo**: si hubo conflicto, broadcast por Realtime al sitio
     viejo avisando que esa cédula se movió -- mismo mecanismo que ya
     existe para presencia/expulsión de dispositivos.
  4. **Red de seguridad sin Realtime**: la sincronización periódica
     (~2 min, ya existe en desktop y mobile) chequea si `sesion_sitio_id`
     remoto sigue siendo el propio; si no, cierra sesión local sola --
     mismo patrón que ya usa `sesion_expulsada` hoy
     (`ResumenSincronizacion::sesion_expulsada`).
  5. Aplica a desktop y mobile (los dos con Supabase Auth ya andando). TUI
     clásica queda fuera por ahora, igual que el resto de lo pendiente ahí.

  No es una tarea chica: migración + función SQL + wiring de Realtime +
  cambios en Rust core (nube:: nuevo + extender el chequeo de
  "sigue activo") + desktop + mobile. Retomar en una pasada dedicada.
- [ ] **Auditoría (`auditoria_cambios`/`gafetes_incidentes`) no se sincroniza
  entre dispositivos (hallazgo 2026-09-17, sin implementar).** Hoy las dos
  tablas son puramente locales -- no aparecen en `src/nube/sincronizacion.rs`
  ni tienen espejo en `supabase/migrations/` (`gafetes_incidentes` lo dice
  explícito en el propio código, `src/nube/sincronizacion.rs:716-719`: sólo
  el estado ACTUAL de un gafete viaja, no su historial de incidentes). Cada
  PC/dispositivo de un mismo sitio tiene su propio registro de auditoría,
  sin vista consolidada. No es un bug -- fue así desde que se implementó --
  pero conviene revisar si conviene centralizarlo el día que un sitio real
  opere con más de un dispositivo y alguien necesite ver "quién cambió qué"
  sin pararse frente a cada PC. Diseño no arrancado: probablemente tabla
  espejo + `cola_salida` sumando `'auditoria_cambio'`/`'incidente_gafete'`
  a su `CHECK`, mismo patrón que `movimiento_visita`/`ruta` (ver
  `src/database/schema.rs`, comentarios "Suma ... al CHECK de
  `cola_salida.entidad`"). Ojo con el volumen -- a diferencia de
  contratistas/gafetes (catálogos chicos), auditoría crece sin techo, más
  parecido a `historial` (que sí tiene lógica de carga incremental/límite,
  ver `Historial.tsx`/`Auditoria.tsx` con su banner de "truncado").
- [ ] **El chequeo cross-device de "gafete ya ocupado en el sitio" no está
  conectado en escritorio (hallazgo 2026-09-17).** El núcleo ya tiene las
  cuatro variantes (`AppCore::gafete_ocupado_en_sitio`/
  `gafete_provisional_ocupado_en_sitio`/`gafete_de_proveedor_ocupado_en_sitio`,
  `src/application/nube.rs`) -- mejor esfuerzo, consultan Supabase en vivo
  si OTRO dispositivo del mismo sitio ya tiene ese número de gafete en un
  movimiento/ingreso todavía abierto, sin bloquear si no hay red. **Mobile
  sí las usa** (`gafete_ocupado_en_sitio_con_secreto` y sus variantes en
  `mobile/rust-core/src/lib.rs`), pero ningún comando de
  `desktop/src-tauri/src/comandos/` las llama -- ni para contratistas, ni
  provisional KOF, ni proveedores (encontrado revisando el modal de
  ingreso de proveedores). Hoy en desktop, dos PCs del mismo sitio pueden
  asignar el mismo número de gafete casi al mismo tiempo sin que ninguna
  se entere hasta el próximo `sincronizar` -- la única red de seguridad es
  el chequeo LOCAL (`registro_ingresos*`/tabla de gafetes propia), que no
  ve lo que pasó en la otra PC todavía. Falta: llamar la variante
  correspondiente justo antes de confirmar cada tipo de ingreso/entrega en
  los comandos de escritorio, mismo punto donde mobile ya lo hace.
- [ ] **Revisar bucket público `historial-web`.** Está documentado como público, vacío y
  sin referencias en código. Confirmar si es vestigio; si no se usa, eliminarlo desde
  Supabase.
- [ ] **Entrega de la clave de SQLCipher vía Supabase Vault, con envelope
  encryption (discutido 2026-09-12, sin implementar).** Sigue abierto el
  problema de `docs/decisiones-tecnicas.md` ("DPAPI insuficiente contra IT
  del cliente" -- ver memoria de sesión "Cifrado en reposo"): un admin con
  control total de la PC física siempre puede, en teoría, sacarle la clave
  al proceso corriendo (debugger/dump de memoria) -- ningún esquema local
  (DPAPI, Vault, TPM) elimina ese límite de fondo, solo cambia qué tan fácil
  es y cuánto daño limita si se filtra una clave. Además, la clave de
  SQLCipher no rota como un JWT -- cambiarla de verdad exige `PRAGMA rekey`
  (reencriptar toda la base con la clave abierta), no es gratis hacerlo
  seguido.

  **Diseño propuesto para reducir el radio de daño y ganar revocación**
  (separar "quién puede pedir la clave" de "la clave en sí"):
  1. Al aprovisionar un dispositivo (`admin-provision-device`), generar una
     clave de cifrado random **por dispositivo** (no una global) y guardarla
     en Vault con un nombre ligado a su `dispositivo_id`.
  2. Edge Function nueva (`device-fetch-db-key` o similar) que exige el
     mismo JWT que ya valida `device-auth`, y le entrega su clave desde
     Vault -- chequea `revoked_at`/`suspended_at` igual que `device-auth`.
  3. La app la pide una sola vez, en `configurar_dispositivo_inicial`, y la
     usa para abrir/crear la base SQLCipher; se cachea localmente para
     poder operar offline después (ese caché sigue teniendo la misma
     debilidad de fondo que DPAPI -- lo que cambia es que revocar el
     dispositivo en Supabase corta el acceso a pedir la clave de nuevo en
     una máquina distinta, y una clave filtrada sólo compromete UN
     dispositivo/sitio, no todos).

  **Por qué todavía no se hizo:** el desarrollo está en fase temprana, las
  bases locales son desechables y no hay ningún dispositivo real en el
  campo corriendo con SQLCipher activo -- es terreno limpio, sin necesidad
  de migrar/reencriptar nada existente. Retomar esto **antes** de que haya
  dispositivos reales en producción, porque después sí implicaría un
  `PRAGMA rekey` por dispositivo ya desplegado.

  Estimado de esfuerzo cuando se retome: Edge Function nueva + generar y
  guardar la clave al aprovisionar, medio día cada una (reutilizan el
  patrón de validación de JWT ya probado en las demás Edge Functions);
  enganchar el fetch/cacheo en el arranque de la app es lo más delicado,
  un par de días bien probados por plataforma que lo necesite.
- [x] **Edge Functions de dispositivos versionadas.** Se trajo al repo el código remoto y
  se eliminó lo que no tenía llamadores reales.
- [x] **Políticas y funciones de seguridad del panel endurecidas.** Se cerraron accesos
  globales indebidos, ejecución pública de funciones sensibles y dependencias de secretos
  hardcodeados.
- [x] **Realtime probado y acotado a su papel correcto.** Sirve como aviso rápido; el
  polling periódico se mantiene como respaldo.
- [x] **Timeout HTTP de nube agregado.** Las llamadas de red ya no pueden quedar colgadas
  indefinidamente.
- [x] **Reloj corregido por servidor.** Las marcas de sincronización ya no dependen del
  reloj local del dispositivo.

## Auditoría de seguridad -- endurecimiento (2026-09-12)

Arrancado a pedido explícito ("lo van a auditar expertos en seguridad"). Objetivo:
que el propio CI atrape lo mecánico (CVEs conocidos, `unsafe` sin documentar, código
sin formatear/lintear) antes de que llegue a revisión humana. Estado real, no
aspiracional -- lo que sigue sin marcar todavía no corrió.

- [x] **`cargo fmt` en verde en los 3 crates Rust** (raíz, `desktop/src-tauri`,
  `mobile/rust-core`). 12 archivos preexistentes nunca se habían formateado con el
  rustfmt actual (`1.9.0`, dic-2026) -- nada tocado por una feature en curso, era
  deuda ya acumulada. `ci.yml` ya corría `cargo fmt --check` en el job `test`, así que
  probablemente estaba rojo en `main` sin que nadie lo notara (no hay alerta de CI
  fallando configurada en ningún lado).
- [x] **`cargo clippy` agregado a los 3 jobs de `ci.yml`** (`test-android`, `test`,
  `test-gui`) -- sin flags extra, reusa los niveles (`pedantic`/`nursery` como
  aviso, denies curados) que ya vivían en cada `Cargo.toml` desde antes.
- [x] **`unsafe` documentado + acotado.** `undocumented_unsafe_blocks` y
  `multiple_unsafe_ops_per_block` ahora `deny` en los 3 `Cargo.toml`. Se agregó el
  comentario `SAFETY:` que faltaba en 3 bloques (`desktop/src-tauri/src/lib.rs`
  `MessageBoxW`, dos llamadas COM de WebView2 en `pdf/generador.rs`) -- el resto
  (DPAPI en `src/nube/credenciales.rs` y su espejo en
  `desktop/src-tauri/src/clave_cifrado.rs`) ya lo tenía. `mobile/rust-core` no tenía
  ningún `unsafe` -- se le agregó `#![forbid(unsafe_code)]` en `src/lib.rs` para que
  no pueda aparecer uno sin querer (es la frontera FFI hacia Kotlin).
- [x] **`cargo-audit` instalado y agregado a `ci.yml`** (los 3 jobs, vía
  `taiki-e/install-action`) -- CVEs conocidos en dependencias contra la base de
  RustSec. **Sin correr todavía contra el crate raíz de verdad** (ver pendiente de
  abajo) ni contra `desktop/src-tauri`/`mobile/rust-core` -- sólo se instaló el
  binario localmente, falta el `cargo audit` real de cada uno.
- [x] **`desktop/tsconfig.json` con `"strict": true`** -- ya empareja con
  `web`/`web-visitas`, que lo tenían y `desktop` no. Compiló limpio al primer
  intento, cero errores nuevos.
- [x] **ESLint en `desktop`** (`eslint.config.js` nuevo, flat config: `@eslint/js` +
  `typescript-eslint` + `react-hooks` + `react-refresh` + `react` con
  `no-danger`/`jsx-no-target-blank` como las dos reglas realmente "de seguridad").
  Los 15 errores que salieron ya se arreglaron (5 `no-non-null-assertion` con
  chequeos reales, 4 `set-state-in-effect` diferidos vía
  `Promise.resolve().then(...)`, 3 `no-unused-vars` resueltos con
  `argsIgnorePattern: "^_"` en vez de tocar la convención ya establecida del
  código). Quedan 36 warnings cosméticos (`react-refresh/only-export-components`,
  2 de compatibilidad de `react-hook-form` con el compilador de React) -- no
  bloquean nada, se pueden ignorar o limpiar después sin apuro. `tsc --noEmit` y
  `npm run test` (191 tests) en verde tras los cambios.
- [x] **`cargo clippy` del crate raíz -- corregido (2026-09-12).** El archivo era
  `src/database/connection.rs:171` -- doc comment mencionaba "SQLite" sin backticks
  (`doc_markdown = "deny"`). Corregido a `` `SQLite` ``; de paso se sacó un import sin
  usar en `tests/autorizacion_roles.rs` que clippy señalaba como warning.
  `cargo clippy --all-targets` en la raíz queda 100% limpio, sin errores ni warnings.
- [x] **`cargo audit` corrido en los 3 crates (2026-09-12).** Raíz, `desktop/src-tauri`
  y `mobile/rust-core` -- los 3 en verde (exit 0). Sólo salen 7 avisos de
  "unmaintained"/"unsound" en dependencias transitivas (`proc-macro-error`, 5 crates
  `unic-*`, `glib` 0.18 vía GTK del lado desktop) -- ninguno es un CVE explotable
  conocido, son warnings que `cargo audit` deja pasar (`exit 0`) salvo que se pida
  `--deny warnings` explícito, que el `ci.yml` actual no pide. No requieren acción
  inmediata, pero quedan documentados por si se quiere reemplazar esas dependencias
  más adelante.
- [x] **ESLint en `web/` y `web-visitas/` (2026-09-12).** Mismo `eslint.config.js`
  que `desktop`. Salieron 13 y 27 errores reales respectivamente -- ya corregidos
  (principalmente `react-hooks/set-state-in-effect` diferido con
  `Promise.resolve().then(...)`, `no-non-null-assertion` con chequeos reales, y en
  `web-visitas/NuevaCita.tsx` un ref mutado a mano que pasó a ser estado real).
  Los 3 frontends (`desktop`, `web`, `web-visitas`) quedan con `npm run lint` en
  0 errores y ya está wireado en `ci.yml`/`web.yml`. Detalle en los commits
  `fix(desktop)`/`feat(web)`/`feat(web-visitas)` del 2026-09-12.
- [ ] **`cargo-deny` (licencias + dependencias duplicadas/baneadas) -- evaluado y
  descartado a propósito (2026-09-17), no reabrir sin una razón nueva.** No es
  un producto que se redistribuya ni tiene requisitos de compliance de
  licencias, así que el beneficio es bajo frente a `cargo-audit`, que ya
  cubre lo que sí importa (CVEs conocidos). Si algún día se distribuye el
  binario a terceros, agregar `cargo-deny` es el paso natural siguiente.
- [x] **Dependabot (2026-09-17).** `.github/dependabot.yml` -- `cargo` en los
  3 crates (raíz, `desktop/src-tauri`, `mobile/rust-core`), `npm` en los 3
  frontends (`desktop`, `web`, `web-visitas`) y `github-actions`, todos
  semanales. Cierra el hueco real: `cargo audit`/ESLint en CI sólo protegen
  código que se pushea, esto avisa de un CVE nuevo en una dependencia que ya
  estaba instalada.
- [x] **CodeQL -- SAST (2026-09-17).** `.github/workflows/codeql.yml` --
  `javascript-typescript` en push/PR a `main` + cron semanal, cubre `web`,
  `web-visitas` y las Edge Functions de Supabase (la superficie con más
  exposición real a inyección/XSS). El núcleo Rust queda fuera de este
  workflow a propósito -- clippy + `cargo-audit` ya lo cubren y el soporte
  de CodeQL para Rust no es lo bastante maduro todavía para este flujo.
- [x] **`zizmor` -- seguridad de los propios workflows de GitHub Actions
  (2026-09-17).** `.github/workflows/zizmor.yml` -- ninguna de las
  herramientas de arriba mira el YAML de CI en sí; `zizmor` busca patrones
  de inyección de comandos vía inputs no confiables (ej. título/body de un
  PR ejecutándose sin querer dentro de un `run:`). SARIF a la pestaña
  Security, push/PR a `main` + cron semanal. No se pudo compilar/correr
  localmente para probarlo antes de commitear (el `cargo install` se cortó
  solo por límite del sandbox de esta sesión, no un problema real del
  crate) -- usa el método oficial (`pipx run zizmor`, ubuntu-latest ya trae
  `pipx`), verificar que el primer run en CI pase en verde.
- [x] **`cargo-geiger` -- superficie de `unsafe` en dependencias
  (2026-09-17).** `.github/workflows/cargo-geiger.yml` -- audita `unsafe`
  en TODO el árbol de dependencias de los 3 crates, no sólo el código
  propio (eso ya lo cubren `undocumented_unsafe_blocks`/
  `multiple_unsafe_ops_per_block` en cada `Cargo.toml`). Sólo informativo
  (no falla el job), semanal + al tocar algún `Cargo.toml`/`Cargo.lock` en
  `main`. Mismo caveat que `zizmor`: no se pudo probar localmente (mismo
  límite del sandbox), verificar el primer run.
- [ ] **4 upgrades de Dependabot deliberadamente NO mergeados (2026-09-18) --
  cada uno rompe la compilación, revisar con calma antes de intentarlos de
  nuevo.** El primer escaneo de Dependabot agrupó parches inofensivos junto
  con saltos de versión mayores en el mismo PR; se cerraron los 3 PRs de
  grupo (`cargo-raiz`, `cargo-desktop`) y se pidió con
  `@dependabot ignore <paquete> major version` que el próximo ciclo los
  regenere sin estos 4 paquetes, hasta que se decida encararlos:
  - **`jsonwebtoken` 9.3.1 → 11.0.0** (`src/nube/`, firma/valida tokens de
    `device-auth`) -- API rota entre v9 y v11 (elegir crypto backend,
    `EncodingKey`/`DecodingKey` renombrados). Código de seguridad crítico,
    no forzar un merge de grupo.
  - **`argon2` 0.5.3 → 0.6.0** (`src/services/password.rs`, hash de
    contraseñas) -- rompe en compilación real:
    `argon2::password_hash::SaltString`/`rand_core` cambiaron de módulo,
    `Error::Password` ya no existe. Mismo criterio que `jsonwebtoken`.
  - **`sentry` 0.36.0 → 0.49.2** (`desktop/src-tauri`) -- elimina el campo
    `ClientOptions.traces_sample_rate` que se usa en
    `inicializar_sentry()` (ver punto 5.4 de
    `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`).
  - **`webview2-com` 0.38.2 → 0.39.1** (`desktop/src-tauri`, junto con
    `windows` 0.61.3 → 0.62.2 en el mismo grupo) -- rompe con
    `.cast()` ya no disponible en `ICoreWebView2` y un struct que dejó de
    ser exhaustive.

## Panel web y modelo multi-sitio

- [ ] **Pulir paneles web existentes.** Mejorar historial y administración del panel
  desplegado; alcance visual y funcional pendiente de definir.
- [ ] **Revisar roles y permisos Root/Administrador/Operador.** El modelo actual está en
  `domain::autorizacion`; falta decidir si las reglas coinciden con el uso real.
- [x] **Reportes globales decididos: historial completo en Supabase.** `web/src/api/historial.ts`
  lee la tabla `ingresos` como historial multi-sitio y la migración
  `agrega_auditoria_completa_a_ingresos` agregó el detalle de auditoría que faltaba.
- [x] **Chequeo cruzado de ingresos abiertos entre sitios -- bloqueo Y aviso simétrico
  (2026-09-11).** Dos piezas, `docs/pendientes.md` original pedía ambas:

  **1. Bloqueo en vivo** (`nube::contratista_activo_en_otro_sitio`, mejor esfuerzo, tope
  5s): al preparar un ingreso, si la cédula ya está activa en OTRO sitio, no deja
  continuar y nombra el sitio. Sin red, no bloquea -- sigue local (así lo pedía el
  pendiente original).

  **2. "Alertar luego al sincronizar"** (`nube::contratistas_con_conflicto_activo`):
  corre después de cada sync exitoso, revisa TODOS los ingresos activos locales contra
  el estado remoto. Deliberadamente simétrico -- cada sitio en conflicto corre la MISMA
  consulta mirando sus propios activos, así ambos se enteran solos sin necesitar una
  tabla de "notificaciones pendientes" ni un canal de mensajería entre sitios. Cubre el
  caso "se registró offline y nadie lo bloqueó a tiempo".

  **Estado de verificación por plataforma (para quien retome esto) --**

  - **Núcleo Rust** (`src/nube/sincronizacion.rs`): las dos funciones de arriba, con
    tests (`cargo test --no-default-features --features terminal-ui,nube,sqlite-plano`).
    **Verificado, 581+ tests pasando.**
  - **Desktop** (`comandos/ingresos.rs::preparar_ingreso`, `comandos/nube.rs::ejecutar_sincronizacion`,
    `App.tsx::manejarResumenSincronizacion`, `api/ingresos.ts`, `api/nube.ts`): ambas
    piezas conectadas de punta a punta. **Verificado** -- `cargo check` del crate
    `control-acceso-desktop` limpio, `tsc --noEmit` limpio, 204/204 tests de Vitest.
  - **`mobile/rust-core`** (`Nucleo::contratista_activo_en_otro_sitio_con_secreto`,
    campo `conflictos_ingreso` en `sincronizar_con_secreto`): escrito espejando
    `gafete_ocupado_en_sitio_con_secreto` (mismo patrón ya existente en este archivo).
    **`cargo check` lanzado pero sin confirmar terminado en la sesión que escribió esto**
    -- el crate tiene su propio `target/` y build de SQLCipher en frío, puede tardar. Si
    quien retome esto lo ve fallar, revisar primero los dos `From`/construcciones de
    `ResumenSincronizacion`/`PreparacionIngreso` en `mobile/rust-core/src/lib.rs` (son
    dos structs UniFFI propias, DISTINTAS de las del núcleo -- fácil olvidar un campo
    nuevo en alguno de los dos sitios que las construyen).
  - **Android Kotlin** (`ActivosViewModel.kt::elegir`, `PantallaConfirmarIngreso.kt`
    (`puedeContinuar`/`mensajeBloqueo`), `PantallaPrincipal.kt` (aviso de conflicto,
    mismo patrón inline que ya usa `nubeViewModel.error`, esta app no tiene Snackbar/Toast
    todavía)): escrito a mano espejando el patrón de `gafeteOcupadoEnSitioConSecreto` ya
    existente, **pero NUNCA COMPILADO NI CORRIDO** -- no hay entorno de build de Android
    en la sesión que escribió esto. Antes de dar esto por bueno: `./gradlew build` (o
    abrir en Android Studio), y probar a mano el flujo de bloqueo (un contratista con
    ingreso activo en otro sitio) y el aviso tras sincronizar.

  Fuera de alcance todavía: TUI (`PreparacionIngreso::activo_en_otro_sitio` existe en el
  núcleo y `src/tui/nuevo_ingreso/state.rs` ya respeta el campo si algún día se completa,
  pero nadie se lo llena ahí -- la TUI no tiene su propio chequeo remoto conectado).
- [ ] **Reagregar el alta de contratista en mobile, con OCR opcional (pedido
  2026-09-11).** El menú "+" de altas (contratista/empresa/usuario) se sacó de mobile el
  2026-09-06 al limitar la app a "sólo registros rápidos + historial" (ver
  `PantallaPrincipal.kt`, comentario de cabecera, y `mobile/android/arquitectura.md`).
  Ahora se pide devolver puntualmente la de contratista -- con la cédula/DIMEX
  completable por OCR de forma OPCIONAL (ya existe `PantallaEscanearCedula`/
  `ModoEscaneoDocumento` para el escaneo de cédula/gafete en el flujo de ingreso, y todo
  `docs/planes-implementados/plan-ocr-escaneo-documentos.md`/`docs/arquitectura/fixtures-ocr-sinteticos.md` como base -- no
  hay que empezar de cero), pero la carga manual sigue siendo el camino principal, no el
  OCR obligatorio. Sin empezar -- decidir primero si esto revive la pantalla vieja
  (¿sigue existiendo en el historial de git?) o si conviene rehacerla contra el
  `Nucleo`/ViewModels actuales, que cambiaron bastante desde el recorte del 2026-09-06.
- [x] **Scoping futuro de administradores del panel omitido por ahora.** Hoy estar en
  `administradores_panel` da acceso completo; limitar admins por sitio queda fuera hasta
  que exista un caso real.
- [x] **Hosting del panel aceptado para el alcance actual.** El panel vive en Cloudflare
  Pages; reabrir la decisión sólo si el alcance cambia.
- [x] **Alta de dispositivos desde panel web.** `web/src/pantallas/Dispositivos.tsx` usa
  Edge Functions versionadas para crear sitios y provisionar dispositivos.
- [x] **Usuarios globales sincronizados.** ROOT/Administrador/Operador viajan por nube con
  `SIN_PASSWORD_LOCAL`; la contraseña local se fija por dispositivo.
- [x] **Contratistas, empresas y gafetes sincronizados.** El catálogo se recibe completo y
  se fusiona por claves reales, evitando duplicados.
- [x] **Creación de usuarios delegada al panel web.** El panel crea usuarios globales sin
  contraseña real ni temporal.
- [x] **Administradores del panel sin autogestión desde la app.** Alta/baja queda fuera del
  propio panel para no permitir que la superficie protegida se fabrique acceso.

## Android y lector de documentos

- [ ] **`cargo ndk` para `mobile/rust-core` no compila localmente en Windows
  con el motor real (`cifrado-sqlite3mc`, default) -- hallazgo 2026-09-15.**
  El cross-compile de OpenSSL vendorizado (que arrastra `sqlite3mc` vía
  `openssl-sys`) para `aarch64-linux-android` falla en esta máquina: primero
  por el Perl recortado de Git Bash (sin `ExtUtils::MakeMaker`, ver
  [[feedback_path_duplicado_rompe_cache_openssl]] -- usar PowerShell
  esquiva ESTO), y después, ya con PowerShell, por un problema de
  MSYS/`make`/rutas de Windows que rompe las barras del `CC` del NDK al
  pasar por `sh` (`C:\Users\...\clang.exe` termina sin separadores). Sin
  resolver todavía -- el camino que sí funciona hoy es CI
  (`build-android`/`build-test-mobile.yml`, Ubuntu, sin este problema de
  Windows). **`cargo ndk -t aarch64-linux-android build --release
  --no-default-features --features sqlite-plano` sí compila local sin
  problema** (23s, sin tocar OpenSSL) -- sirve para probar UI/diseño rápido
  en el dispositivo real, pero esa base queda SIN CIFRAR: nunca usar ese
  `.so` para un APK que vaya a manejar datos reales.
- [ ] **Mobile muestra errores crudos de nube/sincronización, sin traducir
  (hallazgo 2026-09-12).** Desktop redacta todo error de `nube`/`sync` a
  mensajes amigables (`mensaje_nube`/`mensaje_sincronizacion` en
  `src/mensajes.rs`), pero `mobile/rust-core/src/lib.rs` nunca adoptó ese
  patrón -- 23 sitios hacen `NucleoError::Interno { mensaje:
  error.to_string() }` directo, que Kotlin muestra tal cual llega (texto
  crudo de `reqwest`/HTTP, no una frase en español). Encontrado al ver un
  error "401 jwt expirado" crudo tras loguear con Supabase Auth y tocar
  "Sincronizar" -- la causa real de ESE error puntual ya se investigó y
  cerró (ver `docs/decisiones-tecnicas.md`, "token de dispositivo vencido
  a mitad de sincronización"); esta entrada sigue abierta sólo por el
  problema general de mensajes sin traducir en mobile, no por ese caso
  puntual. Preexistente, no introducido por la migración de login a
  Supabase Auth de esa misma fecha.

Revisado contra código el 2026-09-08. Evidencia principal:
`MrzParser.kt`, `LectorDocumentosIdentidad.kt`, `EstabilizadorLectura.kt`,
`PantallaEscanearCedula.kt` y pruebas unitarias dirigidas en verde para parser,
estabilizador y clasificador.

- [ ] **Extracción de DIMEX/licencia sin confirmar contra texto OCR real.**
  Las regex de `extraerDimex`/`extraerLicencia` están probadas contra texto
  sintético inventado, no contra una lectura real de ML Kit -- el usuario
  reportó que la clasificación anda al toque pero la extracción del número
  casi nunca calza salvo en un ángulo casi perfecto. Falta una foto real de
  cada documento para ajustar las regex contra el formato real (espaciado,
  símbolos como "N°", saltos de línea) en vez de seguir adivinando.
- [x] **Carnet de inducción PRAIND agregado al lector.** Dos variantes de
  diseño reales verificadas con tests (`LectorDocumentosIdentidadTest`);
  extrae cédula, nombre y `fecha_vencimiento_praind`. Clasificado antes que
  cédula nacional porque también trae un número de cédula de 9 dígitos
  propio.
- [x] **Recorte del área de análisis al recuadro guía: revertido.** Se
  probó en dispositivo real y empeoró el escaneo sin dar la velocidad
  prometida -- ver `docs/planes-implementados/plan-ocr-escaneo-documentos.md` sección 9.
  `analizarCedula` vuelve a procesar el frame completo.
- [x] **Bug real encontrado y corregido: enfoque de cámara bloqueado.**
  `disableAutoCancel()` en `FocusMeteringAction` no da "enfoque continuo"
  -- bloquea el foco para siempre en lo que la cámara vio al arrancar. Era
  la causa real de que el escaneo pareciera "exigente". Sacado del todo;
  vuelve al autofocus continuo por defecto de CameraX.
- [x] **Debounce de `EstabilizadorLectura` tolera reflejos intermitentes.**
  Pasa de exigir frames consecutivos idénticos a una ventana deslizante
  (`framesRequeridos` de los últimos `ventana` frames).
- [x] **Flujo frente/reverso resuelto en una sola cámara.** `EstabilizadorLectura` intenta
  MRZ primero y cae a OCR de frente si no hay MRZ; no se requiere paso manual para voltear
  el documento.
- [x] **Fallback de MRZ corrupto decidido: rechazar y reintentar.** Si hay MRZ pero falla
  checksum, `EstabilizadorLectura` devuelve `INVALIDO`; tests cubren que un MRZ corrupto no
  confirma aunque se repita.
- [x] **FPS de CameraX omitido como requisito.** El debounce quedó en 3 frames configurables
  (`EstabilizadorLectura(framesRequeridos = 3)`) y cubierto por tests; calibrar FPS real no
  desbloquea ninguna implementación actual.
- [x] **Detección de reflejo omitida por ahora.** Requeriría análisis de píxeles por frame;
  el lector ya maneja lectura parcial con mensaje de mantener firme sin agregar costo
  `O(imagen)`.
- [x] **Fixtures TD3/pasaporte suficientes para el alcance actual.** Existe parser TD3,
  modelo `PASAPORTE` y test `parseaTd3ValidoCompleto`; ampliar variantes queda para cuando
  pasaporte sea un producto formal.
- [x] **PDF417 de cédula anterior omitido.** El plan documenta que el contenido viene
  cifrado; no se implementa sin acceso legítimo al esquema de descifrado.
- [x] **Refactor móvil a ViewModels convertido en criterio, no pendiente abierto.**
  `mobile/android/arquitectura.md` fija la regla incremental y ya hay ViewModels reales
  para pantallas clave.
- [x] **Clasificador y extractores por tipo de documento.**
- [x] **Parser MRZ TD1/TD3 y checksums.**
- [x] **Distinción de cédula nacional 2025+, DIMEX y menores por MRZ/edad.**
- [x] **Estado central de escaneo, estabilidad y viewfinder.**
- [x] **Recorte lógico del área de análisis por `TextBlock.boundingBox`.**
- [x] **Feedback háptico y sonido sutil al confirmar.**
- [x] **Timer periódico móvil de sincronización.** `SincronizacionPeriodica.kt` hace pulso
  cada 2 minutos y convive con Realtime.
- [x] **Placas vehiculares fuera de esta app.** La idea queda documentada sólo como
  referencia para otro proyecto.
- [x] **Control de flash descartado.** El reflejo perjudica más de lo que ayuda en este
  caso.

## Escritorio, Tauri y empaquetado

- [ ] **Marcar perdido/resolver no existe todavía para gafetes de Proveedor
  (hallazgo 2026-09-17).** `GafeteServiceError`/`AppCore` sólo exponen
  `marcar_gafete_perdido_contratista`/`marcar_gafete_perdido_visita` -- no
  hay `marcar_gafete_perdido_proveedor`. Si un gafete tipo Proveedor se
  pierde hoy, no hay forma de registrarlo desde ninguna interfaz (el
  `CHECK` par-exclusivo de `gafetes` en `schema.rs` ya soporta el caso,
  `proveedor_portador_id` incluido -- ver `src/database/queries/gafetes.rs`,
  que ya trae ese nombre para la columna "Asignado a" desde este mismo
  commit). Falta: método en `AppCore` (mismo molde que
  `marcar_gafete_perdido_visita`, eligiendo un `registro_ingresos_proveedor`
  activo como portador en vez de una cita/contratista), comando Tauri, y
  UI en `GestionGafeteModal.tsx` para elegir el ingreso de proveedor
  (buscar por cédula/nombre, no hay catálogo de personas para autocompletar
  -- mismo criterio que el resto de "control de proveedores").
- [ ] **Verificar actualización con otra instancia abierta.** El riesgo quizá no aplica
  por cómo `relaunch()` reinicia el proceso, pero falta una prueba real.
- [x] **Instalador único que incluya CLI y GUI omitido para v1.** Lujo fuera del alcance;
  reabrir sólo con una necesidad concreta.
- [x] **CLI: Alacritty implementado sin preferencia nueva.** `src/main.rs` relanza en
  `Alacritty.exe` si está junto al binario y evita bucles con
  `CONTROL_ACCESO_EN_ALACRITTY`; no queda como feature abierta.
- [x] **PDF: numeración estilizada de páginas omitida.** Requeriría DevTools Protocol de
  WebView2; no se arma sin pedirlo explícitamente.
- [x] **PDF/WebView2: reporte río arriba omitido.** La solución productiva ya sondea el
  archivo en disco; aislar el bug del callback no aporta al uso actual.
- [x] **Excel: gafete como número y bordes omitido.** Quedó fuera porque no se pidió; el
  exportador actual ya cumple el alcance acordado.
- [x] **Pipeline de release de GUI.**
- [x] **Updater firmado vía GitHub Releases.**
- [x] **Error Boundary en React.**
- [x] **Mensajes de login sin filtrar errores crudos de SQLite.**
- [x] **PDF de Historial implementado.**
- [ ] **RBAC de la GUI de escritorio no está realmente aplanado para
  Operador (hallazgo 2026-09-15).** Tras `feat(auth): aplana autorización
  por rol -- el panel es la única puerta` (commit `bb6a7cf`), un usuario
  con rol Operador en desktop: (1) no puede ver la pestaña Auditoría, (2)
  no puede editar la cédula de un contratista, y (3) sí ve la pestaña
  Usuarios, que debería ser responsabilidad exclusiva del panel web. Los
  tres puntos contradicen la intención de "aplanar" permisos en desktop.
  Sin investigar todavía dónde vive el chequeo de rol que sigue
  restringiendo/mostrando de más -- revisar `domain::autorizacion` y el
  render condicional de pestañas en `desktop/src`.
- [x] **RBAC visual de la GUI corregido.**
- [x] **Auditoría GUI genérica construida.**
- [x] **Respaldos en GUI construidos y revisados.**
- [x] **Exportaciones largas sin bloquear UI/comandos.**
- [x] **Clippy pedantic/nursery cerrado en núcleo y adaptador Tauri.**

## Respaldo, base local y dominio

- [x] **Todo el subsistema de respaldo local (AppCore, TUI, desktop) eliminado
  (2026-09-10/12, commits `e643fe8`/`a98d7d9`) -- las tres entradas de abajo que hablaban
  de él ("automático a la 01:00", "retención de 7 respaldos", "eliminar respaldos no
  usados", "importar respaldo") quedan obsoletas de golpe, no descartadas una por una.**
  Motivo real: el respaldo de archivo era incompatible con SQLCipher (cifrado real);
  se reemplazó por depender de la sincronización con la nube como respaldo efectivo.
  `database::backup`, los métodos de respaldo de `AppCore`, la pantalla "Respaldos" del
  TUI y el flujo de restaurar-respaldo de `main.rs` ya no existen -- no hay reemplazo
  pendiente, es la decisión final. Hallazgo 2026-09-13: las 5 entradas de abajo seguían
  marcadas como si el subsistema existiera, sin ninguna nota -- violaba la propia regla
  de este archivo. Corregido acá.
- [x] ~~Eliminar respaldos no usados con confirmación.~~ Obsoleto -- no hay respaldos locales.
- [x] ~~Importar respaldo cuando no existe base local omitido a propósito.~~ Obsoleto -- no
  hay respaldos locales que importar.
- [x] ~~Respaldo automático a la 01:00 hora Costa Rica.~~ Obsoleto -- ver arriba.
- [x] ~~Retención automática queda en 7 respaldos.~~ Obsoleto -- ver arriba.
- [x] ~~Respaldo previo a migraciones y rollback.~~ **Corrección 2026-09-17, verificado
  contra código**: la nota anterior decía que el respaldo puntual pre-migración "sigue
  vivo" en `src/database/schema.rs` -- es falso, no está. `grep` de `TipoRespaldo`/
  `respaldar_antes_de_migrar` en todo `src/` no encuentra ninguna función ni struct con
  ese nombre; sólo quedaban 3 doc-comments y una variante de error
  (`SchemaError::RespaldoPreMigracionFallido`) que la mencionaban sin que nada la
  construyera jamás (código muerto, invisible a clippy por ser un enum público). Limpiado
  en la rama `qa` (ver `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`, punto
  "Backups"). Lo único que sí sigue vivo hoy contra corrupción de esquema es la
  atomicidad transaccional (una migración que falla revierte sola, `PRAGMA user_version`
  incluido) -- no hay ningún archivo de respaldo real, ni general ni pre-migración.
- [ ] **Agregados de dominio con constructores privados diferidos a V3.** Reabrir con
  concurrencia multi-terminal -- **sigue sin hacer** (revertido de `[x]` a `[ ]`,
  2026-09-13: no hay evidencia de que este ítem específico -- reforzar invariantes de
  dominio para más de un terminal escribiendo a la vez -- se haya hecho. No confundir con
  la sincronización multi-dispositivo actual (cada dispositivo con su propia base +
  detección de conflictos entre sitios, eso sí existe y funciona, ver
  `src/nube/sincronizacion.rs`) -- son cosas distintas.
- [x] **SQLite STRICT aplicado donde corresponde.**
- [x] **Historial y auditoría usan conexiones secundarias para cargas/exportaciones.**

## UX y percepción de velocidad

- [x] **Spinner durante debounce de búsqueda (2026-09-18).** `NuevoIngresoModal.tsx` y
  `GestionGafeteModal.tsx` (los dos únicos buscadores con `DEBOUNCE_MS = 120` explícito)
  agregan un `Loader2` girando -- mismo patrón/CSS `.girando` que ya usaba `BarraNube.tsx`
  para el ícono de sincronizar -- mientras el debounce corre y la búsqueda está en vuelo.
  Ver PR #46.
- [x] **Parpadeo de cursor en formularios: obsoleto (2026-09-18).** La entrada decía
  "Login ya lo tiene; extender el patrón a `ui_kit/text_input.rs`", pero ese módulo (TUI
  clásica) ya no existe -- se eliminó por completo en el commit `a98d7d9`
  ("retira CLI y TUI clásica del crate raíz"). Esta entrada quedó vieja sin actualizarse
  tras esa eliminación; no aplica más.
- [ ] **Confirmación visual breve tras guardar/registrar.** Resaltar fila o elemento recién
  creado/editado para que el cambio no se sienta silencioso.
- [x] **Auditar máscaras de entrada en formularios de escritorio (pedido
  2026-09-17, resuelto 2026-09-18).** `react-hook-form` + `zod` ya se usan en TODOS los
  formularios (`esquema = z.object(...)` es el patrón establecido, ver
  cualquier `Formulario*.tsx`/`*Modal.tsx`), así que la validación de
  ESQUEMA ya existía -- lo que faltaba era la máscara a nivel de INPUT. Pasada completa por
  los 4 campos que usaban `<input type="number">`/texto plano sin filtrar
  (`FormularioGafete.tsx` número/desde/hasta, `IngresoProveedorModal.tsx`,
  `EntregarGafeteProvisionalModal.tsx` y `SalidaRutaModal.tsx` -- confirmado con
  `grep 'type="number"'` que no queda ninguno más en `desktop/src/pantallas`, lo que
  sigue con ese tipo son `<select>` nativos, sin nada que filtrar): ahora todos usan
  `inputMode="numeric"` + filtrado en `onChange` como ya hacía el buscador de Gafetes.
  De paso se encontró y corrigió un bug real: el `defaultValue: NaN` que servía como
  "vacío" con `type="number"` (el DOM lo mostraba en blanco solo) se mostraba literal
  como el string "NaN" en un input de texto -- confirmado montando el input real con una
  prueba descartable antes de tocar el componente. Ver PR #45.
  No se tocó la regla inversa (rechazar dígitos en campos de texto, ej. nombre) -- no se
  encontró ningún caso real de eso en la pasada.
- [x] **Respaldo manual y exportación de historial dejaron de congelar la UI.**
- [x] **Frame de transición entre vistas descartado.** La navegación se conserva inmediata.
- [ ] **Cachear en Cloudflare para `web/` y `web-visitas/` (2026-09-18,
  pedido explícito -- asumir que los visitantes tienen conexión mala).**
  Ambas webs corren en Cloudflare Pages. Capas posibles, de menor a mayor
  esfuerzo:
  1. `Cache-Control` agresivo/inmutable para los assets con hash del build
     de Vite (son cacheables para siempre, cada versión tiene nombre
     distinto) -- gratis, sin riesgo, no está configurado explícitamente
     todavía.
  2. Compresión/optimización de imágenes (Cloudflare Polish) si alguna de
     las dos termina mostrando fotos (cédulas/contratistas).
  3. Cachear en el borde (Cloudflare Worker) sólo las consultas de
     lectura que son iguales para cualquier usuario (ej. catálogo de
     sitios/rutas) -- **nunca** nada que dependa de RLS/sesión, eso
     rompería el modelo de permisos por sitio que ya existe.
  Prioridad: seguridad primero, después velocidad -- no tocar nada de
  RLS/autorización para ganar cache.

## Roadmap fuera del alcance actual

- [ ] **V2: visitas/proveedores y “a quién viene a ver”.**
- [ ] **V2: aviso proactivo de PRAIND por vencer.**
- [ ] **V3: concurrencia multi-terminal.** Dispara revisar agregados de dominio y reglas de
  sincronización local.
- [x] **Permisos granulares por usuario descartados por ahora.** Retomar sólo con un caso
  real que los roles actuales no cubran.
