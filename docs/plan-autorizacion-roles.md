# Plan: autorización por rol, invisibilidad de Root, contraseñas y auditoría de campos críticos

> **Cómo usar este documento:** es un prompt-plan autocontenido para otra sesión de IA que no
> tiene el contexto de la conversación donde se diseñó esto. Da la rama, el estado del repo, las
> decisiones ya tomadas (no discutirlas de nuevo) y el plan de fases con anclas exactas de código.
> Cualquier ambigüedad real que aparezca al implementar debe preguntarse al usuario, no adivinarse.

## Contexto de partida

- Repo `acceso_CLI`. Rama de trabajo: `auditoria-dominio-autorizacion` (ya existe con 7 commits
  encima de `main` que reparan 6 hallazgos de `docs/auditoria-dominio-2026-08-20.md`; si esa rama
  ya se mergeó a `main`, crear una rama nueva desde `main` para este plan).
- Leer primero `docs/auditoria-dominio-2026-08-20.md`, hallazgos **#1** ("Autorización protegida
  sólo por la presentación"), **#3** y **#6** (agregados con campos públicos) — ahí está el
  análisis original. Este plan implementa el #1 completo, más una función nueva (contraseña
  propia) y una auditoría acotada, decididas en conversación directa con el usuario. **#3 y #6
  quedan fuera de este plan a propósito** — ver "Qué no hacer" abajo.
- Rigor esperado, igual que el resto del repo: cada cambio lleva tests; antes de cada commit
  deben pasar en verde `cargo test`, `cargo clippy --lib --tests` (sin warnings) y
  `cargo fmt --all -- --check`. Commits chicos, uno por cambio lógico, mensaje en español que
  explique el *porqué*, no sólo el qué (mirar el `git log` de la rama `auditoria-dominio-autorizacion`
  para el tono/formato esperado).
- `RolUsuario` ya existe en `src/models/usuario.rs:12` con 3 variantes: `Root`, `Administrador`,
  `Operador`. No agregar ni renombrar variantes.

## Decisiones de diseño ya tomadas — implementar tal cual, no volver a discutirlas

1. **Jerarquía:** `Root` ⊇ `Administrador` ⊇ `Operador` (todo lo que puede hacer uno, lo puede
   hacer el de arriba), con dos excepciones exclusivas de Root: respaldos, y gestión de otras
   cuentas Root.
2. **Root es invisible para Administrador.** Un Administrador no puede ver, listar, editar,
   activar/desactivar ni resetear la contraseña de una cuenta Root — ni siquiera conociendo su
   ID directamente. Esto se implementa en dos capas (ver Fase 3): filtrado en la consulta (el
   dato no llega) + rechazo explícito en cada escritura (defensa en profundidad).
3. **Matriz de permisos:**

   | Operación | Operador | Administrador | Root |
   |---|:---:|:---:|:---:|
   | Crear/editar contratistas y empresas (datos básicos) | ✅ | ✅ | ✅ |
   | Registrar ingreso / salida | ✅ | ✅ | ✅ |
   | Activar/desactivar contratista o empresa | ❌ | ✅ | ✅ |
   | Gestionar usuarios (crear/editar/activar/desactivar), objetivo Operador o Administrador | ❌ | ✅ | ✅ |
   | Ver o gestionar una cuenta Root | ❌ | ❌ | ✅ (sólo él mismo / otro Root) |
   | Crear/gestionar respaldos | ❌ | ❌ | ✅ |
   | Ver el log de auditoría (Fase 6) | ❌ | ✅ | ✅ |

