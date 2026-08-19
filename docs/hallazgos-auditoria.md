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

**Nivel 2 — Corrompen decisiones o datos, pero no tumban nada**
8. `crear_con_hash`/similares no validan nada — cuenta inutilizable si alguien pasa texto plano.
9. Filtro PRAIND ignora si el tipo de contrato aún lo requiere.
10. Fallo al limpiar respaldos viejos se descarta en silencio (acumulación sin límite).
11. `listar_activos` sin tope de filas — única consulta así en la app.
12. Motivo de denegación con catch-all no exhaustivo (compilador no avisa de variante nueva).
13. Motivo de `PermitidoConAdvertencia` adivinado/hardcodeado fuera del dominio.
14. ROOT inicial se cuelga para siempre si algún día se usa `App::run` sin core.

**Nivel 3 — Deuda estructural (separación de responsabilidades)**
15. `application_id` se adopta antes de verificar integridad del archivo.
16. Regla "último ROOT activo" vive en el repositorio, no en el servicio.
17. `validar_reloj` mezcla SQL crudo en la fachada `AppCore`.
18. `RelojRetrocedido` declarado en el servicio pero generado en `AppCore`.
19. Empresas no valida su formulario; Contratistas sí.
20. `VERSION_REGLAS_ACCESO` vive lejos de las reglas que versiona.
21. **Nuevo:** `Empresa` sin campo `activo` — baja de empresa no revoca acceso de sus
    contratistas (V1, prioridad baja, caso extremo).

**Nivel 4 — Duplicación / código muerto (afecta mantenibilidad, no producción)**
22. Bloque de spawn de hilo para hashear duplicado 3 veces.
23. Mapeo `TipoIngreso` ↔ texto SQL duplicado ~6 veces en 4 archivos (+ duplicado interno en
    `contratista_repository.rs`).
24. `tipo_desde_texto` triplicado en 3 pantallas.
25. Bucle de términos `clave:valor` duplicado en 3 pantallas (falta el helper).
26. Verificación de login duplicada entre servicio y TUI.
27. Chequeo UNIQUE duplicado en 3 servicios.
28. `mover()` duplicada letra por letra en 2 pantallas.
29. Bloque de transacción `Immediate` + `validar_reloj` duplicado 2 veces.
30. `aplicar_retencion` aborta a mitad de camino, error descartado en ambos callers.
31. Limpieza de sentinelas / `.partial` inválido descartan su propio error (2 hallazgos).
32. `take().unwrap()` frágil repetido en los 3 flujos con hilo.
33. Migración 6 carga toda la tabla en memoria (patrón riesgoso a futuro).
34. `UsuarioService::actualizar` es código muerto.
35. `SelectMenu`/`SelectMenuState` sin ningún consumidor real.

**Nivel 5 — Magia / números hardcodeados (arreglo mecánico, bajo riesgo)**
36. `guardando` compartido como mutex implícito entre 2 estados.
37. `"Quintana"` hardcodeado como fallback sin sesión + orden de sondeo inconsistente.
38. Fallo de invariante de migración se ve como error genérico de SQLite.
39. `AppCore::new`/`con_reloj` dejan `ruta_base_datos` vacía sin avisar.
40. Tope de 4 en `tipos_incluidos` repetido en 3 sitios, no derivado de `TipoIngreso::ALL`.
41. Tope del selector de rol hardcodeado (`.min(2)`).
42. Error de fecha corrupta disfrazado de error de SQLite.
43. Tope del desplegable "Tipo" hardcodeado (`3`).
44. `unwrap()` en fecha por defecto de `FiltrosHistorial`.
45. `StandardCommand::Activate` sin consumidor real.
46. Matriz `requiere_praind()`/`requiere_gafete()` sin comentario que explique la regla.
47. `fecha_hora_salida`/`usuario_salida_id` deberían ser un único `Option`.

