# Revisión de carreras y huecos — cambios del 2026-10-03

Fecha: 2026-10-04. Rama revisada: `feat/ingreso-por-correo` (todo lo que entró
el 2026-10-03: ingreso "por correo", "Sin gafete", ingreso único entre
unidades para contratistas, proveedores y KOF, sesión única sin empates,
notificaciones push y los cambios del panel). Los arreglos viven en la rama
`fix/carreras-ingresos-y-gafetes`, que sale de esa misma rama.

Este documento explica, para cada hallazgo, **qué pasaba**, **por qué
pasaba**, **cómo se arregló** y **cómo se comprobó**. Los arreglos se
hicieron en dos rondas sobre la misma rama: primero los cuatro más urgentes y
después el resto de los bugs. Cada arreglo es un commit propio, con su
explicación completa en el mensaje.

## Resumen

| # | Hallazgo | Gravedad | Ronda | Commit |
|---|----------|----------|-------|--------|
| 1 | La cola podía mandar el "cerrar" antes que el "crear": la persona quedaba adentro para siempre en la nube | Alta | 1 | `fix(nube): la cola ya no manda un cierre…` |
| 2 | El gafete de visita se controlaba en un solo sentido (correo → visita, no visita → correo) y sin barrera en la nube | Media | 1 | `fix(visitas): el check-in ya no entrega…` y `fix(supabase): aviso de duplicado…` |
| 3 | Un ingreso rechazado por chocar con el otro equipo de la MISMA unidad no avisaba a nadie | Media | 1 | `fix(supabase): aviso de duplicado…` |
| 6 | El envío de push podía borrar el token recién renovado de un teléfono | Baja | 1 | `fix(push): borrar tokens muertos por token…` |
| — | `supabase/tests/panel_buscar_movimientos.sql` dejó de correr con el índice único del 2026-10-03 | Baja (test) | 1 | `test(supabase): panel_buscar_movimientos…` |
| 4 | Visitas: "¿ya está adentro en otra unidad?" estaba ciego por RLS, y no había garantía en la nube | Media | 2 | `fix(visitas): "¿ya está adentro?" vuelve a ver…` |
| 7 | Un mismo gafete de visita podía quedar abierto en dos visitas (sin índice en la nube) | Media | 2 | `fix(visitas): un gafete de visita ya no puede…` |
| 5 | Una persona podía estar adentro a la vez como contratista, proveedor y por correo | Media | 2 | `fix: una persona ya no puede estar adentro por dos vías…` |
| 8 | Un ingreso del otro equipo cerrado a mano podía "revivir" en la caché | Baja | 2 | `fix(nube): un ingreso del otro equipo cerrado a mano…` |
| 9 | `registrar_token_push`: dos equipos con el mismo token a la vez → 409 | Baja | 2 | `fix(push): registrar el mismo token desde dos equipos…` |

Queda una sola cosa abierta, que no es un bug sino una función nueva: el
historial del panel web no muestra ingresos por correo ni de proveedores
(ver el final).

---

## 1. El "cerrar" salía antes que el "crear" (cola de salida)

### Qué pasaba

Escenario concreto, con cualquier tipo de ingreso (contratista, proveedor,
por correo, préstamo KOF, visita, salida de ruta):

1. 10:00 — el operador registra el ingreso. La cola local guarda una fila
   `crear`.
2. 10:00 — la sincronización intenta mandarla y falla (un corte de red, un
   502 de Supabase). La fila queda `pendiente` con `intentos = 1`, así que
   su próximo intento es a las 10:15 (backoff de 15 minutos por intento).
3. 10:05 — la persona sale. La cola guarda una fila `cerrar` con
   `intentos = 0`: su próximo intento es inmediato.
4. 10:06 — la siguiente sincronización le pide a la cola lo que ya venció
   (`pendientes()`): **sólo el `cerrar`**. Lo manda como
   `PATCH /rest/v1/<tabla>?id=eq.<uuid>&hora_salida=is.null`. En la nube la
   fila todavía no existe, así que el `PATCH` no toca nada, pero PostgREST
   responde 204 igual y la fila de la cola queda `enviado`.