4. **Contraseña — dos flujos distintos, no uno con permisos distintos:**
   - **Self-service ("cambiar mi contraseña"):** cualquier rol, sólo sobre sí mismo, **pide la
     contraseña actual** antes de aceptar la nueva (evita que una sesión desatendida/robada la
     cambie sin que el dueño real se entere). Hoy esta función **no existe** — un Operador no
     tiene ningún camino para cambiar su propia contraseña porque la pantalla Usuarios le está
     oculta. Es una pantalla nueva, no un ajuste de permisos.
   - **Reset de administrador (el que ya existe, `cambiar_password_usuario` en
     `src/application.rs:391`):** Administrador/Root sobre otro usuario, **no pide la contraseña
     anterior** (es justo para cuando el dueño la perdió). Sujeto a la regla de invisibilidad de
     Root: un Administrador no puede resetear la de un Root.
   - Regla combinada:
     ```rust
     fn puede_cambiar_password(actor_id: i64, actor_rol: RolUsuario, objetivo_id: i64, objetivo_rol: RolUsuario) -> bool {
         actor_id == objetivo_id
             || (actor_rol.puede(Operacion::GestionarUsuarios) && puede_gestionar_usuario(actor_rol, objetivo_rol))
     }
     ```
5. **Auditoría acotada, no genérica.** Sólo se audita quién y cuándo cambia `tipo_ingreso` o
   `fecha_vencimiento_praind` de un contratista (vía `ContratistaService::actualizar`). No
   construir un sistema de auditoría genérico para "cualquier campo de cualquier entidad" — eso
   sería sobre-ingeniería para un problema que hoy tiene 2 campos concretos. Si en el futuro hace
   falta auditar más cosas, se agregan más casos al mismo patrón chico, no se generaliza ahora.
   Quién registró un ingreso/salida ya queda grabado desde antes (`usuario_ingreso_id`/
   `usuario_salida_id` + nombre snapshot) — esto es aparte, no se toca.

## Fases (orden obligatorio; cada fase termina con suite verde y, salvo que se diga lo contrario, su propio commit)

### Fase 1 — Núcleo de autorización (autocontenido, bajo riesgo, primero)

Crear el punto único de verdad de la política. Sugerido: `src/domain/autorizacion.rs` (nuevo
módulo, junto a `domain/acceso.rs` que ya tiene el precedente de reglas versionadas — no es
necesario versionarlo igual, es una nota de ubicación, no una obligación de copiar ese patrón).

- `enum Operacion` con al menos: `GestionarRespaldos`, `GestionarUsuarios`,
  `ActivarDesactivarContratista`, `ActivarDesactivarEmpresa`, `VerAuditoria`. (Crear/editar
  contratista-empresa y registrar ingreso/salida no necesitan entrada en el enum porque los 3
  roles pueden hacerlo siempre — no hay nada que decidir ahí.)
- `impl RolUsuario { pub fn puede(self, operacion: Operacion) -> bool }`.
- `pub fn puede_gestionar_usuario(actor: RolUsuario, objetivo: RolUsuario) -> bool` (la función
  de la sección de invisibilidad de Root arriba).
- `pub fn puede_cambiar_password(...)` (la de arriba).
- Tests unitarios que cubran la matriz completa como tabla de casos (todas las combinaciones
  rol×operación y rol×rol para las funciones de dos roles) — no tests sueltos ad hoc, una tabla
  para que quede evidente qué combinaciones se decidieron y cuáles no.

**Antes de escribir código de este módulo, decidir el nombre y la forma exactos leyendo cómo
están organizados `src/domain/acceso.rs` y `src/models/usuario.rs`, para que encaje con el
estilo ya establecido — no hay una única forma correcta, pero sí hay que ser consistente con lo
que ya existe.**

### Fase 2 — Enhebrar el actor por `AppCore` (la fase grande, mecánica)

1. Primero, ejecutar `grep -n "pub fn " src/application.rs` y clasificar cada método público en:
   mutante (necesita actor) vs. de sólo lectura (no lo necesita). Al momento de escribir este
   plan hay 18 métodos que matchean `crear|actualizar|activar|desactivar|registrar|cambiar` — la
   lista exacta puede haber cambiado, no asumir el número, recalcularlo.