**Nivel 6 — Coherencia general / entre pantallas (cosméticos, sin riesgo de datos)**
48. Respaldo "obligatorio" se salta si la ruta no tiene directorio padre.
49. `DatabaseError` no expone `source()` (a diferencia de `SchemaError`).
50. `StandardCommand::FocusNext`/`Primary` casi no se usan.
51. `query_lang` expuesto como `pub mod` completo, rompe convención de `ui_kit`.
52. Búsqueda se limpia al guardar usuario pero no al activar/desactivar.
53. Password ROOT inicial valida orden distinto que Usuarios (coincidencia vs. longitud).
54. ROOT inicial sin tope de longitud en cédula/nombre/password (otros formularios sí).
55. Doble `unwrap()` innecesario en `emitir_guardado`.
56. Mensaje de error persiste tras búsqueda exitosa en Nuevo Ingreso.
57. Cualquier tecla cierra confirmación de Salida Rápida (no sólo Enter/Esc).
58. Salida Rápida no soporta el toggle de ayuda (F1).
59. Campo de error de búsqueda con nombre distinto entre pantallas.
60. Historial no avisa al no poder ocultar la última columna visible.
61. Política de "limpiar filtro tras guardar" distinta en cada pantalla.
62. Empresas resetea selección en cada tecla; las otras 3 pantallas no.
63. Patrón irrefutable asume única variante de `TermValue` (depende de versión pineada).

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
- [ ] **ROOT inicial se cuelga para siempre en modo sin base de datos.**
  `src/tui/app.rs:1264`. A diferencia de los otros 3 flujos con threading, no maneja el
  caso `core: None` — si algún día se usa `App::run` (público, sin core), el formulario
  queda congelado en "Creando" sin salida.
  Relacionado: `App::run` (la variante pública sin core) no tiene ningún llamador real hoy
  (`src/tui/app.rs:1005`) — es la razón de que este hallazgo sea hoy inofensivo.
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
- [ ] **`crear_con_hash`/`cambiar_password_con_hash`/`crear_root_inicial_con_hash` no
  validan nada.** `src/services/usuario_service.rs:91`. Persisten el `password_hash`
  recibido tal cual (ni siquiera comprueban que sea un hash Argon2 válido), y
  `crear_con_hash` tampoco repite el chequeo `ConfiguracionInicialRequerida`. Un caller
  futuro que pase el password en texto plano por error deja esa cuenta inutilizable en el
  login sin ningún aviso al guardar.
- [ ] **Filtro "PRAIND vencido/próximo a vencer" ignora si el tipo de contrato aún lo
  requiere.** `src/database/queries/contratistas.rs:108`. No aplica la condición
  equivalente a `Contratista::requiere_praind()`. Un contratista que cambió a un tipo que
  ya no requiere PRAIND pero conserva una fecha vieja sin limpiar sigue apareciendo como
  "vencido" aunque el sistema real le da acceso permitido sin advertencia.
- [ ] **Fallo al limpiar respaldos automáticos viejos se descarta en silencio.**
  `src/application.rs:386`. `let _ =` sobre `aplicar_retencion`; si falla una vez queda
  roto para siempre sin aviso y los respaldos se acumulan sin límite.
  Nota: toda la función `respaldo_automatico_diario_si_hace_falta` traga sus errores
  (no sólo la retención) — esto es **por diseño**, ya decidido explícitamente en la Fase 4
  ("ignorar en silencio si falla, no es obligatorio"). No tratar como bug salvo que se
  quiera reconsiderar esa decisión.
- [ ] **`listar_activos` (Ingresos Activos) es la única consulta de toda la app sin tope de
  filas.** `src/database/queries/ingresos.rs:33`. `FiltroIngresosActivos` no tiene
  límite/offset y `ACTIVOS_SQL` no lleva `LIMIT`.
- [ ] **Mapeo de motivo de denegación con catch-all no exhaustivo.**
  `src/tui/nuevo_ingreso/state.rs:306`. `mensaje_bloqueo` reimplementa en la TUI el
  conocimiento de qué `MotivoDenegacion` existen; una variante nueva cae en un mensaje
  genérico sin que el compilador avise, justo en la pantalla que decide si alguien entra.
