# Catálogo de gafetes y deuda por pérdida (implementado — ver `docs/pendientes.md`)

> Diseñado y aprobado con el usuario el 2026-08-22. Implementado el 2026-08-30
> siguiendo el orden de la sección 11, con tres adaptaciones respecto a lo que
> dice este documento (nada cambia en el diseño, sólo en dónde encaja):
> 1. `MIGRACION_13` ya la había usado la generalización de auditoría
>    (`auditoria_cambios`) — quedó en `MIGRACION_14`, `SCHEMA_VERSION` 13→14.
> 2. Los mensajes de error (sección 2) ya no viven en
>    `src/tui/app/error_messages.rs` (no existe más) — están en
>    `src/mensajes.rs`, compartido por TUI y GUI.
> 3. La GUI (`desktop/`, inexistente cuando se aprobó este plan) también
>    recibió pantalla propia (`desktop/src/pantallas/Gafetes.tsx` +
>    `GestionGafeteModal.tsx` + `FormularioGafete.tsx`), decisión explícita
>    del usuario al retomarlo — no sólo la validación heredada del núcleo.
> El detalle de qué se hizo y el estado de cada paso vive en
> `docs/pendientes.md`, sección "Catálogo de gafetes". Este documento queda
> como referencia de diseño, no como rastreador de progreso.

## Contexto

Hoy `gafete_numero` en `registro_ingresos` es un `INTEGER` libre: cualquier número sirve, sin relación con los gafetes físicos reales (finitos, ej. 01 al 25). Sólo hay una garantía de BD: un número no puede estar activo (sin salida) dos veces a la vez (`idx_registro_ingresos_gafete_activo`). Esto es funcional pero "fantasma" — un typo entra igual, y no hay forma de sacar de circulación un gafete perdido ni de saber quién lo debe.

Se agrega un catálogo real de gafetes con tres estados (disponible/perdido/de_baja), validación al registrar un ingreso, un aviso no bloqueante si el contratista tiene una deuda pendiente, y una pantalla nueva de gestión. Decisiones ya tomadas con el usuario (no reabrir):
- El campo de gafete en Nuevo Ingreso sigue siendo texto libre, sin desplegable ni buscador — prioridad es velocidad de tecleo. Sólo se agrega validación con mensaje de error claro.
- Deuda = sólo estado + contratista deudor, sin monto ni nota.
- El aviso de deuda al preparar un ingreso es informativo, **no bloquea**.
- Cualquier operador con sesión gestiona el catálogo completo (alta/baja/perdido/resolver) — sin restricción de rol, a diferencia de Empresas.
- El "reporte" de deudas es sólo la lista filtrable en pantalla (`estado:perdido`, mismo patrón `clave:valor` que Contratistas/Historial) — no se exporta a Excel.
- Alta inicial por rango (desde-hasta), además de individual, para cargar 01-25 de una vez.

## 1. Schema — `MIGRACION_13`

`src/database/schema.rs`: `SCHEMA_VERSION` 12→13, agregar bloque `if version == 12 { aplicar_migracion(&transaction, MIGRACION_13, 13)?; version = 13; }` tras el bloque de `MIGRACION_12` (línea ~122).

