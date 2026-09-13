# Gafetes como catálogo compartido (contratista + visita) y aviso en vivo de citas — implementado 2026-09-13

> Implementado y aplicado (código, tests, migraciones locales y de
> Supabase ya corridas contra `control-acceso-nube`). Nace de una sesión
> de diseño sobre por qué `gafetes` hoy solo modela el pool de
> contratistas y qué hacía falta para que visitas empiece a usar el
> mismo catálogo sin que ambos módulos se conozcan entre sí. De paso se
> encontró y se cerró un segundo hueco, más chico y sin relación de
> código: las citas creadas en la web no avisaban en vivo a los
> dispositivos, solo llegaban por el pulso periódico de sync (~2 min).
> La UI del desktop (`Gafetes.tsx` y compañía) también se actualizó al
> contrato nuevo en la misma rama. Ver `docs/planes-implementados/plan-control-visitas.md`
> ("Reglas de negocio de visita -- auditoría completa") para el trabajo
> de seguimiento de la sesión siguiente: los mismos dos chequeos
> cruzados entre sitios/dispositivos que ya tenía contratista, ahora
> espejados para visita. Queda de referencia -- es la base para decidir,
> sesión por sesión, igual que `plan-control-visitas.md` y
> `plan-sesion-unica-dispositivos.md`.

## Por qué existe este documento

`gafetes` hoy es un catálogo real (no un string suelto): tiene estado
(`Disponible/Perdido/DeBaja`), historial de incidentes append-only, y
transiciones validadas en `domain::gafete`. Pero solo lo usa contratista —
visita tiene su propia columna `movimientos_visita.gafete_numero`, un
entero suelto sin relación con el catálogo real. El código ya se
autodocumenta este hueco (`CitaService::registrar_entrada`,
`src/services/cita_service.rs`, comentario explícito citando
`docs/planes-implementados/plan-control-visitas.md`).

Al retomar esto se encontró que **el diseño de fondo ya estaba decidido y
escrito** en `docs/planes-implementados/plan-control-visitas.md:108-138`:
los gafetes de contratista y visita son objetos físicos distintos que
repiten numeración ("el 7 verde y el 7 rojo coexisten"), así que la
unicidad pasa de `numero` solo al par `(numero, tipo)`, con columnas FK
tipadas (no un campo genérico) para no perder integridad referencial real.
Este documento retoma esa decisión, la lleva a un plan concreto capa por
capa, y resuelve dos cosas que el doc original dejaba abiertas: cómo
desacoplar `GafeteService` de `ContratistaRepository` sin sacrificar esa
integridad, y cómo se llama el campo de "a quién se le asignó cuando se
perdió" (el usuario pidió sacar la palabra "deudor" — esta app no lleva
control de dinero, solo trazabilidad de un objeto físico).

## Decisiones ya tomadas (explícitas del usuario)

### 1. Gafete se aísla como recurso compartido, no como algo de contratista

Patrón elegido (validado contra literatura de DDD): **hub-and-spoke /
Open Host Service**, no shared-kernel ni eventos. `gafete_service` y
`domain::gafete` son el hub — no importan `Contratista` ni `CitaVisitante`.
Contratista y visita son spokes que solo le hablan al hub, nunca entre
sí. La verificación de "¿existe ese id de verdad?" (hoy hecha dentro de
`GafeteService::marcar_perdido` contra `ContratistaRepository`) se saca
del servicio genérico y se mueve a la capa de aplicación (`AppCore`), que
sí puede conocer ambos tipos sin que el servicio los conozca.

### 2. Tres columnas FK tipadas, no un campo genérico