- [ ] **`Empresa` no tiene campo `activo` — dar de baja una empresa no revoca el acceso de
  sus contratistas.** `src/models/empresa.rs:1`. No existe ningún camino para "desactivar"
  una empresa; sus contratistas siguen con `tiene_acceso = true` de forma independiente.
  Caso extremo pero real (empresa que deja de operar sigue entrando gente a su nombre).
  Alcance V1, prioridad baja — no bloquea nada hoy, confirmado explícitamente por decisión
  del usuario (ver `project_roadmap_v1_v2` en memoria).
- [ ] **El motivo de `PermitidoConAdvertencia` se adivina fuera del dominio.**
  `src/database/repositories/registro_ingreso_repository.rs:148`. `ResultadoAcceso` no
  transporta por qué se disparó la advertencia; el repositorio hardcodea la suposición
  `=> "PRAIND_PROXIMO_VENCER"`. Si se agrega una segunda regla de advertencia no
  relacionada con PRAIND, todos esos ingresos quedarían con el motivo incorrecto en la base.

## Separación de responsabilidades

- [ ] **Se adopta el `application_id` antes de verificar integridad.**
  `src/database/schema.rs:77`. `verificar_identidad_de_archivo` escribe el "sello" de la
  app antes de que `verificar_integridad_rapida` corra el `quick_check`. Si la escritura
  se completa pero el `quick_check` falla justo después, un archivo dañado o ajeno queda
  "adoptado" como propio para siempre.
- [ ] **La regla del "último ROOT activo" vive en el repositorio, no en el servicio.**
  `src/database/repositories/usuario_repository.rs:121`. `SqliteUsuarioRepository` decide y
  aplica la regla ella misma; los demás repositorios son CRUD puro y esa clase de
  invariantes vive sólo en el service.
- [ ] **`validar_reloj` mezcla SQL crudo y regla de negocio directo en la fachada.**
  `src/application.rs:431`. Es la única consulta SQL de todo el codebase fuera de
  `src/database/*`.
- [ ] **`RegistroIngresoServiceError::RelojRetrocedido` se declara en el servicio pero
  nunca lo genera.** `src/services/error.rs:201`. La regla que lo produce vive entera en
  `AppCore::validar_reloj`, no en `RegistroIngresoService`.
- [ ] **Empresas no valida el formulario; Contratistas sí.**
  `src/tui/empresas/state.rs:283`. Dos pantallas CRUD equivalentes reparten la validación
  distinto: Contratistas valida cédula/nombre/PRAIND en el propio `state.rs`; Empresas
  envía el nombre tal cual, delegando todo al service.
- [ ] **`VERSION_REGLAS_ACCESO` vive lejos de las reglas que versiona.**
  `src/models/registro_ingreso.rs:6`. Declarada en una struct de persistencia, mientras que
  `DIAS_ADVERTENCIA_PRAIND` y el orden de las 5 reglas de acceso viven en
  `domain/acceso.rs`. Sin comentario ni test que fuerce subirla al cambiar la lógica real.

## Deuda técnica / duplicación

- [ ] **Bloque de spawn de hilo para hashear duplicado literalmente 3 veces.**
  `src/tui/app.rs:1185` (`iniciar_creacion_usuario`, `iniciar_cambio_password`,
  `iniciar_root_inicial`). Idéntico carácter por carácter salvo la variable `password`,
  sin una función auxiliar común.
- [ ] **`aplicar_retencion` aborta a mitad de camino y el fallo se pierde en ambos
  callers.** `src/database/backup.rs:382`. Usa `?` dentro de un loop de `remove_file`, así
  que el primer archivo bloqueado detiene todo sin borrar el resto; los dos únicos call
  sites (`application.rs`, `connection.rs`) descartan el `Result` con `let _ =`.
- [ ] **Limpieza de sentinelas de una restauración anterior descarta su error.**
  `src/database/backup.rs:223`. Si falla en silencio, el siguiente intento de restaurar
  falla más adelante con un error genérico sin pista de la causa real.
- [ ] **Limpieza de un `.partial` inválido descarta su propio error.**
  `src/database/backup.rs:179`. Contradice la garantía documentada de "nunca deja un
  `.partial` atrás".
