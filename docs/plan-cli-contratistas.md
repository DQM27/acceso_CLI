# Plan: alta y edición de contratistas en la interfaz `--cli`

## Contexto y decisión (ya acordada con el usuario)

- La CLI (`src/cli/`) hoy cubre el ciclo ingreso/salida. Faltan altas/ediciones de contratistas, empresas y usuarios. **Se empieza por contratistas.**
- UX elegida por el usuario: **formulario en pantalla** dentro del área contextual, y **el mismo formulario precargado** sirve para editar (se llega con `/editar <nombre>` → lista → elegir).
- Todo se apoya en `AppCore` existente; la autorización real ya vive ahí (el rol se re-lee de SQLite en cada operación). No se toca la TUI clásica.

## API existente que se reutiliza (verificado)

- `AppCore::crear_contratista(&self, actor: &UsuarioSesion, datos: DatosContratista) -> Result<i64, ContratistaServiceError>` — `src/application/catalogos.rs:47`. Cualquier actor activo puede crear (Operador incluido).
- `AppCore::actualizar_contratista(&self, actor, id, datos: DatosActualizacionContratista) -> Result<(), _>` — `src/application/catalogos.rs:72`. Cambiar `cedula` exige `Operacion::EditarCedulaContratista`; cambiar `tiene_acceso` exige `ActivarDesactivarContratista` (Root/Admin). El resto lo edita cualquier actor activo. Escribe auditoría.
- `DatosContratista` y `DatosActualizacionContratista` tienen campos idénticos (`src/services/contratista_service.rs:38-56`):
  `cedula: String, nombre: String, empresa_id: i64, tipo_ingreso: TipoIngreso, fecha_vencimiento_praind: Option<NaiveDate>, es_personal_ruta: bool, tiene_acceso: bool`
- Validaciones del servicio (`construir_contratista`, contratista_service.rs:183): trim de cédula/nombre (vacío → `CedulaVacia`/`NombreVacio`), empresa debe existir, si `requiere_praind()` y no hay fecha → `PraindRequerido`, cédula duplicada → `CedulaDuplicada`. Los `Display` de los errores ya están en español, aptos para feedback.
- `ContratistaResumen` (`src/database/queries/contratistas.rs:23-36`) **tiene todos los campos del formulario** (id, empresa_id, cedula, nombre, empresa_nombre, tipo_ingreso, fecha_vencimiento_praind, es_personal_ruta, tiene_acceso): la edición se precarga desde la coincidencia elegida, **sin query extra**.
- `AppCore::listar_empresas()` → `Vec<Empresa>` (catalogos.rs:113); `Empresa { id, nombre, activo }` (`src/models/empresa.rs`).
- `RolUsuario::puede(Operacion)` — `src/domain/autorizacion.rs:14`. La TUI deshabilita la cédula si `!rol.puede(EditarCedulaContratista)` (`src/tui/contratistas/state.rs:424`); se replica ese criterio.
- Reglas de negocio (`src/models/contratista.rs`): `requiere_praind()` = ruta ∨ (Praind ∨ InHouse); `requiere_gafete()` = ¬ruta ∧ (Praind ∨ PorCorreo).
- Formato de fecha de la TUI clásica: `NaiveDate::parse_from_str(v, "%d/%m/%Y")` y auto-inserción de `/` tras 2 y 4 dígitos (`src/tui/contratistas/form.rs:45,98`). Se replica en el módulo nuevo (la TUI es privada/intocable).
- `texto::plegar_para_busqueda` para filtrar empresas sin tildes/mayúsculas.
- Defaults de alta de la TUI: tipo Praind, ruta false, acceso true.

## Diseño de interacción

**Comandos nuevos** (parser): `/nuevo` (alias `/n`) y `/editar <nombre|cédula>` (alias `/e`). `Comando::TODOS` pasa a 7: Ingreso, Salida, Activos, Nuevo, Editar, Ayuda, CerrarSesion.

