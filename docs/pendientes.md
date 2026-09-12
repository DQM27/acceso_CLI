# Pendientes consolidados

Documento único de trabajo pendiente del repo. Reemplaza las listas paralelas y notas
viejas de auditoría, nube, móvil, escritorio, OCR, empaquetado y planes de producto.

## Regla del archivo

Al terminar una tarea de aquí, se marca `[x]` en el mismo commit o en el siguiente. Si
una tarea se descarta, también se marca `[x]` con una nota corta de por qué. Los planes
históricos pueden seguir existiendo como contexto, pero esta lista manda.

## Fuentes consolidadas

- `docs/auditoria-calidad-2026-09.md`
- `docs/plan-persistencia-nube.md`
- `docs/plan-panel-administrativo-web.md`
- `docs/plan-ocr-escaneo-documentos.md`
- `docs/fixtures-ocr-sinteticos.md`
- `docs/idea-lector-placas-vehiculares.md`
- `mobile/android/ARQUITECTURA.md`
- `mobile/README.md`
- `docs/plan-sesion-unica-dispositivos.md`
- tracker anterior de escritorio (absorbido aquí; ya no existe como lista aparte)
- `README.md`, `packaging/msix/README.md`, `packaging/alacritty/README.md`,
  `docs/recuperacion-supabase.md`, `docs/realtime-verificado.md`

---

## Seguridad y nube

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
  `docs/plan-sesion-unica-dispositivos.md`. El mismo secreto hoy activa más de un
  dispositivo sin límite. Plan: secreto de un solo uso, identidad canónica del
  dispositivo en Supabase, sesión propia desacoplada del secreto, panel de presencia,
  y regla de desempate por fecha de alta + expulsión automática para conflictos
  detectados offline.
  - [x] Presencia en tiempo real ya funcionando (2026-09-08): Dispositivos.tsx y
    Usuarios.tsx muestran en vivo quién/qué está conectado y desde dónde.
  - [ ] El resto (secreto de un solo uso, identidad canónica, desempate offline)
    sigue sin implementar.
- [ ] **Sesión única por USUARIO (no por dispositivo).** Ver
  `docs/plan-sesion-unica-dispositivos.md`, sección 7. Evitar que la misma cédula
  tenga sesión abierta en dos dispositivos a la vez -- política ya decidida
  (bloquear el login nuevo, no expulsar al viejo), pero el primer diseño (chequear
  contra la presencia del mismo sitio) se descartó: los usuarios son globales, no
  por sitio, así que ese chequeo tiene un hueco entre sitios distintos. Falta
  diseñar un chequeo genuinamente global (probablemente una tabla/lock en Supabase
  con heartbeat, no presence).
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
  `PantallaPrincipal.kt`, comentario de cabecera, y `mobile/android/ARQUITECTURA.md`).
  Ahora se pide devolver puntualmente la de contratista -- con la cédula/DIMEX
  completable por OCR de forma OPCIONAL (ya existe `PantallaEscanearCedula`/
  `ModoEscaneoDocumento` para el escaneo de cédula/gafete en el flujo de ingreso, y todo
  `docs/plan-ocr-escaneo-documentos.md`/`docs/fixtures-ocr-sinteticos.md` como base -- no
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
  prometida -- ver `docs/plan-ocr-escaneo-documentos.md` sección 9.
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
  `mobile/android/ARQUITECTURA.md` fija la regla incremental y ya hay ViewModels reales
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
- [x] **RBAC visual de la GUI corregido.**
- [x] **Auditoría GUI genérica construida.**
- [x] **Respaldos en GUI construidos y revisados.**
- [x] **Exportaciones largas sin bloquear UI/comandos.**
- [x] **Clippy pedantic/nursery cerrado en núcleo y adaptador Tauri.**

## Respaldo, base local y dominio

- [ ] **Eliminar respaldos no usados con confirmación.** Acción menor, mismo patrón que
  exportar/restaurar.
- [x] **Importar respaldo cuando no existe base local omitido a propósito.** La base ya es
  portátil y la restauración técnica existe; no se construye UX previa al primer arranque
  hasta que se pida.
- [x] **Agregados de dominio con constructores privados diferidos a V3.** Los campos
  públicos no violan el flujo actual; reabrir con concurrencia multi-terminal.
- [x] **Respaldo automático a la 01:00 hora Costa Rica.**
- [x] **Retención automática queda en 7 respaldos.**
- [x] **Respaldo previo a migraciones y rollback.**
- [x] **SQLite STRICT aplicado donde corresponde.**
- [x] **Historial y auditoría usan conexiones secundarias para cargas/exportaciones.**

## UX y percepción de velocidad

- [ ] **Spinner durante debounce de búsqueda.** Señal pequeña mientras pasan los 120ms de
  debounce.
- [ ] **Parpadeo de cursor en formularios.** Login ya lo tiene; extender el patrón a
  `ui_kit/text_input.rs`.
- [ ] **Confirmación visual breve tras guardar/registrar.** Resaltar fila o elemento recién
  creado/editado para que el cambio no se sienta silencioso.
- [x] **Respaldo manual y exportación de historial dejaron de congelar la UI.**
- [x] **Frame de transición entre vistas descartado.** La navegación se conserva inmediata.

## Roadmap fuera del alcance actual

- [ ] **V2: visitas/proveedores y “a quién viene a ver”.**
- [ ] **V2: aviso proactivo de PRAIND por vencer.**
- [ ] **V3: concurrencia multi-terminal.** Dispara revisar agregados de dominio y reglas de
  sincronización local.
- [x] **Permisos granulares por usuario descartados por ahora.** Retomar sólo con un caso
  real que los roles actuales no cubran.