5. 10:15 — se manda el `crear`. La fila entra a la nube **abierta**, y ya
   no hay nada en la cola que la vaya a cerrar.

Consecuencias:

- El panel ("Adentro ahora") muestra a la persona adentro para siempre.
- Por el índice único de cédula activa del 2026-10-03
  (`ingresos_contratista_activo_idx`, `ingresos_proveedor_cedula_activa_idx`,
  `ingresos_correo_cedula_activa_idx`), **esa persona ya no puede volver a
  entrar en ninguna unidad**: la verificación en vivo la ve adentro y el
  `INSERT` recibe 409.
- El equipo que la registró no puede corregirlo: localmente la salida ya
  está hecha, y la caché de "abiertos del otro dispositivo" excluye los
  registros propios, así que ni siquiera la ve. Sólo el otro equipo de la
  unidad puede cerrarla a mano.

El patrón existía antes del 2026-10-03, pero con los índices únicos nuevos
pasó de "un fantasma en el panel" a "una persona bloqueada en todas las
unidades".

### Por qué pasaba

`pendientes()` (`src/nube/sincronizacion/cola.rs`) filtra por
`proximo_intento_en <= ahora`, y `proximo_intento_en` depende de los
intentos de CADA fila. Nada relacionaba el `cerrar` con el `crear` del
mismo registro.

Se comprobó antes de arreglarlo con un test temporal: con un `crear` en
backoff y un `cerrar` recién encolado, `pendientes()` devolvía sólo
`["cerrar"]`.

### Arreglo

`procesar_fila_individual` (el camino por el que pasan todos los cierres,
porque ningún `cerrar` admite lote) consulta antes de enviar
`cierre_espera_su_apertura`:

```sql
SELECT EXISTS(
  SELECT 1 FROM cola_salida
  WHERE entidad = ?1 AND entidad_uuid = ?2
    AND operacion <> 'cerrar' AND estado = 'pendiente'
    AND id < ?3
)
```

Si la apertura sigue pendiente, el cierre **no se envía y no se toca**: no
suma intento ni mueve `actualizado_en`, así que en la siguiente pasada
vuelve a estar listo. Decisiones:

- **Por qué sólo el `cerrar`.** Es la única operación cuyo resultado
  depende de que la fila ya exista en la nube (`PATCH` condicional). Los
  `crear`/`actualizar` son `upsert` idempotentes; frenarlos detrás de una
  fila vieja trabada habría demorado datos sin necesidad.
- **Por qué no bloquea si la apertura quedó `fallido`.** Si la nube rechazó
  la apertura (por ejemplo, el índice único), esa fila nunca va a llegar.
  El cierre sale, su `PATCH` no toca nada y la cola queda limpia, que es
  lo correcto.
- **Sin demora en la misma pasada.** `agrupar_por_entidad_y_operacion`
  conserva el orden de creación, así que el grupo del `crear` se procesa
  antes que el del `cerrar`. Si el `crear` se envía bien, la consulta ya lo
  ve `enviado` y el cierre sale en la misma sincronización.
- **No hizo falta migración local.** La subconsulta sólo recorre filas
  `pendiente` (pocas) y la cola ya tiene el índice parcial
  `idx_cola_salida_pendientes`.

### Cómo se comprobó

Tests nuevos en `src/nube/sincronizacion/tests.rs`:

- `el_cierre_no_sale_mientras_su_apertura_espera_el_reintento`: el escenario
  de arriba. No se hace ningún pedido de red y el `cerrar` queda
  `pendiente` con 0 intentos. **Sin el arreglo, este test falla**
  (comprobado desactivando la consulta).
- `apertura_y_cierre_listos_salen_en_la_misma_pasada_y_en_orden`: primero
  sale el `POST` y después el `PATCH`, en la misma pasada.
- `el_cierre_sale_si_su_apertura_quedo_fallida`.
- `solo_un_cierre_puede_esperar_a_su_apertura`: un `crear` nunca espera, y
  un cierre de otro registro no se frena.