Se seguía el precedente ya escrito en `plan-control-visitas.md` en vez del
campo único que se planteó al principio de la conversación:
`contratista_portador_id` (FK real a `contratistas`) y
`visita_portador_id` (FK real a `cita_visitantes`) — sin
`proveedor_portador_id` todavía porque no existe tabla `proveedores`. Un
`CHECK` obliga a que solo la columna del `tipo` correspondiente pueda
tener valor. Se descartó explícitamente un `portador_tipo` + `portador_id`
sin FK real porque el proyecto entero valora integridad referencial real
(tablas STRICT, `CHECK`s por todos lados) y ese campo la perdería.

### 3. El campo de categoría se llama `tipo`, no `categoria`

Para no introducir un segundo nombre para el mismo concepto que ya usa
`tipo_ingreso`/`TipoIncidenteGafete` en el resto del código. Valores:
`CONTRATISTA`, `VISITA`, `PROVEEDOR` (el `CHECK` ya permite las 3 a
futuro; solo las primeras dos tienen columna de portador hoy).

### 4. Se saca la palabra "deudor"

La app no lleva control de dinero — el campo es sobre trazabilidad de un
objeto físico (a quién se le asignó la última vez, para saber a quién
reclamarle si aparece). `contratista_deudor_id` pasa a
`contratista_portador_id` en ambos niveles (SQLite local y espejo
Supabase).

### 5. Visita empieza a validar contra el catálogo real al hacer check-in

Hoy `CitaService::registrar_entrada` acepta cualquier
`gafete_numero: Option<i64>` sin tocar `gafetes`. Pasa a seguir el mismo
patrón de 3 pasos que ya usa `RegistroIngresoService::registrar_entrada`
para contratista: `buscar_por_numero` → `domain::gafete::validar_para_asignar`
→ chequeo de ocupación contra su propia tabla de movimientos.

### 6. Va en el mismo documento el fix de realtime en citas

Hallazgo relacionado pero de código independiente: las citas creadas en
la web solo llegan a los dispositivos por el pulso periódico de sync
(~2 min), no en vivo, porque el trigger de aviso ya existente
(`private.emitir_cambio_nube_sitio`, usado por `usuarios` y
`movimientos_visita`) nunca se enganchó en la familia de tablas de citas.
Se agrupa acá porque el usuario pidió "ambos en un mismo plan", aunque no
comparten una sola línea de código con el rediseño de gafetes.

## Contexto técnico verificado (no asumido)

- **Gafete NO cruza a `mobile/rust-core` vía UniFFI** — confirmado por
  grep exhaustivo: mobile solo tiene `gafete_numero: Option<i64>` como
  entero suelto en records espejo, sin importar `Gafete`/`EstadoGafete`/
  `gafete_service`/`domain::gafete` en ningún lado. Cero regeneración de
  bindings `.kt` para este trabajo.
- **La unicidad remota real hoy es un constraint de Postgres**
  (`gafetes_sitio_id_numero_key UNIQUE (sitio_id, numero)`, agregado en
  `20260906031836_reset_datos_prueba_y_unicidad_catalogo.sql` porque el
  upsert por `id` (UUID por dispositivo) no detectaba duplicados
  cross-dispositivo). El mecanismo de sync (`on_conflict=sitio_id,numero`,
  `Prefer: resolution=merge-duplicates`) es **last-write-wins**, no
  detección/rechazo de conflictos — el POST que llega segundo pisa al
  primero. Extender a `(sitio_id, numero, tipo)` mantiene la misma
  semántica, solo más fina.
- **El espejo remoto de `gafetes` NO tiene el CHECK cruzado** que sí tiene
  el local (`estado='PERDIDO' ⇔ portador seteado`) — confirmado en
  `supabase/tests/gafetes_autorizacion.sql`, que inserta `PERDIDO` sin
  deudor para probar solo RLS. Decisión: no agregar ese CHECK cruzado del
  lado remoto tampoco — SQLite local sigue siendo la fuente de verdad de
  reglas de negocio; el mirror remoto es estructural (RLS + unicidad para
  upsert), no de validación de negocio.
- **`CitaRepository` no tiene ningún método para buscar un solo
  `cita_visitante` por id** — hace falta agregarlo, no es trabajo
  opcional.