**Flujo:**
1. `/nuevo` → contexto `NuevoContratista` (tarjeta "Enter abre el formulario · Esc cancela"). Si lleva argumentos → `MensajeError` ("el alta no toma argumentos"). Enter → se abre el formulario vacío.
2. `/editar <consulta>` → reutiliza `resolver_busqueda_contratistas` (lista `Coincidencias` con ↑↓). Enter → abre el formulario precargado desde el `ContratistaResumen`.
3. Mientras el formulario está abierto, el input deja de ser línea de comandos y pasa a ser **editor del campo activo** (la etiqueta del prompt cambia: `cédula › `, `nombre › `, …).

**Campos (orden):** Cédula → Nombre → Empresa → Tipo → Fecha PRAIND → Personal de ruta (Sí/No) → Acceso (Sí/No) → Confirmar.

**Teclas dentro del formulario:**
- `↑/↓`: mover campo activo (sincroniza el input con el valor del campo). Salta campos bloqueados por permiso.
- Texto (cédula, nombre, fecha): se teclea directo en el input. La fecha auto-inserta `/` (máx. 8 dígitos).
- Tipo / Ruta / Acceso: `Space` o `←/→` cicla el valor (Praind → InHouse → PorCorreo → Swat; Sí/No).
- Empresa: `Enter` entra al sub-modo selector: el área contextual lista empresas filtradas al teclear (`plegar_para_busqueda`, contiene), `↑↓` elige, `Enter` acepta y avanza, `Esc` vuelve sin cambiar. Se listan sólo activas; si la empresa actual (edición) está inactiva, se muestra igual.
- `Enter` en cualquier campo de texto avanza al siguiente. En `Confirmar`: valida; con errores se marcan `✗` por campo y no avanza; sin errores pasa a sub-fase **Resumen**.
- Resumen: tarjeta con todos los valores → `Enter` persiste → feedback `✓ Contratista registrado — Nombre` / `✓ Cambios guardados — Nombre`, se cierra el formulario y vuelve el contexto normal. Error del servicio → feedback `✗` con su `Display` y se regresa a editar.
- `Esc`: en selector de empresa o resumen → vuelve a editar; editando → cierra el formulario (sin confirmación; reabrir es barato). `Ctrl+C` sigue saliendo de la app.

**Permisos en la UI** (con el `rol` de `Fase::Operando`; AppCore re-verifica de todos modos):
- Editar: cédula habilitada sólo si `rol.puede(EditarCedulaContratista)`; si no, se muestra apagada y se salta en la navegación (igual que la TUI).
- Campo Acceso habilitado sólo si `rol.puede(ActivarDesactivarContratista)`; si no, apagado y se conserva el valor original.
- En alta, la cédula siempre es editable.

## Cambios por archivo

1. **`docs/plan-cli-contratistas.md`** (PRIMER PASO, lo pidió el usuario): copiar este plan tal cual para poder retomar si la app se cierra.

2. **`src/cli/parser.rs`**: variantes `Nuevo` y `Editar` en `Comando` (+ `TODOS` a 7, `nombre()`, `desde_texto()` con alias `n`/`e`). Tests: `/nuevo`, `/editar car`, `/n`, `/e`, largo de `TODOS`.

3. **`src/cli/formulario.rs` (NUEVO, lógica pura sin terminal ni AppCore):**
   - `enum ModoFormulario { Nuevo, Editar { id: i64 } }`
   - `enum Campo { Cedula, Nombre, Empresa, Tipo, FechaPraind, Ruta, Acceso, Confirmar }`
   - `enum Subfase { Editando, EligiendoEmpresa { seleccion: usize }, Resumen }`
   - `struct FormularioContratista { modo, campo, subfase, cedula: String, nombre: String, empresa: Option<(i64, String)>, tipo: TipoIngreso, fecha_praind: String, es_personal_ruta: bool, tiene_acceso: bool, empresas: Vec<Empresa>, cedula_editable: bool, acceso_editable: bool, errores: Vec<(Campo, String)> }`
   - Métodos: `nuevo(empresas, acceso_editable)`, `editar(&ContratistaResumen, empresas, cedula_editable, acceso_editable)`, `campos_navegables()` (omite bloqueados), `siguiente()/anterior()`, `alternar(delta)`, `agregar_fecha(digito)` / `borrar_fecha()` (auto `/`), `empresas_filtradas(consulta)`, `validar() -> Result<DatosContratista, Vec<(Campo, String)>>` (cédula/nombre no vacíos tras trim, empresa presente, fecha `%d/%m/%Y` obligatoria sólo si `requiere_praind()`), `datos_actualizacion()` (misma forma, otro struct).
   - Tests unitarios aquí mismo: navegación salta bloqueados, fecha auto-`/`, validación (vacíos, fecha inválida/requerida/no requerida, sin empresa), ciclo de tipo, precarga en edición, cédula bloqueada no modificable.