2. Extender/reusar el patrón ya existente en `src/application.rs` para `registrar_ingreso`/
   `registrar_salida` (`en_transaccion_con_reloj_validado`, que ya verifica que el actor exista y
   esté activo — ver el commit `f307a07` en el `git log` de esta rama para el precedente exacto).
   Para el resto de métodos mutantes (usuarios, contratistas, empresas, respaldos) hace falta un
   mecanismo equivalente: reciben el actor (probablemente `&UsuarioSesion`, ya definida en
   `src/services/autenticacion_service.rs`), verifican permiso con `RolUsuario::puede(...)` de la
   Fase 1, y **sólo entonces** ejecutan la operación.
3. Decidir y documentar cómo se representa "sin actor" (flujos como la creación de ROOT inicial,
   que legítimamente no tienen sesión todavía) — no debe convertirse en un `Option<Actor>` que
   cada llamador tenga que manejar con pánico o `unwrap`; usar el mismo criterio que ya usa el
   código para "sin core" (`app.rs::abortar_configuracion_inicial_sin_core` como referencia de
   estilo, no para copiar literal).
4. Error nuevo (o extensión de los existentes por servicio — `UsuarioServiceError`,
   `ContratistaServiceError`, `EmpresaServiceError`, `RegistroIngresoServiceError` ya tienen cada
   uno su propio enum) para "operación no autorizada para este rol". Mantener el patrón existente
   de un enum de error por servicio en vez de crear un tipo de error transversal nuevo, salvo que
   se vea una razón concreta para lo contrario al implementar.
5. Actualizar **todos** los llamadores en `src/tui/app.rs` para pasar `self.sesion` (o el actor
   que corresponda) a cada llamada mutante de `AppCore`.
6. Actualizar **todos** los tests que llaman a `AppCore` directamente sin pasar por la TUI (hay
   varios archivos en `tests/` que hacen esto — `tests/app_core.rs`,
   `tests/contratista_service.rs`, `tests/usuario_service.rs`, etc. — correr
   `grep -rl "AppCore::new\|AppCore::abrir" tests/` antes de tocar nada para tener la lista real,
   no asumir cuáles son).

Esta fase es la de mayor superficie de cambio de todo el plan — tocar con cuidado, en commits
chicos por servicio (p. ej. un commit para usuarios, otro para contratistas, otro para empresas,
otro para respaldos) en vez de un solo commit gigante.

### Fase 3 — Invisibilidad de Root

- `FiltroUsuarios`/`buscar_usuarios` (`src/database/queries/usuarios.rs`): cuando el actor no es
  Root, excluir `rol = 'ROOT'` de la consulta SQL misma (el dato no debe llegar a memoria, no
  alcanza con no mostrarlo en la pantalla).
- Cada escritura sobre un usuario objetivo (`actualizar_usuario`, `activar_usuario`,
  `desactivar_usuario`, `cambiar_password_usuario`) rechaza explícitamente si el objetivo es Root
  y el actor no lo es — aunque la Fase 2 ya lo cubra vía `puede_gestionar_usuario`, este chequeo
  puntual necesita su propio test que pruebe "Administrador intenta tocar un ID de Root a mano"
  (no descubierto por la UI, sino pasado directo), para probar que no depende de que la pantalla
  lo esconda.

### Fase 4 — Self-service "cambiar mi contraseña"

- Nueva entrada de menú visible para los 3 roles (`OpcionMenu` en
  `src/tui/menu_principal/state.rs:78` tiene el patrón `visibles_para`/`visible_para` ya
  establecido).
- Pantalla nueva (state + render + tests), del tamaño de `src/tui/salida_rapida/` (chica,
  autocontenida) — no reusar la pantalla Usuarios existente, es un flujo distinto con reglas
  distintas (pide la contraseña actual).
- Servicio nuevo o método nuevo en `UsuarioService` que valida la contraseña actual (reusar
  `verificar_password`/`verificar_candidato` de `src/services/autenticacion_service.rs`, no
  reimplementar la comparación) antes de aceptar la nueva.