---

## 2. Gafete de visita: control en un solo sentido y sin barrera en la nube

### Qué pasaba

El ingreso "por correo" y las visitas (módulo de citas) reparten **el mismo
catálogo físico de gafetes de visita** (`TipoGafete::Visita`), pero cada uno
guarda sus movimientos en su propia tabla:

| Dónde | Ingreso por correo revisaba visitas | Visita revisaba ingresos por correo |
|-------|:-:|:-:|
| Este equipo (SQLite) | Sí (`gafete_de_visita_en_uso`) | **No** |
| El otro equipo (consulta en vivo) | Sí | **No** (escritorio, `citas.rs`) |
| Nube (garantía final) | **No** | **No** |

Es decir: una visita podía recibir un gafete que ya tenía alguien que entró
por correo, en el mismo equipo o en el otro. Y si dos equipos trabajaban sin
conexión, la nube aceptaba las dos filas, porque
`ingresos_correo_gafete_activo_sitio_idx` sólo cubre su propia tabla (un
índice no puede abarcar dos tablas).

### Arreglo, en tres capas

1. **Local** — `MovimientoVisitaRepository::gafete_en_uso_por_ingreso_correo`
   (nuevo), y `CitaService::registrar_entrada` lo consulta junto con
   `buscar_activo_por_gafete`. Responde `CitaServiceError::GafeteOcupado`,
   el mismo error y el mismo mensaje que ya existían. Corre dentro de la
   transacción `Immediate` del check-in, así que no hay ventana entre la
   consulta y el `INSERT`.
2. **En vivo contra el otro equipo** — en el escritorio,
   `gafete_de_visita_libre_en_otro_dispositivo`
   (`desktop/src-tauri/src/comandos/citas.rs`) ahora también consulta
   `gafete_de_correo_ocupado_en_otro_dispositivo` (la función ya existía; la
   usa el ingreso por correo). El teléfono no tiene check-in de visitas.
3. **Nube** — migración
   `20261004120000_conflictos_misma_unidad_y_gafete_de_visita.sql`, parte
   (2): trigger `before insert` en `ingresos_correo` y en
   `movimientos_visita` (función
   `private.gafete_de_visita_libre_en_la_otra_tabla`) que rechaza la fila
   si el mismo gafete está abierto en la OTRA tabla de la misma unidad.
   - **Candado.** `pg_advisory_xact_lock` sobre
     `gafete_visita:<unidad>:<gafete>`, el mismo desde las dos tablas. Dos
     `INSERT` simultáneos con el mismo gafete en tablas distintas se
     serializan: el segundo espera a que el primero confirme y, como la
     consulta del trigger toma una instantánea nueva (READ COMMITTED,
     función volátil), ya ve la fila del primero.
   - **Error.** Usa `23505`, así que PostgREST responde 409. Para
     `ingresos_correo` el mensaje nombra
     `ingresos_correo_gafete_activo_sitio_idx`, que es justo lo que la cola
     del núcleo reconoce (`es_conflicto_gafete_activo`). Así la fila queda
     fallida de inmediato y quien opera recibe el aviso de choque de
     gafete, igual que cuando choca con otro ingreso por correo.
   - **`security definer`.** La consulta a la otra tabla no depende de la
     RLS de quien inserta.
   - **Filas existentes.** Sólo afecta a inserciones nuevas, y una visita sin
     gafete o una fila que entra ya cerrada no se revisan.

### Cómo se comprobó

- Rust: `gafete_en_uso_por_ingreso_correo_solo_cuenta_los_abiertos`
  (repositorio) y
  `registrar_entrada_con_gafete_en_uso_por_un_ingreso_por_correo_falla`
  (servicio).
- SQL: `supabase/tests/gafete_de_visita_entre_tablas.sql` (nuevo). Prueba el
  rechazo en los dos sentidos, que el mensaje nombre el índice, que otra
  unidad no choque, que una visita sin gafete pase y que el gafete se libere
  tras la salida. **Sin los triggers, el test falla** (comprobado borrándolos).
