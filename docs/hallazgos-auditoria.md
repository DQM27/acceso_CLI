# Hallazgos de la auditoría de lógica (2026-08-18)

Auditoría dirigida (multi-agente + verificación manual propia) de las 8 áreas planeadas de
la lógica de la app: fachada `AppCore`, servicios, formularios/autenticación de la TUI,
queries/repositorios, núcleo de base de datos (migraciones/conexión/errores/tiempo/
respaldo), orquestación de `app.rs`/`main.rs`, pantallas de listado/búsqueda, y primitivas
compartidas (`ui_kit`) + modelos de dominio. **Auditoría completa.**

Los 67 hallazgos de abajo fueron verificados uno por uno leyendo el código real (uno de los
hallazgos propuestos por un agente resultó ser falso — afirmaba que `verificar_acceso` no
tenía tests, cuando `tests/domain.rs` ya lo cubre con 16 casos — y se descartó). Se van
marcando `[x]` a medida que se reparan, en el orden que se decida trabajarlos (no
necesariamente el orden de esta lista).

**Estado (2026-08-19): 66/67 reparados**, todo en la rama `fix/auditoria-nivel-1`. Sólo
queda pendiente el hallazgo 21 (`Empresa` sin campo `activo`) por decisión explícita del
usuario — es una funcionalidad nueva (migración, cascada, UI), no un fix de código
existente, y ya está marcado V1/prioridad baja. Varios hallazgos resultaron, al investigar
para repararlos, tener más matices de los que el texto original capturaba (documentado en
cada uno): dos casos donde el "problema" era en realidad una decisión de diseño intencional
que se confirmó y documentó en vez de cambiar (10, 16), un caso donde el fix "correcto"
arquitectónicamente se intentó, rompió un test real, y se revirtió con la lección
documentada (18), y un caso donde unificar dos pantallas reveló que en realidad tenían dos
campos con propósitos distintos, no sólo dos nombres para lo mismo (59).

## Orden sugerido para reparar (de lo más delicado a lo más simple)

Índice rápido, sin repetir el detalle de cada hallazgo (ver arriba/abajo el texto completo
de cada uno). Delicado = puede tumbar la app, corromper datos o dejar un hueco de acceso.
Simple = un cambio local, bajo riesgo, sin tocar lógica compartida.

**Nivel 1 — Riesgo real hoy, tocan datos o disponibilidad — [x] reparado (rama `fix/auditoria-nivel-1`)**
1. [x] `unwrap()` en `contratista_id` al registrar ingreso — panic a mitad de un registro real.
2. [x] Salida de emergencia no espera hilos de Argon2 — escritura de usuario se pierde en silencio.
3. [x] Fallo al reabrir la base mata el proceso entero (ignora `mensaje_inicial`).
4. [x] Rollback de restauración fallida puede dejar el sistema peor que al empezar.
5. [x] Respaldo pre-migración corre antes de validar que el archivo es nuestro.
6. [x] `abrir_y_verificar` migra de verdad al "solo verificar" un respaldo.
7. [x] Respaldo renombrado a mano puede borrarse igual por la retención (`starts_with` laxo).

**Nivel 2 — Corrompen decisiones o datos, pero no tumban nada — [x] reparado (rama `fix/auditoria-nivel-1`)**
8. [x] `crear_con_hash`/similares no validan nada — cuenta inutilizable si alguien pasa texto plano.
9. [x] Filtro PRAIND ignora si el tipo de contrato aún lo requiere.
10. [x] **No es bug, es diseño intencional** (Fase 4) — se deja tal cual, ver nota en el detalle abajo.
11. [x] `listar_activos` sin tope de filas — única consulta así en la app.
12. [x] Motivo de denegación con catch-all no exhaustivo (compilador no avisa de variante nueva).
13. [x] Motivo de `PermitidoConAdvertencia` adivinado/hardcodeado fuera del dominio.
14. [x] ROOT inicial se cuelga para siempre si algún día se usa `App::run` sin core.