- **El trigger de realtime (`cita_sitios`) no requiere ningún cambio de
  cliente** — confirmado leyendo `desktop/src/nubeRealtime.ts` (solo
  filtra por `dispositivo_id`, nunca por `table`) y
  `mobile/android/.../NubeRealtime.kt` (ni siquiera filtra por
  `dispositivo_id`, reacciona a cualquier aviso). Wirear el trigger en la
  DB alcanza.
- **`registro_ingresos.gafete_numero` deliberadamente no es FK a
  `gafetes`** (comentario existente: "hay filas históricas con números
  que el catálogo no tiene por qué conocer") — la misma razón aplica
  simétricamente a `movimientos_visita.gafete_numero`: no se agrega FK,
  la validación es en tiempo de escritura vía `CitaService`, igual que
  contratista.

## Plan de implementación — Feature 1: catálogo de gafetes compartido

### 0. Tipos nuevos — `src/models/gafete.rs`

```rust
pub enum TipoGafete { Contratista, Visita, Proveedor }
// as_str_sql/from_str_sql, igual que EstadoGafete: "CONTRATISTA"/"VISITA"/"PROVEEDOR"

pub enum PortadorGafete { Contratista(i64), Visita(i64) }
// sin Proveedor(i64) todavía -- no hay tabla proveedores
```

`Gafete` gana `tipo: TipoGafete`, `contratista_portador_id: Option<i64>`,
`visita_portador_id: Option<i64>` (reemplaza el único
`contratista_deudor_id` de hoy), más un helper `fn portador(&self) ->
Option<PortadorGafete>`.

### 1. Migración local — `src/database/schema.rs`

- `SCHEMA_VERSION` 34 → 35.
- `MIGRACION_35`: recrea `gafetes` (patrón `_nueva`/`INSERT...SELECT`/
  `DROP`/`RENAME`, igual que MIGRACION_15/27/30) con:
  - `tipo TEXT NOT NULL CHECK (tipo IN ('CONTRATISTA','VISITA','PROVEEDOR'))`
  - `contratista_portador_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT`
  - `visita_portador_id INTEGER REFERENCES cita_visitantes(id) ON DELETE RESTRICT`
  - `CHECK` de tipo↔columna (solo la del tipo correspondiente puede tener valor)
  - `CHECK` de estado↔portador generalizado (`PERDIDO` ⇔ alguna de las dos seteada)
  - `UNIQUE(numero, tipo)` reemplaza `UNIQUE(numero)`
  - Migra filas existentes con `tipo='CONTRATISTA'`, `contratista_portador_id = contratista_deudor_id`
  - Recrea también `gafetes_incidentes` en la misma migración (ver punto 4), porque es tabla hija (`ON DELETE RESTRICT`) y hay que tocarla igual.
  - Sigue el patrón de `aplicar_migracion_15` de `PRAGMA foreign_keys = OFF/ON` + `PRAGMA foreign_key_check` al final (reusa `SchemaError::MigracionStrictReferenciasInvalidas`, no hace falta variante nueva).
- `tests/migraciones.rs`: nuevo test `migracion_35_...` seedeando una fila
  `PERDIDO` pre-migración con `contratista_deudor_id`, corriendo
  `initialize_database`, y comprobando: `foreign_key_check` limpio, la fila
  migró con `tipo='CONTRATISTA'` y el id preservado, `(numero, tipo)` es la
  nueva unicidad (mismo número + tipo distinto = ok; mismo número + mismo
  tipo = falla), y el `CHECK` tipo↔columna rechaza una fila `VISITA` con
  `contratista_portador_id` seteado.

### 2. Dominio — `src/domain/gafete.rs`

**Sin cambios.** Ya es genérico (`puede_marcarse_perdido`/
`puede_resolverse`/`validar_para_asignar` solo tocan `EstadoGafete`/
`Option<&Gafete>`).

### 3. Repositorio — `src/database/repositories/gafete_repository.rs`

```rust
trait GafeteRepository {
    fn crear(&self, numero: i64, tipo: TipoGafete) -> Result<i64, DatabaseError>;
    fn buscar_por_numero(&self, numero: i64, tipo: TipoGafete) -> Result<Option<Gafete>, DatabaseError>;
    fn marcar_perdido(&self, id: i64, portador: PortadorGafete) -> Result<(), DatabaseError>;
    // buscar_por_id, dar_de_baja, resolver, deuda_de_contratista: forma sin cambios
}
```

`marcar_perdido` internamente hace `match portador` para decidir qué
columna llenar. `resolver` limpia ambas columnas (solo una estaba seteada
nunca). Tests: agregar `TipoGafete::Contratista` a las llamadas
existentes; nuevo test probando que `crear(5, Contratista)` y
`crear(5, Visita)` coexisten sin colisión.

### 4. `gafetes_incidentes` — `src/database/queries/gafetes_incidentes.rs`

**Necesita su propio cambio, no es opcional.** Una vez que
`GafeteService` es un solo servicio compartido sobre toda la tabla
`gafetes`, tiene que poder marcar-perdido cualquier fila, sea
`CONTRATISTA` o `VISITA` — si no, la mitad del catálogo no tendría cómo
registrar su pérdida. Se agrega `visita_portador_id INTEGER REFERENCES
cita_visitantes(id) ON DELETE RESTRICT` (recreando la tabla en la misma
MIGRACION_35, junto a `gafetes`), y el `CHECK` existente se generaliza de
"contratista_id obligatorio si PERDIDO" a "alguna de las dos columnas
obligatoria si PERDIDO". **No** se agrega una columna `tipo` propia en
esta tabla — sería redundante (`gafete_id` ya permite hacer join a
`gafetes.tipo` cuando hace falta). `GafetesIncidentesWriter::registrar_perdido`
pasa a recibir `portador: PortadorGafete` en vez de `contratista_id: i64`.

### 5. Servicios

**`GafeteService`** (`src/services/gafete_service.rs`) pierde el genérico
`C: ContratistaRepository` — queda `GafeteService<'a, R: GafeteRepository>`.
`marcar_perdido` recibe `portador: PortadorGafete` en vez de
`contratista_id: i64`, y ya no valida que el id exista (esa
responsabilidad se mueve a `AppCore`, punto 6).

**`CitaService`** (`src/services/cita_service.rs`) gana un tercer genérico
`G: GafeteRepository`. `registrar_entrada` agrega el mismo chequeo de 3
pasos que ya tiene `RegistroIngresoService`: `buscar_por_numero(numero,
TipoGafete::Visita)` → `validar_para_asignar` → ocupación contra
`buscar_activo_por_gafete`. A diferencia de contratista, el gafete de
visita sigue siendo opcional (no hay `requiere_gafete()` para visitantes).
Nuevas variantes en `CitaServiceError`: `GafeteNoRegistrado`,
`GafeteNoDisponible(EstadoGafete)`.

Recomendado: separar `CitaService` en un `CitaConsultaService` (solo
`verificar_check_in`, no necesita `gafetes`) + `CitaService` (las
operaciones que sí escriben), igual que ya existe la separación
`RegistroIngresoConsultaService`/`RegistroIngresoService` — evita construir
un `SqliteGafeteRepository` sin uso en cada preview de check-in.

### 6. Capa de aplicación

**`src/application/gafetes.rs`**: acá vive ahora la validación de
existencia que se sacó del servicio genérico. `marcar_gafete_perdido` se
divide en `marcar_gafete_perdido_contratista(id, contratista_id)` (valida
contra `ContratistaRepository`) y `marcar_gafete_perdido_visita(id,
cita_visitante_id)` (valida contra el nuevo método de `CitaRepository`,
ver abajo). Cada uno arma su propio `PortadorGafete` antes de llamar a
`GafeteService::marcar_perdido`.

**`src/database/repositories/cita_repository.rs`**: nuevo método
`buscar_visitante_por_id(id: i64) -> Result<Option<CitaVisitante>,
DatabaseError>` — no existe ninguna forma hoy de buscar un solo
visitante por id, hace falta agregarlo.

**`src/application/citas.rs`**: `registrar_entrada_visita` construye
`SqliteGafeteRepository` y lo pasa al `CitaService`, igual que
`accesos.rs::registrar_ingreso` ya hace para contratista.

### 7. Tauri (desktop, único frontend con superficie de gafetes)

- `dto/gafetes.rs`: nuevo `TipoGafeteEntrada`, `FiltroGafetesEntrada` gana
  `tipo: Option<TipoGafeteEntrada>`.
- `comandos/gafetes.rs`: `crear_gafete`/`crear_gafetes_rango` ganan
  parámetro `tipo`; `marcar_gafete_perdido` se separa en
  `marcar_gafete_perdido_contratista`/`marcar_gafete_perdido_visita`
  (registrar ambos en `lib.rs::invoke_handler`).
- `comandos/citas.rs`: sin cambio de firma — el error nuevo ya fluye por
  `mensaje_cita`.
- El lado TS (`desktop/src/...`) va a necesitar tipos equivalentes, pero
  el rediseño visual de esa pantalla es tema aparte (ya acordado con el
  usuario) — queda como seguimiento, no bloquea este plan de backend.

### 8. Supabase (espejo)

Nueva migración `agrega_tipo_y_portador_visita_a_gafetes.sql`:
- `alter table gafetes add column tipo text not null default 'CONTRATISTA' check (tipo in ('CONTRATISTA','VISITA','PROVEEDOR'))`
- Renombra `contratista_deudor_id`/`_nombre` → `contratista_portador_id`/`_nombre`
- Agrega `visita_portador_id uuid references cita_visitantes(id)` + `visita_portador_nombre text`
- `drop constraint gafetes_sitio_id_numero_key` → `add constraint ... unique (sitio_id, numero, tipo)`
- **Sin** el CHECK cruzado estado↔portador (decisión explícita, ver
  contexto verificado arriba — el mirror no valida reglas de negocio).
- RLS: sin cambios (las políticas ya filtran solo por `sitio_id`).
- `supabase/tests/gafetes_autorizacion.sql`: agregar aserción de que
  `(sitio_id, numero)` ya no colisiona entre `tipo` distintos, y que
  `(sitio_id, numero, tipo)` sigue colisionando igual.

### 9. `src/nube/sincronizacion.rs`

- **Push** (`construir_cuerpo_gafete`/`enviar_gafete`): el `SELECT` suma
  `LEFT JOIN cita_visitantes cv ON cv.id = g.visita_portador_id`; el JSON
  suma `tipo`/`visita_portador_id`/`visita_portador_nombre`; el
  `on_conflict` pasa a `sitio_id,numero,tipo`.
- **Pull** (`guardar_gafetes`): nuevo índice uuid→id local de
  `cita_visitantes` (más simple que el de contratistas — esta tabla es
  puramente de sincronización, sin creación local previa que compita).
  `ON CONFLICT(numero)` pasa a `ON CONFLICT(numero, tipo)`. Un gafete
  `VISITA`+`PERDIDO` cuyo `cita_visitante` todavía no llegó localmente
  queda `pendiente` (mismo mecanismo que ya existe para contratista) y se
  resuelve en el siguiente ciclo — sin necesidad de reordenar las llamadas
  existentes (`recibir_catalogo_del_sitio` ya corre antes de
  `recibir_citas_del_sitio` en todos los call sites actuales; el
  mecanismo de "pendiente, no avanzar la marca de agua" ya tolera eso).

## Plan de implementación — Feature 2: aviso en vivo de citas

Cambio chico, aislado, sin tocar Rust ni TypeScript.

1. Nueva migración Supabase, copia exacta del patrón de
   `avisa_cambio_nube_en_usuarios.sql`:
   ```sql
   drop trigger if exists cita_sitios_emitir_cambio_nube on public.cita_sitios;
   create trigger cita_sitios_emitir_cambio_nube
   after insert or update or delete on public.cita_sitios
   for each row execute function private.emitir_cambio_nube_sitio();
   ```
2. Se engancha en `cita_sitios` (no en `citas` ni `cita_visitantes`) porque
   es la única tabla de la familia con `sitio_id` propio por fila, y
   `crear_cita_anfitrion` inserta ahí (una fila por sitio) en la misma
   transacción que crea la cita — confirmado leyendo esa RPC.
3. Confirmado que no hace falta tocar `desktop/src/nubeRealtime.ts` ni
   `mobile/.../NubeRealtime.kt` — ninguno de los dos filtra por `table`
   en el payload del broadcast.
4. Verificación manual post-deploy: insertar/borrar una fila de prueba en
   `cita_sitios` y confirmar que llega el broadcast `cambio_nube` al canal
   `sitio:<ese sitio_id>`.
5. Sin cambios en `supabase/tests/realtime_autorizacion.sql` (agnóstico de
   tabla) ni en `cita_visitantes_toca_cita` (mecanismo de watermark
   incremental, no relacionado).

## Riesgos / cosas a no pasar por alto al implementar

- El cambio de `gafetes_incidentes` (punto 4) **no es opcional ni
  postergable** — está acoplado al mismo `MIGRACION_35` que recrea
  `gafetes`, porque es tabla hija con `ON DELETE RESTRICT`.
- `movimientos_visita.gafete_numero` **no gana FK** — validación en
  tiempo de escritura vía `CitaService`, igual que contratista.
- `deuda_de_contratista` queda igual en forma (solo cambia el nombre de
  columna en su SQL interno) — no hay pedido de un equivalente para
  visita, no inventar alcance nuevo.
- Los tipos TS del desktop quedan desactualizados hasta que se haga el
  rediseño de UI acordado por separado — no es un olvido de este plan,
  es alcance explícitamente pospuesto.

## Archivos críticos

- `src/models/gafete.rs`, `src/domain/gafete.rs`
- `src/database/schema.rs` (MIGRACION_35)
- `src/database/repositories/gafete_repository.rs`,
  `src/database/repositories/cita_repository.rs`
- `src/database/queries/gafetes.rs`, `src/database/queries/gafetes_incidentes.rs`
- `src/services/gafete_service.rs`, `src/services/cita_service.rs`, `src/services/error.rs`
- `src/application/gafetes.rs`, `src/application/citas.rs`
- `desktop/src-tauri/src/dto/gafetes.rs`, `desktop/src-tauri/src/comandos/gafetes.rs`
- `src/nube/sincronizacion.rs`
- `supabase/migrations/` (2 archivos nuevos: tipo+portador en gafetes, trigger en cita_sitios)
- `tests/migraciones.rs`, `supabase/tests/gafetes_autorizacion.sql`

## Verificación

- `cargo test` completo (migraciones, repositorio, servicio, aplicación) en verde.
- `cargo clippy --all-targets` limpio.
- Prueba manual: crear un gafete `CONTRATISTA` número 7 y uno `VISITA`
  número 7 sin colisión; marcar perdido uno de cada tipo y confirmar que
  el historial de incidentes muestra el portador correcto; hacer
  check-in de una visita con un gafete no registrado y confirmar el
  mensaje de error nuevo.
- Prueba manual del fix de realtime: crear una cita desde `web-visitas`
  apuntando a un sitio y confirmar que el desktop de ese sitio dispara
  sync casi inmediato (no esperar los ~2 minutos del pulso periódico).