4. **`src/cli/estado.rs`:** variante `ContextState::NuevoContratista`; campo `AppState.formulario: Option<FormularioContratista>`.

5. **`src/cli/resolver.rs`:** `Comando::Nuevo` → `NuevoContratista` o `MensajeError` si trae consulta; `Comando::Editar` → `resolver_busqueda_contratistas`. `calcular_sugerencias`: pistas para `/nuevo` y `/editar`.

6. **`src/cli/render.rs`:**
   - `descripcion_comando`: Nuevo → "dar de alta un contratista", Editar → "editar un contratista — /editar <nombre>".
   - `lineas_formulario(&FormularioContratista, ancho)`: título "NUEVO CONTRATISTA" / "EDITAR CONTRATISTA — <nombre>", una línea por campo (`▸` en el activo, bloqueados apagados con "(sin permiso)", `✗` + mensaje junto a campos con error, bools como Sí/No, tipo con su etiqueta). En `EligiendoEmpresa`: lista filtrada (máx. 7) bajo los campos con selección resaltada. En `Resumen`: tarjeta "REVISAR Y CONFIRMAR".
   - `lineas_contexto`: nueva variante `NuevoContratista` (tarjeta de entrada).
   - `lineas_ayuda`: añadir `/nuevo` y `/editar <nombre>`; actualizar línea de alias (`/i /s /a /n /e /cs`).
   - Prompt: etiqueta según campo activo cuando hay formulario; línea de pistas por sub-fase ("↑↓ campo · Enter siguiente · Esc cancelar", "escriba para filtrar · ↑↓ elegir · Enter aceptar", etc.).

7. **`src/cli/mod.rs`:**
   - `recomputar`: no-op mientras `app.formulario.is_some()`.
   - `manejar_operando`: si hay formulario, delegar en `manejar_formulario` (nuevo) antes que el manejo normal.
   - `manejar_formulario(core, app, key)`: rutas de teclas descritas arriba, sincronizando input ↔ campo activo; la persistencia (Enter en Resumen) llama `core.crear_contratista`/`core.actualizar_contratista` con la `sesion` de `Fase::Operando`, feedback según resultado, y al cerrar formulario: `input.reset()` + `recomputar`.
   - `confirmar`: `ContextState::NuevoContratista` → abrir formulario vacío (cargando `core.listar_empresas()`); rama `Coincidencias` → si el comando es `Editar`, abrir formulario precargado en vez de ficha.
   - Reexportar lo necesario desde `mod.rs` (`mod formulario;`).

8. **Verificación:** `cargo check`, `cargo clippy` (sin warnings nuevos), `cargo test` completo. Si hay snapshots de la CLI que cambien (ayuda), actualizarlos.

## Notas

- No se añaden comandos de activar/desactivar ni borrado: el acceso se controla con el campo Acceso del formulario de edición (igual que la TUI, que no tiene "desactivar contratista" aparte).
- Empresas y usuarios quedan para fases siguientes sobre este mismo patrón (el doc en `docs/` debe mencionarlo como pendiente, quizá también en `docs/pendientes.md` si aplica).
- Restricción vigente del proyecto: no tocar la TUI clásica (`src/tui/`); todo vive en `src/cli/`.
- Nunca usar la palabra prohibida por el usuario (ver AGENTS.md): "puesto de control" / "portería".