```sql
CREATE TABLE gafetes (
    id INTEGER PRIMARY KEY,
    numero INTEGER NOT NULL UNIQUE,
    estado TEXT NOT NULL CHECK (estado IN ('DISPONIBLE', 'PERDIDO', 'DE_BAJA')),
    contratista_deudor_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    CHECK (
        (estado = 'PERDIDO' AND contratista_deudor_id IS NOT NULL)
        OR (estado <> 'PERDIDO' AND contratista_deudor_id IS NULL)
    )
);
CREATE INDEX idx_gafetes_estado ON gafetes(estado);
CREATE INDEX idx_gafetes_contratista_deudor
ON gafetes(contratista_deudor_id) WHERE contratista_deudor_id IS NOT NULL;

CREATE TABLE gafetes_incidentes (
    id INTEGER PRIMARY KEY,
    gafete_id INTEGER NOT NULL REFERENCES gafetes(id) ON DELETE RESTRICT,
    tipo TEXT NOT NULL CHECK (tipo IN ('PERDIDO', 'RESUELTO')),
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    motivo_resolucion TEXT CHECK (
        motivo_resolucion IS NULL OR motivo_resolucion IN ('PAGADO', 'APARECIDO')
    ),
    CHECK (
        (tipo = 'PERDIDO' AND contratista_id IS NOT NULL AND motivo_resolucion IS NULL)
        OR (tipo = 'RESUELTO' AND contratista_id IS NULL AND motivo_resolucion IS NOT NULL)
    )
);
CREATE INDEX idx_gafetes_incidentes_gafete ON gafetes_incidentes(gafete_id, id DESC);
CREATE INDEX idx_gafetes_incidentes_fecha ON gafetes_incidentes(fecha_hora DESC, id DESC);
```

`gafetes` guarda sólo el estado vigente; `gafetes_incidentes` es el historial append-only (mismo patrón que `auditoria_contratistas`), necesario para "quién debe qué" y para no perder el rastro de cuándo se marcó/resolvió. `registro_ingresos.gafete_numero` **no** se vuelve FK a `gafetes.numero` — hay filas históricas con números que el catálogo (creado vacío) no tiene por qué conocer.

## 2. Modelo y errores

- `src/models/gafete.rs` (nuevo): `EstadoGafete{Disponible,Perdido,DeBaja}` (+ `as_str_sql`/`from_str_sql`), `Gafete{id,numero,estado,contratista_deudor_id}`, `MotivoResolucionGafete{Pagado,Aparecido}`. Registrar en `src/models/mod.rs`.
- `src/services/error.rs`: dos variantes nuevas en `RegistroIngresoServiceError` (junto a `GafeteOcupado`): `GafeteNoRegistrado`, `GafeteNoDisponible(EstadoGafete)`. Nuevo enum `GafeteServiceError` (NumeroInvalido, NumeroDuplicado, GafeteNoEncontrado, RangoInvalido, ContratistaDeudorRequerido, ContratistaNoEncontrado, EstadoInvalido, OperacionNoAutorizada, `Database(#[from] DatabaseError)`).
- `src/tui/app/error_messages.rs`: mapear las 2 variantes nuevas en `mensaje_ingreso` ("El número de gafete no existe en el catálogo" / "El gafete está marcado como perdido" / "El gafete está dado de baja"). Nueva función `mensaje_gafete(error: GafeteServiceError) -> String`, mismo patrón que `mensaje_empresa`.

## 3. Validación en `registrar_entrada`

`src/services/registro_ingreso_service.rs`: `RegistroIngresoService` gana tercer genérico `G: GafeteRepository + ?Sized` (campo `gafetes: &'a G`, constructor `new(contratistas, registros, gafetes)`).

Dentro de `registrar_entrada`, entre `ok_or(GafeteRequerido)?` y el chequeo existente de ocupación:
```rust
match self.gafetes.buscar_por_numero(numero)? {
    None => return Err(RegistroIngresoServiceError::GafeteNoRegistrado),
    Some(g) if g.estado != EstadoGafete::Disponible => {
        return Err(RegistroIngresoServiceError::GafeteNoDisponible(g.estado));
    }
    Some(_) => {}
}
// luego sigue el chequeo existente de buscar_ingreso_activo_por_gafete → GafeteOcupado
```
Catálogo primero (¿existe y está disponible?), ocupación después (¿está en uso ahora?) — son precondiciones en orden de especificidad creciente.