- [ ] **Patrón `take().unwrap()` frágil repetido en los 3 flujos con hilo.**
  `src/tui/app.rs:1200` (y `:1248`, `:1288`). Chequean `Some` con un `let-else` y después
  vuelven a hacer `.take().unwrap()` en vez de un único `if let Some(...) =
  self.xxx.take()`. Sound hoy, pero un panic latente si una edición futura inserta algo
  entre esas dos líneas.
- [ ] **Migración 6 carga toda la tabla `registro_ingresos` en memoria.**
  `src/database/schema.rs:154`. Tabla de solo-inserción que crece indefinidamente; sienta
  un patrón riesgoso para una futura migración sobre una tabla ya más grande.
- [ ] **`UsuarioService::actualizar` es código muerto.** `src/services/usuario_service.rs:126`.
  Ningún camino de producción lo llama, sólo sus propios tests — segunda implementación
  paralela y ligeramente distinta de `actualizar_administracion`.
- [ ] **Chequeo de restricción UNIQUE duplicado igual en tres servicios.**
  `src/services/usuario_service.rs:278` (y `contratista_service.rs`, `empresa_service.rs`).
- [ ] **Mapeo `TipoIngreso` ↔ texto SQL duplicado ~6 veces en 4 archivos.**
  `src/database/repositories/contratista_repository.rs:33` (y `queries/contratistas.rs`,
  `registro_ingreso_repository.rs`, `queries/ingresos.rs`). Ningún match "texto→enum" es
  exhaustivo, así que agregar una variante nueva no obliga a actualizarlos todos.
- [ ] **Bloque de transacción `Immediate` + `validar_reloj` duplicado literalmente dos
  veces.** `src/application.rs:217` (`registrar_ingreso` y `registrar_salida`).
- [ ] **Verificación de login duplicada entre el servicio y la TUI.**
  `src/services/autenticacion_service.rs:56`. `autenticar()` sólo la usan los tests; el
  caller real (`app.rs`) copió a mano el mismo `match` para poder correrlo en un hilo.
- [ ] **`mover()` duplicada letra por letra entre dos pantallas.**
  `src/tui/nuevo_ingreso/state.rs:243` y `src/tui/salida_rapida/state.rs:139`. Idénticas
  salvo el nombre del vector.
- [ ] **`tipo_desde_texto` triplicado en 3 archivos.** `src/tui/activos/state.rs:94` (y
  `contratistas/state.rs`, `historial/filtros.rs`). El mismo intérprete `clave:valor` para
  tipo, copiado de forma independiente en cada pantalla.
- [ ] **`TipoIngreso` sin `as_str`/`FromStr`, duplicado hasta dentro de un mismo archivo.**
  `src/database/repositories/contratista_repository.rs:81`. El mapeo texto↔enum está
  duplicado dos veces en este único archivo (líneas 81-84 y 172-175), además de en los
  otros 3-4 archivos ya señalados arriba (`Mapeo TipoIngreso ↔ texto SQL`).
- [ ] **`SelectMenu`/`SelectMenuState` no tiene ningún consumidor real.**
  `src/tui/ui_kit/select_menu.rs`. 149 líneas con navegación y tests, sin ningún caso de
  uso en producción ni en `examples/`.
- [ ] **Bucle orquestador de términos `clave:valor` duplicado en 3 pantallas.**
  `src/tui/ui_kit/query_lang.rs` (falta el helper) — el bucle que separa términos libres de
  términos con clave se repite literal en `activos`, `contratistas` e `historial/filtros.rs`
  en vez de vivir una sola vez en `query_lang.rs`.

## Magia / números y comportamientos implícitos

- [ ] **`guardando` compartido como mutex implícito entre 2 estados pendientes.**
  `src/tui/app.rs:1190`. `creacion_usuario_pendiente` y `cambio_password_pendiente` son dos
  `Option` independientes en `App`; su exclusión mutua depende enteramente de un booleano
  en `UsuariosState` (otro archivo) que nada en `app.rs` valida.
