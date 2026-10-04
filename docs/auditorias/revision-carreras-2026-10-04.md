# Revisión de carreras y huecos — cambios del 2026-10-03

Fecha: 2026-10-04. Rama revisada: `feat/ingreso-por-correo` (todo lo que entró
el 2026-10-03: ingreso "por correo", "Sin gafete", ingreso único entre
unidades para contratistas, proveedores y KOF, sesión única sin empates,
notificaciones push y los cambios del panel). Los arreglos viven en la rama
`fix/carreras-ingresos-y-gafetes`, que sale de esa misma rama.

Este documento explica, para cada hallazgo, **qué pasaba**, **por qué
pasaba**, **cómo se arregló** y **cómo se comprobó**. Al final quedan los
hallazgos que no se arreglaron en esta rama (también anotados en
`docs/pendientes.md`).

## Resumen

| # | Hallazgo | Gravedad | Estado |
|---|----------|----------|--------|
| 1 | La cola podía mandar el "cerrar" antes que el "crear": la persona quedaba adentro para siempre en la nube | Alta | Arreglado |
| 2 | El gafete de visita se controlaba en un solo sentido (correo → visita, no visita → correo) y sin barrera en la nube | Media | Arreglado |
| 3 | Un ingreso rechazado por chocar con el otro equipo de la MISMA unidad no avisaba a nadie | Media | Arreglado |
| 6 | El envío de push podía borrar el token recién renovado de un teléfono | Baja | Arreglado |
| — | `supabase/tests/panel_buscar_movimientos.sql` dejó de correr con el índice único del 2026-10-03 | Baja (test) | Arreglado |
| 4 | Visitas: el control "¿ya está adentro en otra unidad?" sigue ciego por RLS | Media | Pendiente |
| 5 | Una misma cédula puede estar adentro a la vez como contratista, proveedor y por correo | Decisión | Pendiente |

Los números siguen la numeración del informe de la revisión; el 4 y el 5
quedaron para después a propósito (ver el final).

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

## Cómo se verificó todo junto

- **Núcleo Rust:** `cargo fmt`; `cargo clippy --all-targets -D warnings`
  con y sin `--features nube`; `cargo test --features nube` → **891 tests, 0
  fallos**; `cargo test` sin features → **681, 0 fallos**.
- **Escritorio (`desktop/src-tauri`):** no se pudo compilar en este entorno,
  porque faltan las librerías GTK del sistema que pide `gdk-sys`. El cambio
  usa una función ya exportada (`nube::gafete_de_correo_ocupado_en_otro_dispositivo`)
  y pasa `rustfmt --check`. **Lo valida la CI.**
- **SQL:** se levantó un Postgres 16 local con las piezas propias de
  Supabase reemplazadas por equivalentes mínimos (`auth.jwt()`,
  `realtime.send`, `vault`, `net.http_post`, roles) y se aplicaron **todas
  las migraciones del repo en orden, incluida la nueva**. Pasan todos los
  tests de `supabase/tests/` menos `realtime_autorizacion.sql`, que depende
  de la tabla real `realtime.messages` de Supabase (en el reemplazo, `id` es
  de otro tipo). Por eso falla igual con o sin estos cambios.
  - Nota para quien repita esto: aplicar las migraciones desde cero exige
    crear antes el esquema `private` y la política
    `"dispositivos reciben broadcast de su sitio"` sobre
    `realtime.messages`, porque hay migraciones que los usan antes de que
    otra los documente (deriva histórica del proyecto real).

## Despliegue

1. Aplicar `supabase/migrations/20261004120000_conflictos_misma_unidad_y_gafete_de_visita.sql`
   en staging y correr `supabase/tests/gafete_de_visita_entre_tablas.sql` y
   los tres tests de ingreso único/correo.
2. Desplegar la Edge Function `admin-enviar-push`.
3. El arreglo de la cola (1) y el del gafete local (2) viajan con la próxima
   versión de escritorio y teléfono. Los arreglos de la nube (2 y 3) rigen
   apenas se aplica la migración, también para las apps ya instaladas.

## Pendiente (no se arregló en esta rama)

- **4. Visitas siguen ciegas entre unidades.** `visitante_activo_en_otro_sitio`
  y `visitantes_con_conflicto_activo` consultan `/rest/v1/movimientos_visita`
  directo, y la RLS sólo deja ver la propia unidad. Siempre responden
  "libre": es el mismo bug que el 2026-10-03 se corrigió para contratistas
  y proveedores. Arreglo: funciones `security definer` (como
  `ingreso_correo_activo`) y un índice único de cédula activa en
  `movimientos_visita`, verificando antes que no haya duplicados abiertos.
- **5. Una misma cédula, adentro por dos vías.** Cada tabla (contratistas,
  proveedores, correo) tiene su propio índice de cédula activa: nada impide
  que alguien esté adentro como contratista y a la vez por correo. Puede
  ser intencional; hay que decidirlo.
- **Caché de remotos, carrera menor.** `cerrar_ingreso_*_remoto` borra la fila
  de la caché, pero una sincronización que leyó la nube un instante antes
  puede volver a insertarla. La persona reaparece como "adentro" hasta la
  siguiente sincronización.
- **Historial del panel web.** No incluye ingresos por correo ni de
  proveedores (sólo "Adentro ahora" los muestra).
- **`registrar_token_push`.** Si dos equipos registran el mismo token a la vez,
  el `delete` y el `insert` pueden chocar con el `unique(token)`. Es muy
  improbable y el teléfono reintenta en su próximo inicio de sesión.