**Impacto real de cambiar la firma (verificado, más grande de lo que parece a primera vista):** no son ~5 sitios — hay **~40 call sites** de `RegistroIngresoService::new(...)` repartidos en `src/application/accesos.rs` (3) y en `tests/{registro_ingreso_service.rs,preparacion_ingreso.rs,flujo_integracion.rs,historial_inmutable.rs}` (~37 entre todos). Es mecánico (agregar tercer argumento) pero cada test también necesita una instancia de `SqliteGafeteRepository` en scope — tratarlo como su propio paso aislado (paso 4 del orden de implementación) para no mezclarlo con la lógica nueva.

## 4. Aviso no bloqueante de deuda

`PreparacionIngreso` (mismo archivo) gana `pub gafetes_deuda: Vec<i64>` (números que el contratista debe actualmente — `Vec` y no `Option<i64>` porque nada impide más de una deuda simultánea). `preparar_ingreso` lo llena con `self.gafetes.deuda_de_contratista(contratista.id)?` antes de construir el resultado — puramente informativo, ningún `Err` nuevo.

`src/tui/nuevo_ingreso/state.rs`: sin cambios — `PreparacionIngreso` ya viaja completa en `state.preparacion`, así que `gafetes_deuda` llega gratis. `puede_continuar` no debe mirar este campo (no bloquea).

`src/tui/nuevo_ingreso/render.rs`, etapa `Formulario` (después de la línea de "Tipo de ingreso", antes del salto en blanco): si `!p.gafetes_deuda.is_empty()`, agregar línea `"⚠ Este contratista debe el gafete #N[, #M...]"` con estilo `theme.warning()`. El cálculo de alto del panel ya usa `lineas.len()` dinámico, no requiere tocar layout. (Se muestra en `Formulario`, no en `Buscar`, porque el dato sólo existe tras preparar — traerlo antes exigiría un JOIN extra en cada fila de la búsqueda por un dato que sólo importa en el momento de decidir.)

## 5. Capa de datos

- `src/database/queries/gafetes.rs` (nuevo, lectura): `GafeteResumen{id,numero,estado,contratista_deudor_id,contratista_deudor_nombre,fecha_marcado_perdido}`, `FiltroGafetes{numero,estado}` (sin `texto`/límites — el catálogo completo, decenas de filas, se trae entero, igual criterio que `activos`), trait `GafetesQuery::buscar`, `SqliteGafetesQuery<'a>`.
- `src/database/repositories/gafete_repository.rs` (nuevo, escritura): trait `GafeteRepository{crear, buscar_por_id, buscar_por_numero, dar_de_baja, marcar_perdido, resolver, deuda_de_contratista}`, `SqliteGafeteRepository<'a>`. Un solo trait para lectura+escritura (a diferencia de Empresas) porque el catálogo es chico y no hay una proyección cara que justifique separarlos.
- `src/database/queries/gafetes_incidentes.rs` (nuevo, espejo de `auditoria_contratistas.rs`): trait `GafetesIncidentesWriter{registrar_perdido, registrar_resuelto}`, `SqliteGafetesIncidentes<'a>`.
- `src/tui/gafetes/filtros.rs` (nuevo, mismo patrón que `historial/filtros.rs`): resuelve `estado:disponible|perdido|de_baja` (con negación) y `numero:N` para la búsqueda en pantalla.

## 6. Servicios

`src/services/gafete_service.rs` (nuevo): `GafeteService<'a, R: GafeteRepository + ?Sized>` con:
- `crear_uno(numero) -> Result<i64, GafeteServiceError>` (valida `numero > 0`, mapea constraint UNIQUE a `NumeroDuplicado`).
- `crear_rango(desde, hasta) -> Result<Vec<i64>, GafeteServiceError>`: valida `desde > 0 && hasta >= desde`, itera `crear_uno` — si un número falla, el rango completo aborta (transacción del `AppCore` que lo envuelve garantiza atomicidad, no alta parcial silenciosa).
- `dar_de_baja(id)`: sólo si `estado == Disponible`, si no `EstadoInvalido`.
- `marcar_perdido(incidentes: &W, id, contratista_id, usuario_id, ahora)`: sólo si `Disponible`; actualiza `gafetes` y escribe incidente `PERDIDO`.
- `resolver(incidentes: &W, id, motivo, usuario_id, ahora)`: sólo si `Perdido`; vuelve a `Disponible`, limpia deudor, escribe incidente `RESUELTO`.