**Nivel 3 — Deuda estructural (separación de responsabilidades) — [x] reparado 15-20 (rama `fix/auditoria-nivel-1`); 21 pendiente por decisión explícita**
15. [x] `application_id` se adopta antes de verificar integridad del archivo.
16. [x] Regla "último ROOT activo" vive en el repositorio, no en el servicio — confirmado
    intencional (atomicidad), documentado con la prueba que lo exige.
17. [x] `validar_reloj` mezcla SQL crudo en la fachada `AppCore` — SQL movido a
    `database::queries::ingresos::ultimo_instante_movimiento`.
18. [x] `RelojRetrocedido` declarado en el servicio pero generado en `AppCore` — se intentó
    mover al servicio, rompió un test de integración real, y se revirtió; queda documentado
    como decisión intencional (comprobación de sistema, no regla de negocio puntual).
19. [x] Empresas no valida su formulario; Contratistas sí — reparado, ahora valida igual.
20. [x] `VERSION_REGLAS_ACCESO` vive lejos de las reglas que versiona — movida a
    `domain::acceso`, re-exportada desde `models::registro_ingreso` para no romper imports.
21. **Pendiente, por decisión explícita del usuario** (no es fix de código existente, es
    funcionalidad nueva): `Empresa` sin campo `activo` — baja de empresa no revoca acceso de
    sus contratistas (V1, prioridad baja, caso extremo).

**Nivel 4 — Duplicación / código muerto (afecta mantenibilidad, no producción) — [x] reparado (rama `fix/auditoria-nivel-1`)**
22. [x] Bloque de spawn de hilo para hashear duplicado 3 veces.
23. [x] Mapeo `TipoIngreso` ↔ texto SQL duplicado ~6 veces en 4 archivos (+ duplicado interno en
    `contratista_repository.rs`).
24. [x] `tipo_desde_texto` triplicado en 3 pantallas.
25. [x] Bucle de términos `clave:valor` duplicado en 3 pantallas (falta el helper).
26. [x] Verificación de login duplicada entre servicio y TUI.
27. [x] Chequeo UNIQUE duplicado en 3 servicios.
28. [x] `mover()` duplicada letra por letra en 2 pantallas.
29. [x] Bloque de transacción `Immediate` + `validar_reloj` duplicado 2 veces.
30. [x] `aplicar_retencion` aborta a mitad de camino, error descartado en ambos callers.
31. [x] Limpieza de sentinelas / `.partial` inválido descartan su propio error (2 hallazgos).
32. [x] `take().unwrap()` frágil repetido en los 3 flujos con hilo.
33. [x] Migración 6 carga toda la tabla en memoria (patrón riesgoso a futuro) — sólo
    documentado, no se puede reescribir sin riesgo (una migración ya publicada no se toca).
34. [x] `UsuarioService::actualizar` es código muerto.
35. [x] `SelectMenu`/`SelectMenuState` sin ningún consumidor real.

**Nivel 5 — Magia / números hardcodeados (arreglo mecánico, bajo riesgo) — [x] reparado (rama `fix/auditoria-nivel-1`)**
36. [x] `guardando` compartido como mutex implícito entre 2 estados.
37. [x] `"Quintana"` hardcodeado como fallback sin sesión + orden de sondeo inconsistente.
38. [x] Fallo de invariante de migración se ve como error genérico de SQLite.
39. [x] `AppCore::new`/`con_reloj` dejan `ruta_base_datos` vacía sin avisar.
40. [x] Tope de 4 en `tipos_incluidos` repetido en 3 sitios, no derivado de `TipoIngreso::ALL`.
41. [x] Tope del selector de rol hardcodeado (`.min(2)`).
42. [x] Error de fecha corrupta disfrazado de error de SQLite.
43. [x] Tope del desplegable "Tipo" hardcodeado (`3`).
44. [x] `unwrap()` en fecha por defecto de `FiltrosHistorial`.
45. [x] `StandardCommand::Activate` sin consumidor real.
46. [x] Matriz `requiere_praind()`/`requiere_gafete()` sin comentario que explique la regla.
47. [x] `fecha_hora_salida`/`usuario_salida_id` deberían ser un único `Option`.

