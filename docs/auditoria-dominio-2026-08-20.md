# Auditoría de arquitectura y dominio (2026-08-20)

Auditoría en modo solo lectura, realizada en paralelo sobre dominio/modelos,
aplicación/servicios e infraestructura/TUI/pruebas. El objetivo principal fue comprobar
si las decisiones e invariantes del negocio están protegidas por el dominio y no sólo por
la interfaz o por convenciones de uso.

## Resumen ejecutivo

La aplicación tiene una base técnica sólida: las reglas principales de acceso cuentan
con buena cobertura, las escrituras críticas usan transacciones `IMMEDIATE`, existen
restricciones para ingresos y gafetes activos, y las pruebas de migración, concurrencia,
respaldos e integridad pasan.

El riesgo principal es arquitectónico y de autorización: `AppCore` y varios servicios
permiten operaciones sensibles sin recibir un actor autenticado ni comprobar su rol y
estado. La TUI oculta opciones, pero no constituye una frontera de seguridad. Además,
algunas entidades permiten construir estados que deberían ser imposibles, por lo que el
dominio todavía no es la única autoridad de las decisiones.

**Estado (2026-08-20): 11 hallazgos, verificados uno por uno leyendo el código real antes
de reparar (no se tomó ninguno por alucinado — ver detalle en cada uno). 6 reparados (#2,
#4, #5, #7, #9, #11), 1 revisado y descartado con evidencia (#8), 1 no se toca por
contradecir una decisión explícita ya tomada en una sesión anterior (#10 — ver
`plan-saneamiento.md`/`hallazgos-auditoria.md`). Pendientes #1, #3 y #6 — los tres son
refactors arquitectónicos de alcance amplio (contexto de actor en toda `AppCore`,
agregados de dominio con constructores privados) que ameritan una decisión de diseño
explícita antes de tocarlos, no sólo un fix puntual.**

## Hallazgos

### 1. [ ] Alta — Autorización protegida sólo por la presentación

**Pendiente — requiere decisión de diseño, no es un fix puntual (2026-08-20).** El
hallazgo es real: verificado que ninguna mutación de `AppCore` recibe ni comprueba un
actor. Pero cerrarlo bien (política explícita por caso de uso, contexto de sesión
enhebrado por toda la fachada) toca decenas de métodos de `AppCore` y todos sus
llamadores en `app.rs` — un alcance muy distinto al resto de hallazgos de esta lista, que
son fixes locales y contenidos. Además, hoy `AppCore` sólo lo llama la TUI (que ya exige
login y ya oculta opciones por rol) — no es una ruta explotable hoy, es una fragilidad
arquitectónica si algún día se agrega otro consumidor de `AppCore`. Vale la pena
diseñarlo con el usuario antes de tocarlo, no adivinar el alcance solo.

`AppCore` expone mutaciones de contratistas, empresas, usuarios, roles, contraseñas,
ingresos, salidas y respaldos sin recibir un actor autenticado ni aplicar una política de
permisos. La política visible actualmente consiste principalmente en ocultar opciones de
la TUI.

Referencias:

- `src/application.rs:245-263`
- `src/application.rs:281-315`
- `src/application.rs:340-406`
- `src/tui/menu_principal/state.rs:69-82`
- `src/tui/app.rs:1769-1834`

Impacto: un adaptador futuro, una ruta interna o una sesión degradada podría ejecutar
operaciones administrativas. También es posible atribuir movimientos a un ID de usuario
distinto del actor real.

Acción propuesta:

- Introducir un `ActorAutorizado` o contexto de sesión en los comandos de aplicación.
- Definir una política explícita por caso de uso y denegar por defecto.
- Comprobar dentro de la transacción que el actor existe, continúa activo y posee el rol
  necesario.
- Para movimientos auditados, asegurar que el autor persistido sea el actor autorizado.

Criterios de cierre:

- Un operador no puede ejecutar casos de uso administrativos aunque invoque directamente
  `AppCore`.
- Un usuario inactivo no puede crear entradas ni salidas.
- Los tests cubren actor ajeno, operador, usuario inactivo y revocación concurrente.

### 2. [x] Alta — Una sesión revocada puede continuar registrando movimientos

**Reparado (2026-08-20).** `AppCore::en_transaccion_con_reloj_validado` (compartida por
`registrar_ingreso`/`registrar_salida`) ahora verifica que `usuario_id` sea un operador
activo dentro de la misma transacción `Immediate` que el reloj — una desactivación
concurrente no puede colarse entre el chequeo y la escritura. Nueva
`RegistroIngresoServiceError::OperadorNoAutorizado`. 4 tests de integración en
`tests/operador_activo.rs`.

Entrada y salida reciben un `usuario_id` crudo. La persistencia exige que el usuario
exista mediante una FK, pero no que esté activo. La TUI tampoco fuerza siempre el cierre
de sesión cuando una cuenta es desactivada o degradada.

Referencias:

- `src/application.rs:245-260`
- `src/application.rs:281-290`
- `src/services/registro_ingreso_service.rs:203`
- `src/database/repositories/registro_ingreso_repository.rs:175-185`
- `src/database/repositories/registro_ingreso_repository.rs:319-327`
- `src/database/schema.rs:497-500`
- `src/tui/app.rs:1906-1917`

Acción propuesta: resolver este punto junto con el contexto de actor del hallazgo 1,
revalidándolo dentro de la misma transacción del movimiento. Como defensa adicional,
puede evaluarse un trigger que rechace autores inactivos.

### 3. [ ] Alta — El agregado de ingreso puede persistirse en estados incoherentes

**Pendiente — requiere decisión de diseño (2026-08-20).** Confirmado: `crear_registro`
en `tests/registro_ingreso_repository.rs` arma un `NuevoRegistroIngreso` a mano, con
campos que no tienen por qué corresponder entre sí — nada al nivel de tipos lo impide.
Pero en el camino real de producción, el único lugar que construye este struct es
`RegistroIngresoService::registrar_entrada`, derivándolo siempre de un `Contratista`
recién leído — la inconsistencia sólo es alcanzable si algo más, en el futuro, construye
el struct directamente sin pasar por el servicio. Convertirlo en un agregado con
constructor privado (la corrección de fondo) es un rediseño real del modelo, no un
parche — mejor decidirlo junto con el hallazgo 1, ya que ambos tocan la misma capa de
"qué puede construir esto directamente".

`NuevoRegistroIngreso` y `DatosHistoricosEntrada` tienen campos públicos. El repositorio
acepta el snapshot proporcionado por el llamador, mientras que SQLite sólo protege IDs
con claves foráneas independientes. Es posible asociar un contratista con una empresa
existente que no le corresponde, falsear el snapshot o persistir combinaciones que el
servicio normalmente rechazaría.

Referencias:

- `src/models/registro_ingreso.rs:29-50`
- `src/database/repositories/registro_ingreso_repository.rs:168-202`
- `src/database/schema.rs:460-500`
- `tests/registro_ingreso_repository.rs:97-117`

Acción propuesta:

- Construir el ingreso exclusivamente mediante un agregado o fábrica validada.
- Hacer privados sus campos y expresar las transiciones `registrar_entrada` y `cerrar`.
- Derivar el snapshot en el `INSERT` mediante un `SELECT` unido entre contratista y
  empresa, o reforzar la relación mediante trigger/FK compuesta.
- Probar empresa ajena, snapshot manipulado y PRAIND sin gafete.

### 4. [x] Alta — Una actualización administrativa puede restaurar un hash anterior

**Reparado (2026-08-20).** `persistir_usuario` ya no incluye `password_hash` en su
`UPDATE` — ni `actualizar_protegiendo_ultimo_root` ni `establecer_activo` (sus dos únicos
llamadores) deben tocar esa columna; el único camino que la escribe sigue siendo
`actualizar_password`. Esto hace la condición de carrera imposible en vez de improbable.
Test de regresión en `tests/usuario_service.rs` que reproduce el camino exacto.


`actualizar_administracion` lee el usuario completo y conserva su `password_hash`. El
repositorio abre después una transacción y relee el estado actual, pero puede terminar
persistiendo el objeto anterior completo. Si otra instancia cambia la contraseña entre
ambas lecturas, una edición de nombre o rol puede restaurar el hash viejo.

Referencias:

- `src/services/usuario_service.rs:139-148`
- `src/database/repositories/usuario_repository.rs:119-135`
- `src/database/repositories/usuario_repository.rs:238-245`

Acción propuesta: usar un `UPDATE` parcial que no incluya `password_hash`, reconstruir el
nuevo estado desde la fila leída dentro de la transacción o incorporar versionado
optimista. Añadir una prueba de concurrencia entre cambio de contraseña y edición
administrativa.

### 5. [x] Media-alta — El login diferido puede aceptar credenciales revocadas

**Reparado (2026-08-20).** `recibir_autenticacion_si_lista` vuelve a resolver el
candidato contra SQLite (rápido, sin Argon2) antes de aceptar la sesión, descartando el
snapshot que viajó por el canal — `buscar_candidato` ya rechaza cuentas inactivas, así
que la revalidación reutiliza esa regla en vez de duplicarla. Test de regresión que
desactiva la cuenta justo después de arrancar el hilo de Argon2, antes de cualquier
sondeo del canal (la carrera queda determinista, no depende de cuánto tarde Argon2).


La aplicación obtiene usuario y hash, ejecuta Argon2 fuera del hilo principal y acepta el
snapshot sin consultar nuevamente SQLite. Durante ese intervalo la cuenta puede ser
desactivada, degradada o cambiar de contraseña.

Referencias:

- `src/services/autenticacion_service.rs:62-83`
- `src/services/autenticacion_service.rs:91-99`
- `src/tui/app.rs:1185-1190`

Acción propuesta: tras verificar Argon2, finalizar la autenticación contra la base,
comprobando ID, estado, rol y una versión o el hash vigente. Añadir pruebas de invalidación
concurrente.

### 6. [ ] Media — El modelo de contratista permite estados imposibles

**Pendiente — requiere decisión de diseño (2026-08-20).** Confirmado: `Contratista` tiene
todos los campos públicos, `empresa_activa` incluido, construible libremente en cualquier
combinación (se ve en cómo lo arman los tests, campo por campo). En el camino real,
`empresa_activa` siempre llega ya resuelto por `JOIN` desde el repositorio — no hay hoy
un lugar de producción que lo setee mal. El fix de fondo (mover el modelo al dominio,
campos privados, constructores `crear`/`actualizar`/`evaluar_acceso`) es el mismo tipo de
rediseño que el hallazgo 3, sobre el modelo hermano (`Contratista` vs
`NuevoRegistroIngreso`) — mejor abordarlos juntos, con el usuario, que por separado.

`Contratista` expone todos sus campos, incluido `empresa_activa`, aunque éste es un dato
derivado. `verificar_acceso` confía en ese booleano y las invariantes de PRAIND se validan
manualmente en un servicio concreto.

Referencias:

- `src/models/contratista.rs:5-19`
- `src/domain/acceso.rs:18-48`
- `src/services/contratista_service.rs:130-180`

Acción propuesta: mover el modelo al dominio, hacer privados sus campos e introducir
constructores y operaciones como `Contratista::crear`, `actualizar` y `evaluar_acceso`.
Los servicios deben orquestar repositorios y transacciones, no recordar cada invariante.

### 7. [x] Media — El historial no conserva todas las entradas de la decisión

**Reparado (2026-08-20).** Migración 8: `registro_ingresos` gana
`empresa_activa_snapshot` (`DEFAULT 1`, correcto para toda fila existente — ver el
commit para el razonamiento completo, no sólo "la mejor reconstrucción posible").
`DatosHistoricosEntrada`/`MovimientoIngresoResumen` exponen el campo; el servicio lo
puebla desde `contratista.empresa_activa` (ya resuelto por el repositorio vía `JOIN`).
Test que registra una entrada con la empresa activa, la desactiva después, y confirma
que el historial reconstruye el estado que tenía al momento del ingreso.

Desde la versión 2 de las reglas, `empresa_activa` influye en el resultado, pero la
fotografía histórica no almacena ese dato. Por ello no puede reconstruirse íntegramente
por qué se tomó una decisión pasada.

Referencias:

- `src/models/registro_ingreso.rs:29-38`
- `src/database/repositories/registro_ingreso_repository.rs:168-182`

Acción propuesta: persistir todas las entradas utilizadas por la política versionada,
incluido el estado de la empresa. Añadir una prueba que reconstruya una decisión sólo a
partir del snapshot histórico.

### 8. [x] Media-baja — La preparación del ingreso puede mezclar snapshots

**Revisado, no se cambia (2026-08-20).** La carrera descrita no es alcanzable con la
arquitectura real de esta app: `InstanciaGuard` garantiza una sola instancia por base de
datos, y el bucle de la TUI es síncrono de un solo hilo — entre las dos líneas de Rust
que leen `contratista` y `empresa` en `preparar_ingreso` no puede ejecutarse ningún otro
código que escriba en SQLite, porque no hay otro proceso ni otro hilo que compita por la
conexión en ese instante. Es exactamente el mismo razonamiento que ya está documentado
para un caso análogo en `docs/plan-saneamiento.md` ("Ediciones concurrentes de
contratistas... Con la instancia única, una sola vista activa y el bucle síncrono
actual, este escenario no puede producirse mediante la aplicación"). Agregar una
transacción de lectura defendería contra un escenario que no puede darse hoy — se deja
documentado para reconsiderar si la app pasara a varias terminales o conexiones
escritoras concurrentes, como ya prevé esa misma sección.

`preparar_ingreso` consulta contratista y empresa por separado, sin una lectura
transaccional consistente. Un cambio concurrente puede hacer que la decisión preliminar
y el nombre mostrado procedan de estados diferentes. El registro definitivo sí revalida
atómicamente, por lo que el problema afecta principalmente a la información mostrada al
operador.

Referencias:

- `src/services/registro_ingreso_service.rs:168-190`

Acción propuesta: usar una proyección única o una transacción de lectura para la vista
previa, conservando el registro definitivo como autoridad final.

### 9. [x] Baja — PRAIND ausente y vencido son indistinguibles

**Reparado (2026-08-20).** Nueva `MotivoDenegacion::PraindNoRegistrado` para "nunca se
cargó la fecha"; `PraindVencido` queda sólo para una fecha existente fuera de vigencia.
El match exhaustivo de `nuevo_ingreso/state.rs` (a propósito, sin `_=>`) obligó al
compilador a pedir un mensaje para la variante nueva.

Ambas situaciones producen `PraindVencido`. Esto pierde precisión en mensajes,
estadísticas y auditoría.

Referencias:

- `src/domain/acceso.rs:39-48`
- `src/domain/resultado_acceso.rs:2-6`

Acción propuesta: añadir un resultado como `PraindNoRegistrado` y mantener
`PraindVencido` únicamente para una fecha existente fuera de vigencia.

### 10. [ ] Baja — Los fallos del respaldo automático son silenciosos

**No se toca — contradice una decisión explícita ya tomada (2026-08-20).** Es correcto
sobre el código (`respaldo_automatico_diario_si_hace_falta` traga todos sus errores), pero
esto ya se decidió a propósito en una sesión anterior: `docs/hallazgos-auditoria.md`
("por diseño, ya decidido explícitamente en la Fase 4 — 'ignorar en silencio si falla, no
es obligatorio'... no tratar como bug salvo que se quiera reconsiderar esa decisión") y
`docs/plan-saneamiento.md` ("Errores observables — Descartado, el usuario determinó que
este nivel de auditoría no aplica para este tipo de app"). La auditoría que generó este
hallazgo no tenía ese contexto de sesiones previas. Se deja pendiente para que el usuario
decida si quiere reabrir esa decisión — no es una omisión mía, es deliberado.

Los errores al listar respaldos, crear el automático o aplicar la retención se descartan.
La aplicación puede aparentar estar protegida sin haber creado el respaldo diario.

Referencia:

- `src/application.rs:414-430`

Acción propuesta: conservar el comportamiento no bloqueante, pero registrar el último
intento fallido y mostrar una alerta operativa en la TUI.

### 11. [x] Baja — El repositorio no cumple `cargo fmt --check`

**Reparado (2026-08-20).** `cargo fmt --all` aplicado en un commit aislado, sin mezclar
con cambios funcionales — `cargo fmt --all -- --check` ya pasa limpio.

`cargo fmt --all -- --check` detecta diferencias de formato en numerosos archivos. No es
un defecto funcional, pero dificulta revisiones y puede hacer fallar un pipeline que exija
formato estándar.

Acción propuesta: ejecutar `cargo fmt --all` en un cambio aislado para no mezclar ruido de
formato con las correcciones funcionales.

## Fortalezas verificadas

- La decisión y escritura de entrada/salida se ejecutan bajo transacciones `IMMEDIATE`.
- Los índices únicos protegen contratistas y gafetes activos frente a concurrencia.
- El registro definitivo revalida revocaciones antes de insertar.
- La protección del último usuario ROOT activo es transaccional.
- Las reglas principales tienen buena cobertura, incluidos límites de fecha, tipo de
  ingreso, personal de ruta y empresa inactiva.
- Hay cobertura sólida de migraciones, claves foráneas, respaldos, FTS, atomicidad e
  inmutabilidad histórica.
- `cargo test --all-features` pasó completamente, incluidas las suites de integración.
- `cargo clippy --all-targets --all-features -- -D warnings` pasó sin advertencias.

## Orden sugerido de implementación

1. Crear el contexto de actor y centralizar autorización/revocación.
2. Corregir la actualización concurrente de usuario y cerrar correctamente el login
   diferido.
3. Convertir ingreso y contratista en agregados de dominio con construcción controlada.
4. Reforzar la integridad defensiva de los movimientos en SQLite.
5. Completar el snapshot histórico y precisar los motivos de denegación.
6. Hacer observable el fallo de respaldos automáticos.
7. Aplicar formato en un cambio independiente.

## Pruebas nuevas recomendadas

- Operador intentando cada comando administrativo directamente contra `AppCore`.
- Usuario desactivado intentando registrar entrada y salida.
- Revocación o degradación durante una sesión ya iniciada.
- Desactivación, cambio de rol o contraseña mientras Argon2 está verificando.
- Cambio concurrente de contraseña y edición administrativa del mismo usuario.
- Empresa válida pero ajena al contratista en un nuevo ingreso.
- Snapshot manipulado y PRAIND sin gafete mediante la API del repositorio.
- Reconstrucción completa de una decisión histórica.
- Fallos de disco/permisos durante el respaldo automático.