`GafeteConsultaService` trivial (delega a `GafetesQuery::buscar`), mismo patrón que `EmpresaConsultaService`.

## 7. `src/application/gafetes.rs` (nuevo)

Mismo esqueleto que `catalogos.rs`: cada función abre `Transaction::new_unchecked(..., Immediate)`, valida `verificar_actor_activo` (sin `rol.puede(...)` — cualquier operador), opera, comitea:
`buscar_gafetes`, `crear_gafete`, `crear_gafetes_rango`, `dar_de_baja_gafete`, `marcar_gafete_perdido`, `resolver_gafete`. Registrar `mod gafetes;` en `src/application/mod.rs`.

## 8. Pantalla `src/tui/gafetes/{mod.rs,state.rs,render.rs,tests.rs,filtros.rs}`

Clonar el esqueleto de `src/tui/empresas/` (referencia directa).

`state.rs`:
- `ModoGafetes{Normal, Busqueda{texto:TextInput}, Alta(FormularioAlta), MarcarPerdidoBuscarDeudor(BuscarDeudor), ConfirmacionResolver{gafete_id,numero,motivo}, ConfirmacionBaja{gafete_id,numero}}`.
- `FormularioAlta{modo:Individual|Rango, numero, desde, hasta: TextInput, error}` — validación de rango en dos capas: tope defensivo en UI (`hasta-desde <= 200`, evita mandar miles de filas por typo) y validación real/atómica en el servicio.
- `BuscarDeudor{gafete_id,numero,texto:TextInput,resultados:Vec<ContratistaResumen>,seleccion}` — clona el patrón de búsqueda de `nuevo_ingreso::ModoBuscarIngreso` (debounce 120ms, `AppCore::buscar_contratistas`), acotado a este sub-flujo (no reutiliza `NuevoIngresoState` completo).
- `AccionGafetes{Ninguna, Volver, Buscar, CrearUno, CrearRango, DarDeBaja, BuscarDeudor, MarcarPerdido, Resolver}` — `handle_key` nunca toca la DB, sólo devuelve la acción (igual que Empresas).
- Teclas en `Normal`: `/` buscar, `N` alta (Tab alterna Individual/Rango), `B` baja (sólo si `Disponible`), `P` marcar perdido (sólo si `Disponible`, abre búsqueda de deudor), `R` resolver (sólo si `Perdido`; `1`=Pagado/`2`=Aparecido arman la confirmación directo, sin tercer nivel de menú).
- `GafetesState` gana dos `Debounce` (búsqueda principal + sub-buscador de deudor) y dos métodos `tick`/`tick_deudor`.

`render.rs`: `ScreenShell` + tabla maestro-detalle (`master_detail_areas`), columnas NÚMERO (display `{:02}`, dato sigue `i64`)/ESTADO/DEUDOR; panel de detalle en `Perdido` muestra deudor + fecha (de `gafetes_incidentes`, vía `fecha_marcado_perdido` proyectado en `GafeteResumen`).

## 9. Menú y `Vista`

