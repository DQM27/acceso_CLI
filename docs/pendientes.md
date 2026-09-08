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
- [ ] **Revisar bucket público `historial-web`.** Está documentado como público, vacío y
  sin referencias en código. Confirmar si es vestigio; si no se usa, eliminarlo desde
  Supabase.
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
- [ ] **Chequeo cruzado de ingresos abiertos entre sitios.** Con conexión, bloquear el
  segundo ingreso abierto del mismo contratista en otro sitio; offline, registrar y
  alertar luego al sincronizar.
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
estabilizador, clasificador y región de interés.

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