- Carrera real con dos sesiones de Postgres: la sesión 1 inserta una visita
  con el gafete 900 y espera 3 s antes de confirmar; la sesión 2 inserta un
  ingreso por correo con el 900 en ese lapso. La sesión 2 quedó esperando
  el candado y, al confirmar la 1, fue rechazada. Resultado: 1 visita
  abierta y 0 ingresos por correo con ese gafete.

---

## 3. Choque con el otro equipo de la misma unidad: sin aviso

### Qué pasaba

Los índices únicos de cédula activa son **globales**: cubren todas las
unidades y también los dos equipos de una misma unidad. Si el equipo A
registra a alguien sin conexión y el equipo B de la misma unidad ya lo
tenía adentro, la cola de A recibe 409 y marca la fila `fallido`. El
ingreso **sigue abierto en A**.

El aviso a quien opera lo dan `contratistas_con_conflicto_activo`,
`proveedores_con_conflicto_activo` y `correos_con_conflicto_activo` al
terminar la sincronización. Esas funciones llaman a
`*_activos_en_otras_unidades` en la nube, que filtraban
`sitio_id <> unidad del equipo`. Resultado: el choque entre los dos equipos
de la misma unidad **no avisaba nada**. Sólo subía el contador de fallos de
la cola, y ese ingreso nunca llegaba a la nube, así que se perdía del
historial. El comentario de `marcar_fallida_por_ingreso_activo` prometía un
aviso que en ese caso no ocurría.

### Arreglo

Migración `20261004120000_...`, parte (1): `create or replace` de
`contratistas_activos_en_otras_unidades`,
`proveedores_activos_en_otras_unidades` y
`correos_activos_en_otras_unidades`. Ahora también reportan la cédula si la
fila abierta en la nube es de la misma unidad pero de **otro equipo**:

```sql
and (i.sitio_id <> v_equipo.sitio_id or i.dispositivo_entrada_id <> v_equipo.id)
```

- **Por qué no da falsos avisos.** El cliente sólo pregunta por cédulas que
  tiene abiertas localmente. Como el índice único no admite dos filas
  abiertas con la misma cédula, si la que está en la nube es de otro
  equipo, la de este equipo no está en la nube (fue rechazada o todavía no
  se envió). Es un duplicado real. El equipo que registró la fila que sí
  está en la nube nunca se ve a sí mismo.