- [ ] **Nombre `"Quintana"` hardcodeado como fallback sin sesión.**
  `src/tui/app.rs:1484`. Si se abre Nuevo Ingreso sin `self.sesion`, ese literal queda
  grabado como responsable del registro sin explicación de por qué ese nombre.
  Relacionado — orden de sondeo inconsistente: `root_inicial_pendiente` se revisa antes de
  leer teclas del usuario en el bucle principal; los otros 3 flujos con hilo se revisan
  después, en la sección de tick (`src/tui/app.rs:1062`), sin ningún comentario que
  explique la diferencia.
- [ ] **Fallo de invariante de migración se ve como error genérico de SQLite.**
  `src/database/schema.rs:113`. Si `version != SCHEMA_VERSION` al final, se devuelve
  `rusqlite::Error::InvalidQuery` — un error que en la práctica nunca viene de SQLite.
- [ ] **`AppCore::new`/`con_reloj` dejan `ruta_base_datos` vacía sin avisar en la firma
  pública.** `src/application.rs:81`. Construir un `AppCore` así y luego pedir un respaldo
  crea silenciosamente una carpeta `backups` relativa al directorio de trabajo del proceso.
- [ ] **Tope de 4 en `tipos_incluidos` trunca en silencio, repetido en 3 sitios.**
  `src/database/queries/ingresos.rs:160` (y `buscar_historial`, `ContratistasQuery::buscar`).
  No se deriva de `TipoIngreso::ALL.len()`.
- [ ] **Tope del selector de rol hardcodeado (`.min(2)`) en vez de `ROLES.len()`.**
  `src/tui/usuarios/state.rs:495`. Si se agrega un cuarto rol, queda inalcanzable desde el
  teclado sin ningún error de compilación.
- [ ] **Error de fecha corrupta se disfraza de error de SQLite.** `src/application.rs:450`.
  `validar_reloj` fabrica a mano un `FromSqlConversionFailure` para un error de parseo que
  no viene de SQLite — manda a depurar en la dirección equivocada.
- [ ] **Tope del desplegable "Tipo" hardcodeado en vez de `tipos().len()`.**
  `src/tui/contratistas/state.rs:486`. Literal `3` en vez de derivarse del array. Un quinto
  `TipoIngreso` quedaría inalcanzable con la flecha abajo.
- [ ] **`unwrap()` en la fecha por defecto de `FiltrosHistorial`.**
  `src/tui/historial/filtros.rs:27`. Seguro hoy, panic latente si cambia el día usado.
- [ ] **`StandardCommand::Activate` no tiene ningún consumidor real.**
  `src/tui/ui_kit/keyboard.rs:52`. Se genera para la tecla Espacio pero ningún `state.rs`
  hace `match` sobre ella — sólo aparece en el propio test del módulo.
- [ ] **Matriz `requiere_praind()`/`requiere_gafete()` sin explicar la regla de negocio.**
  `src/models/contratista.rs:18`. Combinación específica por `TipoIngreso` (InHouse
  requiere PRAIND pero nunca gafete, PorCorreo al revés) sin ningún comentario que la
  justifique — sólo se puede inferir leyendo el `match` línea por línea.
- [ ] **`fecha_hora_salida`/`usuario_salida_id` son 2 `Option` independientes.**
  `src/models/registro_ingreso.rs:65`. Deberían ser un único `Option<Salida{...}>` — el
  tipo permite hoy construir un registro "con salida sin usuario" o viceversa.

## Coherencia

- [ ] **El respaldo "obligatorio" pre-migración se salta si la ruta no tiene directorio
  padre.** `src/database/connection.rs:135`. Contradice el propio comentario de la
  función, que lo describe como bloqueante.
- [ ] **`DatabaseError` no expone la causa real vía `source()`.**
  `src/database/error.rs:34`. A diferencia de `SchemaError` (mismo módulo), que sí la
  reenvía — pierde el mensaje real de SQLite en cualquier consumidor que la use.