**Nivel 6 — Coherencia general / entre pantallas (cosméticos, sin riesgo de datos) — [x] reparado (rama `fix/auditoria-nivel-1`)**
48. [x] Respaldo "obligatorio" se salta si la ruta no tiene directorio padre.
49. [x] `DatabaseError` no expone `source()` (a diferencia de `SchemaError`).
50. [x] `StandardCommand::FocusNext`/`Primary` casi no se usan — documentado, no se fuerza
    adopción (cada pantalla tiene su propia noción de "siguiente campo").
51. [x] `query_lang` expuesto como `pub mod` completo, rompe convención de `ui_kit`.
52. [x] Búsqueda se limpia al guardar usuario pero no al activar/desactivar.
53. [x] Password ROOT inicial valida orden distinto que Usuarios (coincidencia vs. longitud).
54. [x] ROOT inicial sin tope de longitud en cédula/nombre/password (otros formularios sí).
55. [x] Doble `unwrap()` innecesario en `emitir_guardado`.
56. [x] Mensaje de error persiste tras búsqueda exitosa en Nuevo Ingreso.
57. [x] Cualquier tecla cierra confirmación de Salida Rápida (no sólo Enter/Esc).
58. [x] Salida Rápida no soporta el toggle de ayuda (F1).
59. [x] Campo de error de búsqueda con nombre distinto entre pantallas — de paso se encontró
    y corrigió una colisión de nombre de campo real al unificar (`mensaje` duplicado en
    Contratistas/Empresas, dos conceptos distintos que había que fusionar con cuidado, no
    un simple renombre).
60. [x] Historial no avisa al no poder ocultar la última columna visible.
61. [x] Política de "limpiar filtro tras guardar" distinta en cada pantalla.
62. [x] Empresas resetea selección en cada tecla; las otras 3 pantallas no.
63. [x] Patrón irrefutable asume única variante de `TermValue` (depende de versión pineada).

Los ítems que no cuentan un número aquí (Nivel 1-6 cubre 63 de 67 — el resto son notas
"Relacionado" ya incluidas dentro del punto que las menciona, no hallazgos aparte) están
listados completos en las secciones de abajo.

## Impacto directo

- [x] **`unwrap()` sobre `contratista_id` en el camino que registra un ingreso real.**
  `src/tui/nuevo_ingreso/state.rs:220`. Depende de una relación no forzada por el tipo
  (`etapa == Formulario` implica `contratista_id == Some`). Es el único `unwrap()` de las
  pantallas operativas — un panic ahí tumba la TUI completa a mitad de un registro, con
  alguien físicamente parado en la puerta.
- [x] **La salida de emergencia no espera los hilos de Argon2 pendientes.**
  `src/tui/app.rs:1339`. Si se dispara crear-usuario/cambiar-contraseña/ROOT-inicial/login
  y antes de que llegue el resultado el operador presiona la salida de emergencia, el hilo
  sigue corriendo pero su resultado cae en un canal sin receptor — la escritura real en
  SQLite nunca ocurre, sin ningún error ni aviso.
- [x] **Un fallo al reabrir la base mata todo el proceso en vez de mostrarlo en la TUI.**
  `src/main.rs:35`. Si `AppCore::abrir` falla en cualquier vuelta del bucle (incluida la
  que sigue a una restauración fallida), el proceso entero termina con un `eprintln`
  crudo, ignorando el mecanismo `mensaje_inicial` que la Fase 3b introdujo justo para esto.