- Hilo de Argon2 aparte para no bloquear la TUI, mismo patrón que ya usan login/crear
  usuario/cambiar contraseña administrativa en `src/tui/app.rs` (buscar
  `generar_hash_en_hilo`/`HiloUsuarioPendiente` como referencia directa, es el mismo mecanismo).

### Fase 5 — Auditoría de campos críticos

- Migración nueva en `src/database/schema.rs` (sumar 1 a `SCHEMA_VERSION`, que a la fecha de este
  plan es 8 — verificar el valor real antes de escribir la migración, puede haber cambiado). Ver
  `MIGRACION_8` en el mismo archivo como plantilla exacta de estilo y de cómo se agrega una
  migración nueva sin tocar las anteriores.
- Tabla nueva, por ejemplo `auditoria_contratistas(id, fecha_hora, usuario_id, contratista_id,
  campo, valor_anterior, valor_nuevo)` — el nombre y la forma exacta quedan a criterio de quien
  implemente, siempre que capture como mínimo quién, cuándo, qué campo y los dos valores.
- `ContratistaService::actualizar` (`src/services/contratista_service.rs`) recibe el actor (ya
  disponible desde la Fase 2) y, si `tipo_ingreso` o `fecha_vencimiento_praind` cambiaron
  respecto al valor anterior, inserta una fila de auditoría **dentro de la misma transacción**
  que la actualización — no como un paso aparte que pueda quedar desincronizado.
- Tests que prueben: cambiar el tipo no deja rastro si no cambió de verdad (evitar ruido),
  cambiarlo sí lo deja con el actor correcto, y que un rollback de la transacción no deja fila de
  auditoría huérfana.

### Fase 6 — Pantalla de auditoría (sólo lectura, Administrador+)

- Pantalla nueva, gateada por `Operacion::VerAuditoria`, sin ninguna acción de escritura.
- Query de sólo lectura sobre la tabla nueva de la Fase 5, con paginación siguiendo el mismo
  criterio ya usado en Historial (`src/database/queries/ingresos.rs::buscar_historial` como
  referencia de estilo para total+página coherentes).

## Qué NO hacer (fuera de alcance de este plan a propósito)

- **No** implementar los hallazgos #3/#6 de `docs/auditoria-dominio-2026-08-20.md` (convertir
  `NuevoRegistroIngreso`/`Contratista` en agregados con constructores privados). Quedan
  documentados como pendientes para cuando se diseñe V3 (concurrencia multi-terminal) — hoy esa
  protección es cosmética porque la app es de instancia única y un solo hilo.
- **No** tocar `docs/auditoria-ui-tui-2026-08-20.md` ni sus hallazgos — es una auditoría aparte,
  de otra sesión, sobre consistencia visual, sin relación con este plan.
- **No** generalizar la auditoría de la Fase 5 a otros campos o entidades más allá de
  `tipo_ingreso`/`fecha_vencimiento_praind` de contratistas.
- **No** agregar ni renombrar variantes de `RolUsuario`.
- **No** usar `--no-verify` ni saltarse hooks para forzar un commit si algo falla — arreglar la
  causa real.

## Checklist de cierre

- [ ] `cargo test` completo en verde.
- [ ] `cargo clippy --lib --tests` sin warnings.
- [ ] `cargo fmt --all -- --check` limpio.
- [ ] `docs/auditoria-dominio-2026-08-20.md` actualizado: hallazgo #1 marcado `[x]` con el
      detalle de qué se hizo y en qué commits (mismo formato que los hallazgos #2/#4/#5/#7/#9/#11
      ya marcados en ese documento).
- [ ] Reporte final al usuario resumiendo qué se implementó, en qué commits, y cualquier decisión
      de diseño que haya hecho falta tomar sobre la marcha (nombres de tipos, forma exacta de la
      tabla de auditoría, etc.) que no estuviera ya fijada en este documento.