- `src/tui/menu_principal/state.rs`: `OpcionMenu::GestionGafetes` agregado a `TODAS` (`[Self; 12]`). **Atajo `G`** (no un dígito) — evita correr los atajos numéricos ya memorizados de Usuarios(6)/Auditoría(7)/Respaldos(8)/CambiarPassword(9). `visible_para`: no agregar brazo, cae al `_ => true` por defecto (visible para todos los roles).
- `src/tui/app.rs`: `Vista::GestionGafetes` nueva; campo `gafetes: GafetesState` en `App`; rama de render, rama de `handle_key`, rama de apertura desde menú (dispara `solicitar_carga`), y `tick()`/`tick_deudor()` enganchados en el loop principal — mismo patrón que las demás pantallas ya conectadas ahí.
- `src/tui/app/actions/gafetes.rs` (nuevo archivo, en vez de mezclarlo en `catalogos.rs` que ya es "Contratistas y Empresas"): `procesar_accion_gafetes`, mismo patrón exacto que `procesar_accion_empresas` (mapea error con `mensaje_gafete`, llama `completar_*`, re-despacha `Buscar` tras mutar para refrescar la fila tocada).

## 10. Pruebas

- Unit `gafete_service.rs`: transiciones válidas/inválidas de estado, alta duplicada, rango inválido, rango con un número ya existente aborta completo.
- Unit `registro_ingreso_service.rs` (extender): `registrar_entrada` con gafete no registrado / perdido / de_baja / ocupado; `preparar_ingreso` propaga `gafetes_deuda` sin bloquear.
- Integración `tests/registro_ingreso_service.rs`: mismos casos contra SQLite real (confirma que los `CHECK` no interfieren).
- Integración de query `gafetes.rs`: `EXPLAIN QUERY PLAN` usa `idx_gafetes_estado`.
- Unit `tui/gafetes/state.rs`/`tests.rs`: transiciones de modo, validación de rango, prefijo `✓` sólo en éxito.
- Unit `nuevo_ingreso` (extender): con deuda no vacía, `etapa` sigue en `Formulario` (no bloquea).
- Snapshots (si aplica, mismo mecanismo que las demás pantallas): `gafetes::render` en sus modos principales.

## 11. Orden de implementación (cada paso compila y pasa `cargo test` antes del siguiente)

1. Schema (`MIGRACION_13`) + `src/models/gafete.rs`.
2. Capa de datos: `queries/gafetes.rs`, `repositories/gafete_repository.rs`, `queries/gafetes_incidentes.rs` — sin consumidores aún, sólo tests de integración propios.
3. `services/gafete_service.rs` + `GafeteServiceError` — tests unitarios de transición de estado.
4. Cambio de firma de `RegistroIngresoService` (paso aislado, toca ~40 call sites de `new(...)` en `application/accesos.rs` + 4 archivos de `tests/`) + validación de catálogo en `registrar_entrada` + `gafetes_deuda` en `preparar_ingreso` + nuevas variantes de error + mapeo en `error_messages.rs`. Suite completa en verde antes de seguir.
5. `src/application/gafetes.rs` + registrar en `application/mod.rs`.
6. Aviso de deuda en `nuevo_ingreso/render.rs` (sin tocar `state.rs`).
7. Pantalla `src/tui/gafetes/` completa (state/render/tests/filtros).
8. Enganche en menú/`app.rs`/`actions/gafetes.rs` — primer punto en que la pantalla es alcanzable; probar manualmente con `cargo run`.
9. `cargo fmt` + Clippy estricto (`-D warnings`) + suite completa, cierre de todo el trabajo.
10. Actualizar `docs/pendientes.md` con una sección nueva y checkboxes por los pasos 1-8, marcando `[x]` lo resuelto en el mismo commit (regla explícita ya existente en ese documento).

## Verificación end-to-end

`cargo test` (suite completa) tras cada paso marcado arriba. Manual: `cargo run`, entrar como operador, dar de alta el rango 1-25 en Gestión de Gafetes (`G`), dar un ingreso PRAIND con gafete `05` (éxito), repetir con `05` otra vez (error "ocupado"), probar un número fuera de catálogo como `99` (error "no existe"), marcar `05` como perdido con un contratista, iniciar un nuevo ingreso para ese contratista y confirmar que aparece el aviso sin bloquear, resolver la deuda ("apareció") y confirmar que `05` vuelve a estar disponible para asignar.