- **Sin cambiar las apps.** El nombre de la unidad llega como
  `"<unidad> (otro equipo de esta unidad)"`. El aviso que ya muestran
  escritorio y teléfono ("… tiene un ingreso activo acá Y en <unidad> — hay
  que resolverlo.") se lee bien tal cual. Las firmas no cambian, así que
  las apps ya instaladas se benefician sin actualizarse. El nombre
  `en_otras_unidades` se conserva por compatibilidad.
- Se actualizó el comentario de `marcar_fallida_por_ingreso_activo`.

### Cómo se comprobó

Los tests SQL `ingresos_por_correo.sql`, `ingreso_unico_entre_unidades.sql` e
`ingreso_unico_proveedores_y_kof.sql` tenían un paso "3b" que preguntaba
desde el **otro** equipo de la unidad A por una cédula registrada por el
primero, y esperaba 0. Con el arreglo ese caso pasa a avisarse a propósito,
así que el paso quedó en dos:

- **3b:** el equipo que registró no se ve a sí mismo (0 filas).
- **3c:** el otro equipo de la unidad ve 1 fila, con
  `"<unidad> (otro equipo de esta unidad)"`.

---

## 6. Push: el borrado de tokens muertos podía llevarse uno nuevo

### Qué pasaba

`admin-enviar-push` manda en lotes de 10 contra FCM y, al final, borra los
tokens que FCM dio por muertos. Los borraba **por `dispositivo_id`**. Si
mientras se enviaba el teléfono registraba un token nuevo
(`registrar_token_push` pisa la fila del equipo), el borrado se llevaba el
token nuevo y válido. El equipo dejaba de recibir avisos hasta su próximo
inicio de sesión.

### Arreglo

Se borra **por `token`**: sólo desaparece la fila si todavía tiene el token
muerto. El borrado va en lotes de 50 (`TOKENS_POR_BORRADO`), porque el
filtro viaja en la URL y cada token ronda los 160 caracteres. La respuesta
(`tokens_eliminados`) mantiene su forma, así que el panel no cambia.

### Cómo se comprobó

Revisión del código. No hay Deno en el entorno de esta revisión, así que no
se corrió un chequeo de tipos de la Edge Function: hay que verificarla al
desplegar.

---

## Arreglo adicional: `panel_buscar_movimientos.sql`

Al correr la suite SQL completa apareció que
`supabase/tests/panel_buscar_movimientos.sql` ya no se podía ejecutar desde
el 2026-10-03: su fixture insertaba dos ingresos abiertos con la misma
cédula, cosa que `ingresos_contratista_activo_idx` ahora rechaza. Se le dio
salida al ingreso "de hace 40 días". El test no dependía de que estuviera
abierto: la búsqueda filtra por fecha de entrada.

---

## 4. Visitas: "¿ya está adentro?" ciego por RLS (ronda 2)

### Qué pasaba

Es el mismo bug que el 2026-10-03 se corrigió para contratistas y
proveedores. `visitante_activo_en_otro_sitio` (aviso al hacer el check-in) y
`visitantes_con_conflicto_activo` (aviso posterior a sincronizar) consultaban
`/rest/v1/movimientos_visita?...&sitio_id=neq.<unidad propia>`. La política
"leer movimientos_visita del propio sitio o admin" sólo deja ver la propia
unidad, así que **siempre respondían vacío**: ningún aviso funcionó nunca. Y
en la nube no había garantía: un visitante podía quedar con dos visitas
abiertas.

### Arreglo

- **Migración `20261004130000_visitas_ven_otras_unidades`:**
  - índice único `movimientos_visita_cedula_activa_idx`. Antes de crearlo
    se comprueba que no haya duplicados abiertos y, si los hay, aborta
    diciendo qué cédula cerrar;
  - `visita_activa_de_visitante(p_cedula)` y
    `visitantes_activos_en_otras_unidades(p_cedulas)`, `security definer`.
    La segunda también reporta lo abierto por el otro equipo de la misma
    unidad.
- **Núcleo:** las dos consultas usan esas funciones, con la misma firma. La
  cola reconoce el 409 del índice nuevo y deja la apertura fallida de
  inmediato.
- **Escritorio:** `registrar_entrada_visita` no registra si la nube dice que
  el visitante ya está adentro. Es mejor esfuerzo como el aviso: sin red se
  registra y el aviso posterior avisa si chocó.

### Cómo se comprobó

- **Rust:** 4 tests nuevos o reescritos (los viejos verificaban la URL de la
  consulta ciega).
- **SQL:** `supabase/tests/visitas_entre_unidades.sql`. También se probó que
  la guarda de duplicados aborta la migración con su mensaje.

---

## 7. Gafete de visita sin índice en la nube dentro de las visitas (ronda 2)

### Qué pasaba

`movimientos_visita` nunca tuvo índice de "gafete en uso" en la nube: la
unicidad era sólo local. Todos los demás movimientos con gafete sí lo
tienen. Si los dos equipos entregaban el mismo gafete de visita sin
conexión, la nube aceptaba las dos visitas.

### Arreglo

- **Migración `20261004140000_gafete_de_visita_unico_en_visitas`:**
  - índice `movimientos_visita_gafete_activo_sitio_idx`, con guarda de
    duplicados;
  - el trigger de gafete compartido con correo nombra este índice al
    rechazar una visita.
- **Núcleo:** la cola reconoce ese 409 como choque de gafete, con el tipo
  nuevo `TipoMovimientoGafete::Visita` ("El check-in de visita de … con
  gafete de visita … no quedó registrado en la nube").
- **Escritorio y teléfono:** el tipo nuevo llega con su texto. Los bindings
  Kotlin se regeneraron con uniffi-bindgen, como hace la CI; el único cambio
  es la variante `VISITA`.

### Cómo se comprobó

- Núcleo: `visita_con_gafete_ya_activo_queda_fallida_con_aviso`.
- Escritorio: vitest y `tsc`.
- SQL: casos G4b y G4c en `gafete_de_visita_entre_tablas.sql`.
- Kotlin: no se pudo correr Gradle aquí. `ConflictoGafeteTest` incluye el
  caso nuevo y lo corre la CI.

---

## 5. Una persona, adentro por dos vías a la vez (ronda 2)

### Qué pasaba

Contratistas, proveedores e ingresos por correo tienen cada uno su regla e
índice de "cédula activa", pero ninguno mira a los otros dos. La misma
persona podía estar adentro como contratista y a la vez como proveedor o
por correo, en la misma unidad o en otra. Físicamente es una sola persona.

Decisión tomada: se bloquea. Las visitas (módulo de citas) quedan fuera a
propósito: son otro dominio, con su propia regla (punto 4).

### Arreglo, en las tres capas habituales

1. **Local** (`queries::persona_adentro`): mira las otras dos vías en este
   equipo y en la caché del otro equipo, con la cédula normalizada. Cada
   servicio responde `AdentroPorOtraVia(via)`: "Esta persona ya está adentro
   como proveedor — registre primero esa salida". La vista previa del
   contratista ya lo avisa antes de confirmar.
2. **En vivo:** función `persona_adentro_por_otra_via`, consultada en
   paralelo en los tres flujos verificados.
   - **Contratista:** bloquea con "otro sitio" y la vía al lado del nombre
     ("Cartago (como proveedor)"). No hace falta una variante nueva de
     `BloqueoIngreso`, que viaja a las dos apps.
   - **Proveedor y correo:** si la persona está adentro por otra vía,
     responden `AdentroPorOtraVia`; si la consulta falla, no se registra.
3. **Nube** (migración `20261004150000_persona_adentro_por_una_sola_via`):
   - trigger en las tres tablas con un candado por cédula normalizada;
     rechaza con 23505 nombrando el índice de la propia tabla, que la cola
     ya reconoce;
   - los avisos posteriores a sincronizar reportan a quien está adentro por
     otra vía ("<unidad> (como contratista)"). Mismas firmas, así que las
     apps instaladas también lo muestran.

### Cómo se comprobó

- **Rust:** tests de la consulta local, uno por servicio, el bloqueo en la
  nube por otra vía y el cliente de la función.
- **SQL:** `persona_adentro_por_una_sola_via.sql`.
- **Carrera real:** dos sesiones registran a la misma persona como
  contratista y como proveedor a la vez. La segunda espera el candado y es
  rechazada; queda 1 ingreso abierto.

---

## 8. Caché de remotos: un cierre manual podía "revivir" (ronda 2)

### Qué pasaba

La recepción de abiertos lee la nube y después reemplaza entera la caché
`*_remotos`. Si en medio quien operaba cerraba a mano un registro del otro
equipo, el reemplazo lo volvía a insertar y la persona reaparecía "adentro"
hasta la siguiente sincronización. Mientras tanto, la regla "ya está
adentro en el otro dispositivo" frenaba un ingreso legítimo. Un aviso en
vivo atrasado podía hacer lo mismo.

### Arreglo

- **Migración local 56:** tabla `remotos_cerrados_aca` para las lápidas.
- **`cierres_remotos`:** el cierre borra la fila y anota la lápida en una
  sola transacción, y la recepción no inserta un uuid con lápida. Esto vale
  para las cuatro cachés: contratistas, proveedores, correo y KOF.
- **Cuándo se olvida una lápida:** cuando la nube ya no devuelve el registro
  abierto, o al día.

### Cómo se comprobó

- `un_ingreso_remoto_cerrado_aca_no_revive_con_una_lectura_vieja` falla sin
  el arreglo.
- `una_lapida_de_mas_de_un_dia_se_olvida`.
- Los tests de migración que rebobinan el esquema sueltan también la tabla
  nueva.

---

## 9. `registrar_token_push` con el mismo token a la vez (ronda 2)

### Qué pasaba

Si dos equipos registraban el mismo token al mismo tiempo, ninguno de los
dos `delete` veía la fila sin confirmar del otro, y el segundo `insert`
chocaba con `unique (token)`: el segundo registro fallaba con 409.
Reproducido con dos sesiones de Postgres.

### Arreglo

Migración `20261004160000_registrar_token_push_sin_choques`: un candado por
token antes del `delete`. Gana el último en llegar, que es lo que la función
ya pretendía.

### Cómo se comprobó

El mismo escenario de dos sesiones termina sin error y con el token en el
segundo equipo.

---

## Cómo se verificó todo junto

- **Núcleo Rust:** `cargo fmt`; `cargo clippy --all-targets -D warnings`
  con y sin `--features nube`; `cargo test --features nube` → **904 tests, 0
  fallos**; `cargo test` sin features → **687, 0 fallos** (al cierre de la
  ronda 2).
- **`mobile/rust-core`:** clippy y **130 tests**, 0 fallos. Bindings Kotlin
  regenerados y al día.
- **Escritorio:** vitest y `tsc` de `desktop/` pasan. El crate de Tauri
  (`desktop/src-tauri`) no se pudo compilar aquí porque faltan las
  librerías GTK del sistema que pide `gdk-sys`. Sus cambios pasan
  `rustfmt --check` y usan funciones ya exportadas por el núcleo. **Lo valida
  la CI.**
- **Android (Gradle):** no se pudo correr aquí. **Lo valida la CI.**
- **SQL:** se levantó un Postgres 16 local con las piezas propias de
  Supabase reemplazadas por equivalentes mínimos (`auth.jwt()`,
  `realtime.send`, `vault`, `net.http_post`, roles) y se aplicaron **todas
  las migraciones del repo en orden, incluidas las cinco nuevas**. Pasan
  todos los tests de `supabase/tests/` menos `realtime_autorizacion.sql`,
  que depende de la tabla real `realtime.messages` de Supabase (en el
  reemplazo, `id` es de otro tipo). Por eso falla igual con o sin estos
  cambios.
  - **Carreras reales probadas con dos sesiones:** gafete visita/correo,
    persona contratista/proveedor y token push.
  - Nota para quien repita esto: aplicar las migraciones desde cero exige
    crear antes el esquema `private` y la política
    `"dispositivos reciben broadcast de su sitio"` sobre
    `realtime.messages`, porque hay migraciones que los usan antes de que
    otra los documente (deriva histórica del proyecto real).

## Despliegue

1. **Migraciones de Supabase**, en este orden, primero en staging:
   - `20261004120000_conflictos_misma_unidad_y_gafete_de_visita.sql`
   - `20261004130000_visitas_ven_otras_unidades.sql`
   - `20261004140000_gafete_de_visita_unico_en_visitas.sql`
   - `20261004150000_persona_adentro_por_una_sola_via.sql`
   - `20261004160000_registrar_token_push_sin_choques.sql`

   La 130000 y la 140000 crean índices únicos y abortan con un mensaje claro
   si encuentran visitas abiertas duplicadas: en ese caso hay que cerrar las
   sobrantes y volver a aplicar. Después, correr `supabase/tests/`.
2. **Edge Function:** desplegar `admin-enviar-push`.
3. **Apps:** los arreglos del núcleo viajan con la próxima versión de
   escritorio y teléfono (la base local migra sola a la versión 56). Los
   arreglos de la nube rigen apenas se aplican las migraciones, también para
   las apps ya instaladas.

## Pendiente

- **Historial del panel web sin ingresos por correo ni de proveedores.** No
  es un bug: la vista `panel_movimientos` y su pantalla están diseñadas
  para contratistas (tipo de ingreso, medio, PRAIND). Sumar las otras vías
  es una función nueva, con columnas y filtros propios. Por ahora "Adentro
  ahora" sí las muestra.