- [ ] **`StandardCommand::FocusNext`/`Primary` casi no se usan.**
  `src/tui/ui_kit/keyboard.rs`. Sólo `historial/state.rs` los usa de verdad; el resto de
  pantallas manejan Tab/BackTab/Enter/Espacio con matches propios en vez de pasar por
  `standard_command` — la abstracción compartida cubre menos de lo que su existencia
  sugiere.
- [ ] **`query_lang` es el único submódulo de `ui_kit` expuesto como `pub mod` completo.**
  `src/tui/ui_kit/mod.rs:9`. El resto son privados con una lista curada de `pub use` —
  rompe la convención del propio archivo sin razón aparente.

## Coherencia entre pantallas

- [ ] **La búsqueda se limpia al guardar un usuario pero no al activar/desactivarlo.**
  `src/tui/usuarios/state.rs:347` vs `:311`. Dos caminos de la misma pantalla que no
  coinciden en cómo tratan el filtro activo.
- [ ] **Password de ROOT inicial valida coincidencia antes que longitud; Usuarios lo hace al
  revés.** `src/tui/configuracion_inicial/state.rs:225` vs `usuarios/state.rs`. Misma regla,
  dos copias independientes que ya divergen en el orden de evaluación.
- [ ] **ROOT inicial es la única cuenta sin tope de longitud en cédula/nombre/password.**
  `src/tui/configuracion_inicial/state.rs:193`. El resto de formularios (Usuarios,
  Contratistas, Empresas) sí acotan esos mismos campos.
- [ ] **Doble `unwrap()` innecesario en `emitir_guardado` tras mover el formulario.**
  `src/tui/usuarios/state.rs:587`. Frágil ante un reordenamiento futuro del código; hoy no
  entra en pánico por pura casualidad de orden.
- [ ] **Mensaje de error persiste tras una búsqueda exitosa en Nuevo Ingreso.**
  `src/tui/nuevo_ingreso/state.rs:92`. `completar_busqueda` no limpia `self.error` en la
  rama `Ok`, a diferencia de la misma función en Salida Rápida que sí lo hace.
- [ ] **Cualquier tecla cierra la confirmación de Salida Rápida, no sólo Enter/Esc.**
  `src/tui/salida_rapida/state.rs:88`. `menu_principal` sí exige explícitamente Enter o Esc
  para resolver una confirmación pendiente.
- [ ] **Salida Rápida no soporta el toggle de ayuda (F1).**
  `src/tui/salida_rapida/state.rs:1`. Ni siquiera importa `StandardCommand`, a diferencia
  de Nuevo Ingreso y Menú Principal.
- [ ] **Campo de error de búsqueda distinto entre pantallas (`mensaje` vs `error_carga`).**
  `src/tui/activos/state.rs:264`. Activos/Historial usan `self.mensaje`;
  Contratistas/Empresas usan `self.error_carga` — mismo concepto, dos nombres.
- [ ] **Historial no avisa al no poder ocultar la última columna visible.**
  `src/tui/historial/state.rs:334`. Activos y Contratistas sí muestran un mensaje
  explicando la restricción; Historial simplemente no hace nada.
- [ ] **Política de "limpiar filtro tras guardar" distinta en cada pantalla.**
  `src/tui/contratistas/state.rs:601`. Contratistas limpia sólo al crear, Empresas sólo al
  crear, Activos nunca — mismo patrón de inconsistencia que el de Usuarios, reproducido de
  tres formas distintas más.
- [ ] **Empresas resetea la selección en cada tecla de búsqueda; las otras 3 no.**
  `src/tui/empresas/state.rs:238`. Mientras el debounce está pendiente, Empresas se ve
  vacía; Activos/Contratistas/Historial mantienen resaltada la fila anterior.
- [ ] **Patrón irrefutable asume una única variante de `TermValue`.**
  `src/tui/ui_kit/query_lang.rs:33`. `let TermValue::Simple(valor) = &term.value;` en 3
  funciones, apoyándose en que la crate externa `query-parser` (pineada a `0.2.0`) hoy sólo
  tiene esa variante, sin comentario que documente por qué es seguro sólo mientras se
  dependa de esa versión.