- [x] **ROOT inicial se cuelga para siempre en modo sin base de datos.**
  `src/tui/app.rs:1264`. A diferencia de los otros 3 flujos con threading, no manejaba el
  caso `core: None`. **Reparado:** `App::run` ahora sí tiene un llamador real
  (`terminal::run_sin_core`, agregado al reparar el hallazgo de Nivel 1 "un fallo al
  reabrir la base mata todo el proceso") — se agregó `abortar_configuracion_inicial_sin_core`
  para que, sin `core`, la solicitud pendiente se resuelva con un error visible en vez de
  quedar congelada en "Creando" para siempre.
- [x] **El respaldo obligatorio pre-migración corre antes de validar que el archivo es
  nuestro.** `src/database/connection.rs:119`. `open_database` llama
  `respaldar_antes_de_migrar` antes de `initialize_database`, pero el rechazo de bases
  ajenas (`application_id`) vive dentro de `initialize_database`. Un archivo de otra app
  con `user_version` entre 1 y 5 por casualidad puede terminar copiado a `backups/` antes
  de ser rechazado.
- [x] **`abrir_y_verificar` migra el esquema de verdad, no solo "verifica".**
  `src/database/backup.rs:261`. Restaurar un respaldo viejo puede terminar aplicándole
  migraciones y persistiéndolas, un efecto secundario que el rustdoc público de
  `restaurar_respaldo` nunca menciona.
- [x] **El rollback de una restauración fallida puede dejar el sistema peor que al
  empezar.** `src/database/backup.rs:251`. Si el intento de reinstalar la base anterior
  también falla (ambos pasos descartan su error con `let _ =`), la base activa queda con
  la candidata rota y la base buena queda huérfana en un archivo sentinela — sin que el
  error devuelto lo distinga de un rollback exitoso.
- [x] **Un respaldo renombrado a mano para protegerlo puede ser borrado igual por la
  retención.** `src/database/backup.rs:348`. `interpretar_nombre` matchea el tipo con
  `starts_with` sin exigir que lo que sigue sea el sufijo numérico documentado —
  `"automatico_no_borrar".starts_with("automatico_")` es `true`.
- [x] **`crear_con_hash`/`cambiar_password_con_hash`/`crear_root_inicial_con_hash` no
  validan nada.** `src/services/usuario_service.rs:91`. Persisten el `password_hash`
  recibido tal cual (ni siquiera comprueban que sea un hash Argon2 válido), y
  `crear_con_hash` tampoco repite el chequeo `ConfiguracionInicialRequerida`. Un caller
  futuro que pase el password en texto plano por error deja esa cuenta inutilizable en el
  login sin ningún aviso al guardar.
- [x] **Filtro "PRAIND vencido/próximo a vencer" ignora si el tipo de contrato aún lo
  requiere.** `src/database/queries/contratistas.rs:108`. No aplica la condición
  equivalente a `Contratista::requiere_praind()`. Un contratista que cambió a un tipo que
  ya no requiere PRAIND pero conserva una fecha vieja sin limpiar sigue apareciendo como
  "vencido" aunque el sistema real le da acceso permitido sin advertencia.
- [x] **Fallo al limpiar respaldos automáticos viejos se descarta en silencio.**
  `src/application.rs:386`. `let _ =` sobre `aplicar_retencion`; si falla una vez queda
  roto para siempre sin aviso y los respaldos se acumulan sin límite.
  Nota: toda la función `respaldo_automatico_diario_si_hace_falta` traga sus errores
  (no sólo la retención) — esto es **por diseño**, ya decidido explícitamente en la Fase 4
  ("ignorar en silencio si falla, no es obligatorio"). No tratar como bug salvo que se
  quiera reconsiderar esa decisión. **No se tocó código** — se marca resuelto porque ya
  estaba decidido, no porque se haya cambiado el comportamiento.
- [x] **`listar_activos` (Ingresos Activos) es la única consulta de toda la app sin tope de
  filas.** `src/database/queries/ingresos.rs:33`. `FiltroIngresosActivos` no tiene
  límite/offset y `ACTIVOS_SQL` no lleva `LIMIT`.
- [x] **Mapeo de motivo de denegación con catch-all no exhaustivo.**
  `src/tui/nuevo_ingreso/state.rs:306`. `mensaje_bloqueo` reimplementa en la TUI el
  conocimiento de qué `MotivoDenegacion` existen; una variante nueva cae en un mensaje
  genérico sin que el compilador avise, justo en la pantalla que decide si alguien entra.
- [ ] **`Empresa` no tiene campo `activo` — dar de baja una empresa no revoca el acceso de
  sus contratistas.** `src/models/empresa.rs:1`. No existe ningún camino para "desactivar"
  una empresa; sus contratistas siguen con `tiene_acceso = true` de forma independiente.
  Caso extremo pero real (empresa que deja de operar sigue entrando gente a su nombre).
  Alcance V1, prioridad baja — no bloquea nada hoy, confirmado explícitamente por decisión
  del usuario (ver `project_roadmap_v1_v2` en memoria).
- [x] **El motivo de `PermitidoConAdvertencia` se adivina fuera del dominio.**
  `src/database/repositories/registro_ingreso_repository.rs:148`. `ResultadoAcceso` no
  transporta por qué se disparó la advertencia; el repositorio hardcodea la suposición
  `=> "PRAIND_PROXIMO_VENCER"`. Si se agrega una segunda regla de advertencia no
  relacionada con PRAIND, todos esos ingresos quedarían con el motivo incorrecto en la base.

## Separación de responsabilidades

- [x] **Se adopta el `application_id` antes de verificar integridad.**
  `src/database/schema.rs:77`. **Reparado:** `rechazar_archivo_ajeno` (solo lectura) corre
  antes de `verificar_integridad_rapida`; `adoptar_application_id` (la escritura) corre
  después, sólo si el `quick_check` pasó.
- [x] **La regla del "último ROOT activo" vive en el repositorio, no en el servicio.**
  `src/database/repositories/usuario_repository.rs:121`. **Confirmado intencional, no se
  mueve:** documentado con comentario explícito — la lectura del conteo y la escritura
  comparten la misma transacción `Immediate` a propósito, para la misma prevención de
  condición de carrera que `crear_root_inicial_atomico`; moverlo al servicio la reabriría
  (`dos_conexiones_no_pueden_desactivar_ambos_roots` es la prueba que lo exige).
- [x] **`validar_reloj` mezcla SQL crudo y regla de negocio directo en la fachada.**
  `src/application.rs:431`. **Reparado:** el SQL se movió a
  `database::queries::ingresos::ultimo_instante_movimiento`; `validar_reloj` en
  `application.rs` quedó como regla pura sin SQL propio.
- [x] **`RegistroIngresoServiceError::RelojRetrocedido` se declara en el servicio pero
  nunca lo genera.** `src/services/error.rs:201`. **Se intentó mover a
  `RegistroIngresoService`** (repositorio con nuevo método `ultimo_instante_movimiento`) —
  rompió `tests/flujo_integracion.rs` porque ese test llama al servicio directo con datos de
  prueba cuyos tiempos no representan un reloj real avanzando. Revertido; queda documentado
  como decisión intencional (comprobación de sanidad de todo el sistema, no una regla de
  negocio de una entrada/salida puntual).
- [x] **Empresas no valida el formulario; Contratistas sí.**
  `src/tui/empresas/state.rs:283`. **Reparado:** nueva función `construir` en
  `empresas/state.rs` (mismo patrón que `contratistas::construir`) que valida y recorta el
  nombre antes de despachar, con test nuevo para el caso vacío.
- [x] **`VERSION_REGLAS_ACCESO` vive lejos de las reglas que versiona.**
  `src/models/registro_ingreso.rs:6`. **Reparado:** movida a `domain::acceso`, junto a
  `DIAS_ADVERTENCIA_PRAIND` y las 5 reglas; re-exportada desde `models::registro_ingreso`
  para no romper los imports existentes.

## Deuda técnica / duplicación

- [x] **Bloque de spawn de hilo para hashear duplicado literalmente 3 veces.**
  `src/tui/app.rs:1185` (`iniciar_creacion_usuario`, `iniciar_cambio_password`,
  `iniciar_root_inicial`). **Reparado:** extraído a `App::generar_hash_en_hilo`.
- [x] **`aplicar_retencion` aborta a mitad de camino y el fallo se pierde en ambos
  callers.** `src/database/backup.rs:382`. **Reparado:** ahora es best-effort por archivo
  (sigue borrando el resto de sobrantes aunque uno esté bloqueado).
- [x] **Limpieza de sentinelas de una restauración anterior descarta su error.**
  `src/database/backup.rs:223`. **Reparado:** ahora propaga el error si el sentinela existe
  y no se puede borrar (sólo se ignora `NotFound`, el caso normal).
- [x] **Limpieza de un `.partial` inválido descarta su propio error.**
  `src/database/backup.rs:179`. **Reparado:** nueva variante `RespaldoError::LimpiezaFallida`
  que adjunta el error original en vez de descartar el fallo de limpieza.
- [x] **Patrón `take().unwrap()` frágil repetido en los 3 flujos con hilo.**
  `src/tui/app.rs:1200` (y `:1248`, `:1288`). **Reparado:** los 3 flujos usan un único
  `match .take() { Some(...) => ..., None => ... }` (2 de los 3 ya habían quedado así al
  consolidar `hilo_usuario_pendiente` en el Nivel 5; el de ROOT inicial se corrigió igual).
- [x] **Migración 6 carga toda la tabla `registro_ingresos` en memoria.**
  `src/database/schema.rs:154`. **No se toca la lógica** (una migración ya publicada no se
  reescribe) — se agregó un comentario de advertencia explícito para no repetir el patrón
  en una migración futura sobre una tabla más grande.
- [x] **`UsuarioService::actualizar` es código muerto.** `src/services/usuario_service.rs:126`.
  **Reparado:** eliminado junto con `UsuarioRepository::actualizar_identidad_y_rol` (que
  sólo él llamaba). Tenía más cobertura de la que el hallazgo original detectó — no sólo
  sus propios tests, sino 7 casos en `tests/root_inicial.rs` que ejercitan la protección
  del último ROOT activo — migrados a `actualizar_administracion` (el camino real de
  producción) en vez de perderse.
- [x] **Chequeo de restricción UNIQUE duplicado igual en tres servicios.**
  `src/services/usuario_service.rs:278` (y `contratista_service.rs`, `empresa_service.rs`).
  **Reparado:** movido a `DatabaseError::es_constraint_unique`.
- [x] **Mapeo `TipoIngreso` ↔ texto SQL duplicado ~6 veces en 4 archivos.**
  `src/database/repositories/contratista_repository.rs:33` (y `queries/contratistas.rs`,
  `registro_ingreso_repository.rs`, `queries/ingresos.rs`). **Reparado:**
  `TipoIngreso::as_str_sql`/`from_str_sql` únicos, usados en los 4 archivos.
- [x] **Bloque de transacción `Immediate` + `validar_reloj` duplicado literalmente dos
  veces.** `src/application.rs:217` (`registrar_ingreso` y `registrar_salida`). **Reparado:**
  extraído a `AppCore::en_transaccion_con_reloj_validado`.
- [x] **Verificación de login duplicada entre el servicio y la TUI.**
  `src/services/autenticacion_service.rs:56`. **Reparado:** extraído a la función libre
  `verificar_candidato`, usada tanto por `autenticar()` como por el hilo de `app.rs`.
- [x] **`mover()` duplicada letra por letra entre dos pantallas.**
  `src/tui/nuevo_ingreso/state.rs:243` y `src/tui/salida_rapida/state.rs:139`. **Reparado:**
  extraído a `ui_kit::mover_seleccion`.
- [x] **`tipo_desde_texto` triplicado en 3 archivos.** `src/tui/activos/state.rs:94` (y
  `contratistas/state.rs`, `historial/filtros.rs`). **Reparado:** movido a
  `TipoIngreso::from_str_filtro`.
- [x] **`TipoIngreso` sin `as_str`/`FromStr`, duplicado hasta dentro de un mismo archivo.**
  `src/database/repositories/contratista_repository.rs:81`. **Reparado:** mismo fix que el
  mapeo SQL de arriba (`as_str_sql`/`from_str_sql`).
- [x] **`SelectMenu`/`SelectMenuState` no tiene ningún consumidor real.**
  `src/tui/ui_kit/select_menu.rs`. 149 líneas con navegación y tests, sin ningún caso de
  uso en producción ni en `examples/`. **Reparado:** módulo eliminado y su re-export en
  `ui_kit/mod.rs` quitado.
- [x] **Bucle orquestador de términos `clave:valor` duplicado en 3 pantallas.**
  `src/tui/ui_kit/query_lang.rs` (falta el helper) — el bucle que separa términos libres de
  términos con clave se repite literal en `activos`, `contratistas` e `historial/filtros.rs`
  en vez de vivir una sola vez en `query_lang.rs`. **Reparado:** extraído a
  `query_lang::resolver_terminos`.

## Magia / números y comportamientos implícitos

- [x] **`guardando` compartido como mutex implícito entre 2 estados pendientes.**
  `src/tui/app.rs:1190`. **Reparado:** `creacion_usuario_pendiente`/`cambio_password_pendiente`
  se consolidaron en un único `Option<HiloUsuarioPendiente>` (enum de 2 variantes) — la
  exclusión mutua ahora es estructural, no depende de que nada valide un booleano en otro
  archivo.
- [x] **Nombre `"Quintana"` hardcodeado como fallback sin sesión.**
  `src/tui/app.rs:1484`. **Reparado:** el fallback pasó a `"Usuario desconocido"` con un
  comentario explicando que es puramente defensivo (el menú no es alcanzable sin sesión).
  Relacionado — orden de sondeo inconsistente: **reparado**, los 4 sondeos de hilos de
  Argon2 (login, ROOT inicial, crear usuario/cambiar contraseña) ahora corren siempre en
  el mismo punto del bucle, después de leer teclas.
- [x] **Fallo de invariante de migración se ve como error genérico de SQLite.**
  `src/database/schema.rs:113`. **Reparado:** nueva variante
  `SchemaError::VersionInesperadaTrasMigrar { encontrada }` en vez de un
  `rusqlite::Error::InvalidQuery` fabricado a mano.
- [x] **`AppCore::new`/`con_reloj` dejan `ruta_base_datos` vacía sin avisar en la firma
  pública.** `src/application.rs:81`. **Reparado:** rustdoc explícito en ambos
  constructores documentando la consecuencia sobre `directorio_respaldos()`.
- [x] **Tope de 4 en `tipos_incluidos` trunca en silencio, repetido en 3 sitios.**
  `src/database/queries/ingresos.rs:160` (y `buscar_historial`, `ContratistasQuery::buscar`).
  **Reparado:** los 3 sitios ahora derivan el tamaño del array de `TipoIngreso::ALL.len()`
  en vez del literal `4`.
- [x] **Tope del selector de rol hardcodeado (`.min(2)`) en vez de `ROLES.len()`.**
  `src/tui/usuarios/state.rs:495`. **Reparado:** ahora usa `ROLES.len() - 1`.
- [x] **Error de fecha corrupta se disfraza de error de SQLite.** `src/application.rs:450`.
  **Reparado:** nueva variante `DatabaseError::FechaCorrupta(String)` en vez de un
  `FromSqlConversionFailure` fabricado a mano.
- [x] **Tope del desplegable "Tipo" hardcodeado en vez de `tipos().len()`.**
  `src/tui/contratistas/state.rs:486`. **Reparado:** ahora usa `tipos().len().saturating_sub(1)`.
- [x] **`unwrap()` en la fecha por defecto de `FiltrosHistorial`.**
  `src/tui/historial/filtros.rs:27`. **Reparado:** `unwrap_or(h)` en vez de `unwrap()` — sin
  panic posible aunque el día usado cambiara en el futuro.
- [x] **`StandardCommand::Activate` no tiene ningún consumidor real.**
  `src/tui/ui_kit/keyboard.rs:52`. **Reparado:** variante eliminada (Espacio ya no se
  intercepta como comando transversal; cada pantalla lo maneja en su propio match, como ya
  hacía).
- [x] **Matriz `requiere_praind()`/`requiere_gafete()` sin explicar la regla de negocio.**
  `src/models/contratista.rs:18`. **Reparado:** rustdoc que remite a la tabla "Reglas para
  PRAIND y gafete" ya documentada en `docs/diagrama-logico.md`.
- [x] **`fecha_hora_salida`/`usuario_salida_id` son 2 `Option` independientes.**
  `src/models/registro_ingreso.rs:65`. **Reparado:** consolidados en
  `salida: Option<SalidaRegistroIngreso { fecha_hora, usuario_id }>` — ya no se puede
  construir un registro "con salida sin usuario" o viceversa.

## Coherencia

- [x] **El respaldo "obligatorio" pre-migración se salta si la ruta no tiene directorio
  padre.** `src/database/connection.rs:135`. **Reparado:** ahora devuelve
  `SchemaError::RespaldoPreMigracionFallido` en vez de `Ok(())` silencioso.
- [x] **`DatabaseError` no expone la causa real vía `source()`.**
  `src/database/error.rs:34`. **Reparado:** `source()` implementado, reenvía la causa de
  `Sqlite(_)`.
- [x] **`StandardCommand::FocusNext`/`Primary` casi no se usan.**
  `src/tui/ui_kit/keyboard.rs`. **Documentado, no forzado:** rustdoc en el enum aclara la
  adopción real — no es un bug, cada pantalla tiene su propia noción de "siguiente campo".
- [x] **`query_lang` es el único submódulo de `ui_kit` expuesto como `pub mod` completo.**
  `src/tui/ui_kit/mod.rs:9`. **Reparado:** ahora es `mod` privado con `pub use
  query_lang::{Term, resolver_terminos, valores}`, igual que el resto de `ui_kit`.

## Coherencia entre pantallas

- [x] **La búsqueda se limpia al guardar un usuario pero no al activar/desactivarlo.**
  `src/tui/usuarios/state.rs:347` vs `:311`. **Reparado:** `completar_estado` ahora también
  limpia el filtro.
- [x] **Password de ROOT inicial valida coincidencia antes que longitud; Usuarios lo hace al
  revés.** `src/tui/configuracion_inicial/state.rs:225` vs `usuarios/state.rs`. **Reparado:**
  mismo orden que Usuarios (longitud antes que coincidencia).
- [x] **ROOT inicial es la única cuenta sin tope de longitud en cédula/nombre/password.**
  `src/tui/configuracion_inicial/state.rs:193`. **Reparado:** mismos topes que Usuarios (30
  cédula, 60 nombre, 128 password).
- [x] **Doble `unwrap()` innecesario en `emitir_guardado` tras mover el formulario.**
  `src/tui/usuarios/state.rs:587`. **Reparado:** `rol`/`activo` se capturan antes de mover
  `f`; se eliminó `formulario_actual()`, que quedó sin otro uso (código muerto).
- [x] **Mensaje de error persiste tras una búsqueda exitosa en Nuevo Ingreso.**
  `src/tui/nuevo_ingreso/state.rs:92`. **Reparado:** `completar_busqueda` limpia
  `self.error` en la rama `Ok`, igual que Salida Rápida.
- [x] **Cualquier tecla cierra la confirmación de Salida Rápida, no sólo Enter/Esc.**
  `src/tui/salida_rapida/state.rs:88`. **Reparado:** sólo Enter/Esc, igual que
  `menu_principal`.
- [x] **Salida Rápida no soporta el toggle de ayuda (F1).**
  `src/tui/salida_rapida/state.rs:1`. **Reparado:** F1 alterna `ayuda_expandida`, que
  extiende el pie con el hint de F2.
- [x] **Campo de error de búsqueda distinto entre pantallas (`mensaje` vs `error_carga`).**
  `src/tui/activos/state.rs:264`. **Reparado:** unificado a `mensaje` en las 4 pantallas —
  Contratistas/Empresas tenían en realidad *dos* campos (`mensaje` para éxito,
  `error_carga` para fallo de carga); se fusionaron en uno solo con el mismo criterio por
  contenido (`starts_with('✓')`) que ya usaba Activos.
- [x] **Historial no avisa al no poder ocultar la última columna visible.**
  `src/tui/historial/state.rs:334`. **Reparado:** mismo mensaje que Activos/Contratistas.
- [x] **Política de "limpiar filtro tras guardar" distinta en cada pantalla.**
  `src/tui/contratistas/state.rs:601`. **Reparado:** Contratistas y Empresas limpian tanto
  al crear como al editar; Activos limpia tras registrar una salida — mismo criterio que ya
  se fijó para Usuarios.
- [x] **Empresas resetea la selección en cada tecla de búsqueda; las otras 3 no.**
  `src/tui/empresas/state.rs:238`. **Reparado:** ya no resetea `seleccion` en
  Backspace/Char, igual que las otras 3.
- [x] **Patrón irrefutable asume una única variante de `TermValue`.**
  `src/tui/ui_kit/query_lang.rs:33`. **Reparado:** consolidado en un único helper privado
  `valor_simple` con el comentario que documenta la dependencia de `query-parser 0.2.x`, en
  vez de repetir el patrón (y el supuesto sin explicar) en 3 funciones.
