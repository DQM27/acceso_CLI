# Despliegue a producción: lo que está en staging y todavía no en producción

Registro de todo cambio que se aplicó en **staging**
(`control-acceso-staging`, `pmrytjktlyiuikxuuxpr`) y que falta reproducir en
**producción** (`control-acceso-nube`, `xidaepyaljzkpbsxrqsm`). Regla del
proyecto: nada se aplica en producción sin autorización explícita del dueño.

Mantener este archivo al día: cada cambio que se aplique en staging se anota
aquí en el mismo commit. Cuando se aplique en producción, se mueve a la
sección "Aplicado en producción" con fecha y quién lo autorizó.

Última revisión: 2026-10-05.

## 0. Lista de verificación para los PR a main (auditada otra vez el 2026-10-06)

Cruce hecho por nombre entre el repo (117 migraciones de nube en
`feat/visitas-movil`, sin versiones repetidas), staging (55 en su historial) y
producción (104 en su historial, sólo lectura de la lista). Todo lo que está en
`main` ya está en producción, y producción no tiene nada que el repo no tenga.

### 0.1 Qué lleva cada PR

Las ramas forman una sola cadena, cada una contiene a la anterior:

`main` → `feat/ingreso-por-correo` → `fix/carreras-ingresos-y-gafetes` →
`feat/reglas-compartidas` → `feat/web-visitas` → `feat/escritorio-pendientes`
→ `feat/visitas-movil`

Se mergean **en ese orden, un PR por rama contra `main`**, con *merge commit*
(no squash ni rebase: reescribir los commits rompería las ramas siguientes).
Cada PR lleva sólo lo nuevo de su rama; la columna "Rama" de 0.2 dice qué
migraciones trae cada uno.

**Mergear a `main` aplica las migraciones en producción:** `main` está
conectado a la integración de GitHub de Supabase con "Deploy to production"
(desde el 2026-09-12, ver `docs/decisiones-tecnicas.md`). Cada PR aplica sus
migraciones al entrar, en orden de versión. Las apps no se publican (salen con
tags `v*`). Dos merges piden publicar algo enseguida:
- **PR 3** (`feat/reglas-compartidas`, migración 7): borra la función con la
  que el panel publicado crea contratistas. Antes del merge, desplegar
  `admin-crear-contratista` y `admin-editar-contratista` en producción;
  después, publicar el panel nuevo.
- **PR 4** (`feat/web-visitas`, migración 9): quita la escritura directa del
  anfitrión en citas. Publicar la web de visitas nueva enseguida.

Chequeado en producción el 2026-10-06 (sólo lectura), para que ningún merge se
corte a mitad:
- 3 y 4: 0 visitas abiertas duplicadas por cédula o por gafete (0.5);
- 7: `panel_crear_contratista` existe con la firma exacta que borra;
- 8: es `not valid`, no revisa filas existentes;
- 9: los `drop policy` usan `if exists`, y `crear_cita_anfitrion` tiene los
  mismos argumentos y devuelve `uuid`, así que se puede reemplazar;
- 5: sólo pone triggers para registros nuevos.

Fuera de la cadena:
- `prueba/radix-visitas`: ya está entera dentro de la cadena; no lleva PR.
- `feat/analisis-syncfusion`: otra línea, con su propia migración
  (`panel_resumen_movimientos`, ya aplicada en staging). PR aparte.
- `claude/rediseno-web-visitas`: reemplazada por la web de visitas nueva; no
  se mergea (ver 0.6).
- PR de Dependabot (#92, #94, #99, #103, #104, #106): después de la cadena.

### 0.2 Migraciones de nube pendientes en producción (13, en este orden)

Producción tiene las 104 primeras del repo; le faltan exactamente estas. En
staging se comparan por **nombre** (staging anota la hora de aplicación, no la
versión del archivo).

| # | Archivo del repo | Rama (PR) | En staging | Cuándo va en producción |
|---|---|---|---|---|
| 1 | `20261003210000_ingresos_por_correo.sql` | `feat/ingreso-por-correo` | En el historial | Antes de las apps nuevas |
| 2 | `20261004120000_conflictos_misma_unidad_y_gafete_de_visita.sql` | `fix/carreras-ingresos-y-gafetes` | En el historial | Antes de las apps nuevas (ver 2.5b) |
| 3 | `20261004130000_visitas_ven_otras_unidades.sql` | `fix/carreras-ingresos-y-gafetes` | En el historial | Igual que la 2. Antes, revisar 0.5 |
| 4 | `20261004140000_gafete_de_visita_unico_en_visitas.sql` | `fix/carreras-ingresos-y-gafetes` | En el historial | Igual que la 2. Antes, revisar 0.5 |
| 5 | `20261004150000_persona_adentro_por_una_sola_via.sql` | `fix/carreras-ingresos-y-gafetes` | En el historial | Igual que la 2 |
| 6 | `20261004160000_registrar_token_push_sin_choques.sql` | `fix/carreras-ingresos-y-gafetes` | **A mano**, no figura en el historial. Verificado: la función tiene el candado `token_push:` | Cuando sea; no rompe nada |
| 7 | `20261004170000_alta_de_contratistas_por_edge_function.sql` | `feat/reglas-compartidas` | **A mano**, no figura. Verificado: `panel_crear_contratista` ya no existe | **Después** de desplegar la Edge Function `admin-crear-contratista` y publicar el panel nuevo: borra la función SQL que usa el panel viejo (ver 2.6) |
| 8 | `20261005120000_usuarios_nombre_obligatorio.sql` | `feat/reglas-compartidas` | En el historial | Cuando sea (ver 2.7) |
| 9 | `20261005130000_visitas_web_anfitrion.sql` | `feat/web-visitas` | **En dos partes:** las funciones en el historial como `visitas_web_anfitrion_funciones`; los 7 `drop policy`, a mano. Verificado: 4 funciones nuevas y 0 políticas de escritura directa del anfitrión | Las funciones, cuando sea; los `drop policy`, **después** de publicar la web de visitas nueva (ver 2.8). Si se aplica el archivo entero, tiene que ser junto con la web |
| 10 | `20261005133000_guardar_cita_reglas_del_nucleo.sql` | `feat/web-visitas` | En el historial | Después de la 9 (reemplaza `guardar_cita`, que crea la 9). No rompe nada: la web nueva ya valida igual. Antes, la consulta de sólo lectura de 2.8 |
| 11 | `20261005140000_movimientos_visita_placa.sql` | `feat/escritorio-pendientes` | En el historial | **Antes** de las apps nuevas de escritorio y teléfono: las dos suben la placa (ver 2.9 y 2.10) |
| 12 | `20261006100000_nombres_de_visitantes_en_mayuscula.sql` | `feat/visitas-movil` | En el historial | Después de la 10 (reemplaza `guardar_cita`). No rompe nada (ver 2.11) |
| 13 | `20261006101000_nombre_de_usuarios_en_mayuscula.sql` | `feat/visitas-movil` | En el historial | Cuando sea (ver 2.11) |

Las tres "a mano" (6, 7 y la segunda parte de la 9) existen en staging aunque
su historial no las nombre. Para producción conviene aplicarlas con la
herramienta de migraciones, así quedan anotadas; si la herramienta se cuelga
con los `drop` (le pasó a staging), el editor SQL sirve igual, y se verifican
con las mismas consultas.

### 0.3 Edge Functions

Comparadas el 2026-10-06 contra lo desplegado en staging y producción.

| Función | Cambio en la cadena | Staging | Producción |
|---|---|---|---|
| `admin-crear-contratista` | Nueva (`feat/reglas-compartidas`) | Versión 3 | **Desplegar** antes de publicar el panel y antes de la migración 7 |
| `admin-editar-contratista` | Nueva (`feat/reglas-compartidas`) | Versión 1 | **Desplegar** antes de publicar el panel |
| `admin-enviar-push` | Borra tokens muertos por token, no por equipo (`fix/carreras-ingresos-y-gafetes`) | **No desplegado**: sigue la versión del 03/10 | **Desplegar** (también en staging) |
| `admin-create-usuario` | Nombre en mayúscula con la regla del núcleo (`feat/visitas-movil`) | **No desplegado** (el trigger de la migración 13 cubre el alta mientras tanto) | Desplegar; sin apuro si la 13 ya está aplicada |
| Las otras 8 (`device-auth`, `device-vincular`, `admin-create-site`, `admin-delete-device`, `admin-list-devices`, `admin-provision-device`, `admin-reset-password-usuario`, `admin-revoke-device`) | Sólo la línea de tipos (`functions-js@2`), sin cambio de comportamiento | — | Volver a desplegar es opcional |
| `sync-access-policy` | Sin cambios | — | — |

Staging además tiene `admin-suspend-device` y `admin-crear-codigo-vinculacion`,
que no están en el repo ni en producción: no van.

Para desplegar las que usan las reglas (`admin-crear-contratista`,
`admin-editar-contratista`, `admin-create-usuario`) hay que subir también
`_shared/reglas/` (el WebAssembly comprimido): conviene la CLI,
`supabase functions deploy <nombre>`.

### 0.4 Base local de los equipos (SQLite)

Main está en el esquema **54**; la rama llega al **57** (y `feat/visitas-movil`
al **58**):
- **55:** ingreso por correo;
- **56:** lápidas de cierres de remotos;
- **57:** placa de las visitas;
- **58:** visitas abiertas por el otro equipo de la unidad (sólo
  `feat/visitas-movil`).

Cada equipo migra solo al abrir la app nueva y **no hay vuelta atrás** (ver la
sección 3). Probar primero en un solo equipo.

### 0.5 Antes de aplicar las migraciones 2 a 5 en producción

Las migraciones 3 y 4 crean índices únicos sobre las visitas abiertas. Si
producción tuviera una persona o un gafete abiertos dos veces, ellas mismas se
detienen con un mensaje claro ("cerrar las sobrantes antes de aplicar"), sin
dejar nada a medias. Para no llevarse la sorpresa en pleno despliegue, con
autorización correr antes en el editor SQL de producción (sólo lectura; deben
dar 0 filas):

```sql
select visitante_cedula, count(*)
from public.movimientos_visita where hora_salida is null
group by 1 having count(*) > 1;

select sitio_id, gafete_numero, count(*)
from public.movimientos_visita
where hora_salida is null and gafete_numero is not null
group by 1, 2 having count(*) > 1;
```

### 0.6 Lo que hay en staging y NO va con este PR

- **Rama vieja `claude/rediseno-web-visitas`:** `rediseno_visitas_tablas_nuevas`,
  `optimiza_rls_e_indices_de_visitas` y `rediseno_visitas_rpcs_vista_busqueda`.
  La web de visitas nueva no las usa (ver 2.8).
- **`feat/analisis-syncfusion`:** `panel_resumen_movimientos` (en esa rama,
  `20260930130100_panel_resumen_movimientos.sql`). Es otra línea de trabajo, con
  su propio PR. Su versión no choca con las de main.
- **`rutas_documento_tramo_viaje`** (y sus dos arreglos): no existen en el repo
  (ver 2.4).
- **`crea_personas_vetadas` / `revierte_personas_vetadas`:** se aplicaron y se
  revirtieron en staging; no hay nada que llevar (ver 2.2).
- **`lote_00` a `lote_04` y `fix_orden_esquema_private_antes_de_hora`:** con esto
  se armó staging desde cero; equivalen a migraciones que producción ya tiene.

### 0.7 Orden completo sugerido para producción

Los números son los de la tabla de 0.2.

1. Respaldo (sección 1).
2. Migración 1 (`ingresos_por_correo`).
3. Revisar 0.5 y aplicar las migraciones 2 a 5.
4. Migraciones 6 y 8, la parte de funciones de la 9, y la 10.
5. Migraciones 11 (`movimientos_visita_placa`), 12 y 13.
6. Desplegar las Edge Functions de 0.3: `admin-crear-contratista`,
   `admin-editar-contratista`, `admin-enviar-push` y `admin-create-usuario`.
7. Publicar el panel web, la web de visitas y las apps (escritorio y teléfono;
   primero un equipo de prueba).
8. Migración 7 y los `drop policy` de la 9.
9. Anotar todo en la sección 5.

## 1. Antes de empezar

1. **Respaldo.** Descargar un respaldo de la base de producción (Supabase →
   Database → Backups) y anotar la hora.
2. **Ver qué tiene ya producción.** En el editor SQL de producción (sólo
   lectura):

   ```sql
   select version, name
   from supabase_migrations.schema_migrations
   order by version desc
   limit 30;
   ```

   Comparar contra la tabla de la sección 2. Staging registra las
   migraciones con la hora en que se aplicaron, que no coincide con la
   versión del nombre del archivo del repo. Se comparan por **nombre**.
3. **Orden.** Aplicar en el orden de la tabla. Cada fila indica de qué
   depende.

## 2. Cambios de Supabase en staging, pendientes en producción

| # | Migración (nombre en staging) | Archivo en el repo | Rama dueña | ¿Rompe apps viejas? |
|---|---|---|---|---|
| ~~1~~ | ~~`cambio_nube_lleva_la_fila_de_ingresos`~~ **Aplicada en producción el 2026-09-29 (ver sección 5)** | `supabase/migrations/20260929075806_cambio_nube_lleva_la_fila_de_ingresos.sql` | `claude/nucleo-n1-n3` (N1, estable) | No |
| 2 | `rediseno_visitas_tablas_nuevas` | `supabase/migrations/20260927120000_rediseno_visitas_tablas_nuevas.sql` | `claude/rediseno-web-visitas` | Revisar con esa rama |
| 3 | `optimiza_rls_e_indices_de_visitas` | `supabase/migrations/20260927121500_optimiza_rls_e_indices_de_visitas.sql` | `claude/rediseno-web-visitas` | Revisar con esa rama |
| 4 | `rediseno_visitas_rpcs_vista_busqueda` | `supabase/migrations/20260927150000_rediseno_visitas_rpcs_vista_busqueda.sql` | `claude/rediseno-web-visitas` | Revisar con esa rama |
| 5 | `rutas_documento_tramo_viaje` (+ `_fix_search_path`, `_indices_fk`) | **No existe en el repo** | `feat/rutas-documento-tramo-viaje` (19 sep, sin unir) | Desconocido |
| ~~6~~ | ~~`hora_servidor_ms`~~ **Aplicada en producción el 2026-09-29 (ver sección 5)** | `supabase/migrations/20260929075809_hora_servidor_ms.sql` | `claude/fusion-integral` (reloj en ms) | No |

### 2.1 Aviso en vivo con los datos (`cambio_nube_lleva_la_fila_de_ingresos`)

- **Qué hace:** redefine `private.emitir_cambio_nube_sitio()` para que el
  aviso `cambio_nube` lleve `id` y, en `ingresos`, la fila completa
  (`registro`). Los equipos con N1 guardan la fila al instante, sin ir a
  la nube.
- **Compatibilidad:** las apps que no conocen esos campos los ignoran y
  siguen sincronizando por tabla. Se puede aplicar antes o después de
  actualizar las apps. Sin esta migración, N1 funciona igual pero sin la
  aplicación instantánea.
- **Verificar después de aplicar:**

  ```sql
  select pg_get_functiondef('private.emitir_cambio_nube_sitio'::regproc)
         like '%''registro''%' as lleva_la_fila;
  ```

  Y en la práctica: registrar un ingreso en un equipo y ver que el otro lo
  muestra en Activos en uno o dos segundos.
- **Deshacer:** volver a crear la función con el cuerpo de
  `20260906044549_avisa_cambio_nube_segun_quien_escribe_no_quien_creo_la_fila.sql`.

### 2.1b Hora del servidor en milisegundos (`hora_servidor_ms`)

- **Qué hace:** crea `public.hora_servidor_ms()`, que devuelve la hora del
  servidor en ms (`clock_timestamp()`). El núcleo (escritorio y móvil) la
  consulta varias veces al autenticarse y mide el desfase del reloj del
  equipo con precisión de milisegundos (`src/nube/reloj_preciso.rs`). Sin
  datos de nadie; ejecutable por `anon` y `authenticated`.
- **Compatibilidad:** sin la función, las apps siguen midiendo con el
  header HTTP `Date` (precisión de 1 s), como antes. Se puede aplicar antes
  o después de actualizarlas.
- **Verificar después de aplicar:**

  ```sql
  select public.hora_servidor_ms() - (extract(epoch from now()) * 1000)::bigint as diferencia_ms;
  ```

  (debe dar un número chico y positivo). Aplicada en staging el
  2026-09-29.
- **Deshacer:** `drop function public.hora_servidor_ms();`

### 2.2 Nota: veto por persona, aplicado y revertido en staging

El historial de staging muestra `crea_personas_vetadas` seguida de
`revierte_personas_vetadas`: se probó un veto con tabla propia y se
descartó a favor del mismo interruptor de contratistas (ver
`docs/features-futuras/plan-veto-por-persona.md`). El efecto neto es cero:
**no hay nada que aplicar en producción** por esto. El bloqueo de
proveedores y visitas vive sólo en el núcleo de las apps.

### 2.3 Rediseño de la web de visitas (filas 2 a 4)

Vienen de otra línea de trabajo (`claude/rediseno-web-visitas`, ver
`docs/handoff-rediseno-web-visitas.md`). Se despliegan junto con esa rama,
en el orden de sus archivos, siguiendo sus propias notas. No se aplican
sueltas.

### 2.4 Tramos de viaje de rutas (fila 5): riesgo

Estas tres migraciones están aplicadas en staging, pero **ningún archivo
del repo las contiene**: se aplicaron directo, desde la rama
`feat/rutas-documento-tramo-viaje` (19 sep, sin unir). Producción no se
puede reconstruir a partir del repo mientras siga así.

Antes de llevarlas a producción:
1. Recuperar su SQL desde staging (`supabase_migrations.schema_migrations`
   guarda el texto de cada una).
2. Versionarlas como archivos en esa rama.
3. Decidir si esa rama sigue viva.

### 2.5 Verificación antes del PR de `claude/fusion-integral` (2026-09-29)

Comparadas las migraciones de staging, producción y la rama, y las tablas y
columnas que usan las apps de la rama:

- **Producción está al día con `main`** (última:
  `agrega_indice_unico_gafete_activo_por_sitio`).
- **La rama sólo necesita dos migraciones nuevas en producción:** filas 1
  (`cambio_nube_lleva_la_fila_de_ingresos`) y 6 (`hora_servidor_ms`).
  Ninguna rompe apps viejas y las apps nuevas funcionan sin ellas (más
  lento: sin la fila en el aviso y con el reloj a ±1 s).
- **Veto:** `crea_personas_vetadas` + `revierte_personas_vetadas` en
  staging; verificado que no queda ninguna tabla ni función de ese diseño.
  Nada que aplicar en producción (ver 2.2).
- **Visitas (filas 2-4) y rutas (fila 5)** son de otras ramas; las apps de
  esta rama no usan sus tablas ni RPC.
- **Staging está desfasado en `salidas_ruta`:** la fila 5 (sin archivo en el
  repo) le quitó `numero_ruta`, `sub_numero`, `numero_documento`,
  `fecha_documento`, `resultado` y `motivo_resultado` y le agregó
  `viaje_id`. Las apps (de `main` y de esta rama) mandan esas columnas, así
  que **en staging las salidas de ruta no pueden subir** (la tabla tiene 0
  filas). En producción sí están. No bloquea el PR, pero las pruebas de
  rutas contra staging van a fallar hasta resolver la rama de rutas.
- `telemetria_diagnostico` sólo existe en staging, como corresponde.

### 2.5b Arreglos de condiciones de carrera (aplicados en staging el 2026-10-05)

Rama `fix/carreras-ingresos-y-gafetes` (incluida en `feat/reglas-compartidas`).
Detalle en `docs/auditorias/revision-carreras-2026-10-04.md`. Antes de aplicar
se verificó que staging no tuviera visitas abiertas duplicadas (0 por cédula,
0 por gafete).

| Migración del repo | Nombre en staging | Estado en staging |
|---|---|---|
| `20261004120000_conflictos_misma_unidad_y_gafete_de_visita.sql` | `conflictos_misma_unidad_y_gafete_de_visita` | Aplicada |
| `20261004130000_visitas_ven_otras_unidades.sql` | `visitas_ven_otras_unidades` | Aplicada |
| `20261004140000_gafete_de_visita_unico_en_visitas.sql` | `gafete_de_visita_unico_en_visitas` | Aplicada |
| `20261004150000_persona_adentro_por_una_sola_via.sql` | `persona_adentro_por_una_sola_via` | Aplicada |
| `20261004160000_registrar_token_push_sin_choques.sql` | — (no figura en el historial) | Aplicada a mano por el dueño en el editor SQL de Supabase. El editor no la anota en `supabase_migrations.schema_migrations`: para saber si está, revisar que la función tenga el candado `token_push:` |

Verificado después de aplicar: existen `persona_adentro_por_otra_via`,
`visita_activa_de_visitante` y `visitantes_activos_en_otras_unidades`, los
índices `movimientos_visita_cedula_activa_idx` y
`movimientos_visita_gafete_activo_sitio_idx`, y los triggers de "una sola vía"
y de gafete de visita compartido.

**Importante para producción:** las apps de esta rama consultan
`persona_adentro_por_otra_via` antes de registrar una entrada, y sin
verificar en la nube no se registra. Por eso **estas migraciones van ANTES de
publicar las apps**. Producción además todavía no tiene `ingresos_por_correo`
(`20261003210000`), que va antes que todas estas.

### 2.7 Edición de contratistas y usuarios desde el panel (staging, 2026-10-05)

Rama `feat/reglas-compartidas`, commit `4b8c3c5`.

| Cambio | Estado en staging |
|---|---|
| Edge Function `admin-editar-contratista` (nueva) | Desplegada, versión 1, fijada a `4b8c3c5`, `verify_jwt` activado |
| Edge Function `admin-crear-contratista` (acepta personal de ruta) | Desplegada, versión 3, fijada a `4b8c3c5` |
| Migración `20261005120000_usuarios_nombre_obligatorio.sql` | Aplicada (nombre `usuarios_nombre_obligatorio`) |

Probado contra staging con un administrador y un usuario temporales,
borrados al terminar junto con el contratista de prueba:
- alta con personal de ruta → 200;
- editar nombre y cédula de alguien que está afuera → 200, lo que ejecuta la
  consulta de "¿está adentro?";
- pasar a POR CORREO → 422;
- id inexistente → 404;
- usuario: nombre y rol → guardado, nombre vacío → 23514, sin sesión de
  administrador → 0 filas.

El rechazo de la cédula de alguien adentro no se probó en vivo, para no
crear una entrada falsa que llegara a los equipos del sandbox; está cubierto
por los tests de Deno y del núcleo.

Orden en producción: después de 2.6, desplegar las dos funciones, publicar
el panel y aplicar la migración (no rompe nada si va antes).

### 2.8 Web de visitas nueva (staging, 2026-10-05)

Rama `feat/web-visitas` (sale de `feat/reglas-compartidas`). Reemplaza a la
línea vieja `claude/rediseno-web-visitas` (filas 2 a 4 y sección 2.3): la web
nueva **no usa** esas tablas ni esas funciones, trabaja sobre el modelo de
citas que ya leen escritorio y teléfono (`citas`, `cita_sitios`,
`cita_visitantes`, `movimientos_visita`). Esas filas no se llevan a producción
con esta rama.

Migración `20261005130000_visitas_web_anfitrion.sql`, en dos partes en staging
porque la herramienta de Supabase se cuelga con los `drop policy`:

| Parte | Estado en staging |
|---|---|
| Funciones (todo menos los `drop policy`): `crear_cita_anfitrion` (reemplazada), `editar_cita_anfitrion`, `cancelar_cita_anfitrion`, `estado_visitantes_de_mis_citas`, `visitantes_anteriores` y las auxiliares de `private` | Aplicada (nombre `visitas_web_anfitrion_funciones`) |
| Los 7 `drop policy` de escritura directa del anfitrión (al principio del archivo) | Aplicada el 2026-10-05 por el dueño en el editor SQL (no figura en el historial). Verificado: no queda ninguna política `anfitrion%` que no sea `SELECT` en esas tres tablas (sólo la restrictiva "solo dispositivos vigentes"); como anfitrión, un `insert` directo → 42501 y un `update` directo → 0 filas, mientras crear y cancelar por las funciones siguen funcionando |

Probado contra staging como anfitrión (dentro de una transacción que se
deshizo; no quedó nada guardado):
- crear → id pedido; el reintento igual devuelve la misma; el mismo id con
  otro contenido → 23505; fecha pasada → rechazada;
- la cédula se guarda en su forma única (`1-0847-0293` → `108470293`);
- personas anteriores: 3 sin filtro, 1 buscando por nombre;
- editar → la vieja queda CANCELADA y la nueva VIGENTE; el reintento devuelve
  la misma nueva; editar una cancelada → rechazado;
- cancelar dos veces → sin error (idempotente);
- otro anfitrión: cancelar → P0002, llegadas → 0 filas.

El rechazo de editar "si alguien ya entró" está cubierto por
`supabase/tests/visitas_anfitrion.sql`; no se probó en vivo porque la cita de
prueba (`dfc2f424…`, cédula 900000077) todavía no tiene entrada en la
portería del sandbox.

**Orden en producción:** la migración y la web nueva salen **juntas**: la web
vieja cancelaba con un `UPDATE` directo, y los `drop policy` lo deshabilitan.
Las funciones solas no rompen nada (la web vieja sigue funcionando); los
`drop policy` van después de publicar la web nueva.

**Reglas del núcleo en la web (2026-10-05):** la web valida la cita con
`reglas/src/cita.rs` por WebAssembly (ver
`docs/arquitectura/reglas-compartidas.md`). Al publicarla, su `_headers` ya
trae `'wasm-unsafe-eval'` en `script-src`; sin eso el navegador bloquea el
paquete y el botón "Agendar" queda deshabilitado con el aviso de recargar.
El WebAssembly no necesita migración.

**La base con las mismas reglas:** migración
`20261005133000_guardar_cita_reglas_del_nucleo.sql`. `private.guardar_cita`
deja de guardar "como vino" un documento que no se puede normalizar: exige
la forma única de `normalizar_cedula`, de 3 a 20 caracteres (antes, hasta 30;
uno de 21 a 30 se agendaba y la portería nunca lo reconocía). Además colapsa
los espacios del nombre (mínimo 2 caracteres) y pasa la placa a mayúsculas.
Sólo afecta lo que se guarde de ahí en adelante.

| Estado en staging | |
|---|---|
| Aplicada el 2026-10-05 (nombre `guardar_cita_reglas_del_nucleo`) | Antes de aplicarla: 3 visitantes en `cita_visitantes`, ninguno fuera de la regla. Verificado en un bloque que se deshizo: 21 caracteres, `AB` y `12#45` → "Documento de visitante inválido…"; `01-0847-0293` / `  Ana   María ` / ` abc123 ` → `108470293` / `Ana María` / `ABC123`. Batería `supabase/tests/visitas_anfitrion.sql` en verde (con la función vieja falla en el caso de 21 caracteres) |

**Antes de aplicarla en producción** (sólo lectura, con autorización):
```sql
select count(*) filter (where public.normalizar_cedula(cedula) is null or length(cedula) < 3) as fuera_de_regla
from public.cita_visitantes;
```
Si da más de 0, esas citas ya no coinciden en la portería; se revisan a mano,
pero no impiden aplicar la migración (no toca filas existentes). Va después de
`20261005130000_visitas_web_anfitrion.sql`, que crea la función.

### 2.9 Escritorio: visitas unificadas y medio de ingreso (staging, 2026-10-05)

Rama `feat/escritorio-pendientes` (sale de `feat/web-visitas`).

| Cambio | Estado en staging |
|---|---|
| Migración `20261005140000_movimientos_visita_placa.sql`: columna `placa` en `movimientos_visita` (NULL = caminando), límite de 20 caracteres y la placa dentro del disparador de inmutabilidad | Aplicada (nombre `movimientos_visita_placa`). Verificado: columna, restricción y disparador. Batería `supabase/tests/movimientos_visita_placa.sql` en verde junto con las otras 25 |
| App de escritorio: Visitas con "Esperadas hoy", "Adentro" e "Historial", "Por correo" dentro del modal "Nueva visita", medio de ingreso en el check-in | Sólo en la rama; se prueba con `npm run tauri dev` contra staging |

**Orden en producción:** la migración **antes** que la app de escritorio
nueva. La app sube `placa` con cada movimiento de visita y la pide al bajar
el historial: sin la columna, la nube rechaza las dos cosas y la cola de
visitas se traba. Las apps viejas no la mandan ni la piden, así que la
migración sola no rompe nada.

### 2.10 Teléfono: verificación de visitas agendadas (2026-10-05)

Rama `feat/visitas-movil` (sale de `feat/escritorio-pendientes`). En
"Externos", la primera opción ahora es "Visita": un campo de cédula (escrita o
escaneada) y el núcleo decide si sigue la entrada (gafete opcional, caminando
o en vehículo), la salida (si ya está adentro, en ese equipo o en la PC) o un
aviso (azul si la cita es para otro día). Debajo, quién está adentro por
visita en toda la unidad, con las mismas tarjetas y el mismo diálogo de salida
que contratistas (ícono de PC si entró por el otro equipo), refrescado en vivo.
Sin historial.

- **Sin migraciones de nube.** Usa columnas que `movimientos_visita` ya tiene.
- **Base local: migración 58** (`movimientos_visita_remotos`, caché de las
  visitas abiertas por el otro equipo, y su lápida en `remotos_cerrados_aca`).
  La sincronización la llena en los dos perfiles (el historial de visitas
  sigue siendo sólo de la PC). Dar salida a una visita del otro equipo la
  cierra en la nube; y una visita propia a la que el otro equipo dio salida se
  cierra también en local (`recibir_cierres_de_movimientos_visita_propios`).
- **En vivo:** el aviso de `movimientos_visita` ya existía; dispara la
  sincronización de esa tabla y la lista se vuelve a leer.
- **Las reglas son las de escritorio:** los chequeos contra la nube de una
  entrada (gafete en uso en el otro equipo, visitante adentro en otra unidad)
  se pasaron del comando de escritorio a
  `application::registrar_entrada_visita_verificada`, y ahora la usan los dos.
- **Orden en producción:** igual que el escritorio nuevo, **después** de la
  migración `20261005140000_movimientos_visita_placa.sql` (fila 11 de 0.2): el
  teléfono también sube la placa con cada movimiento de visita.

### 2.11 Todo nombre de persona o empresa en mayúscula (2026-10-06)

Rama `feat/visitas-movil`. La regla vive en el núcleo
(`reglas/src/nombre.rs`: `nombre_en_mayusculas` al guardar,
`nombre_mientras_se_escribe` para las interfaces) y la usan todos:

- **Núcleo (escritorio y teléfono), al guardar:** contratistas (ya lo hacía),
  empresas, empresas de proveedores, proveedor (persona), "por correo",
  encargados de ruta y el ROOT inicial.
- **Citas (web de visitas):** `validarCita` por WASM pone en mayúscula el
  nombre y la empresa del visitante; `guardar_cita` repite la regla (fila 12
  de 0.2).
- **Usuarios:** el alta (`admin-create-usuario`) usa la regla por WASM; la
  edición desde el panel es un `update` directo, así que un trigger en
  `usuarios` repite la regla para los dos caminos (fila 13).
- **Interfaces, mientras se escribe:** panel y web de visitas con el WASM;
  teléfono con `nombreMientrasSeEscribe` del núcleo (uniffi); escritorio con
  `src/nombres.ts` (sólo llega al núcleo por comandos asíncronos; el núcleo
  vuelve a aplicar la regla al guardar).
- **No toca lo ya guardado.**
- **Pendiente de desplegar:** la Edge Function `admin-create-usuario` con la
  regla quedó en el repo pero **no** se desplegó en staging (sin CLI en la
  sesión: hay que subir `_shared/reglas/`). Mientras tanto el trigger de la
  fila 13 cubre el alta igual.

### 2.12 Web de visitas: correo y contraseña con código de activación (2026-10-06)

Rama `feat/login-correo-web-visitas`. El cliente pidió quitar el ingreso con
Google: los anfitriones usan el correo de su dominio. Sin SMTP ni correos: el
panel da de alta y restablece con un código de activación (mismo modelo que la
contraseña temporal de los usuarios de la portería). Análisis y amenazas:
`docs/auditorias/investigacion-login-correo-web-visitas-2026-10-06.md`.

- **Migración** `20261006130000_activacion_de_anfitriones_desde_el_panel.sql`:
  `anfitriones.auth_user_id` (se enlaza solo con las cuentas existentes),
  restricción de correo en minúsculas, `private.anfitriones_activacion` (hash
  bcrypt, 72 h, 5 intentos), `private.bitacora_anfitriones`, funciones solo
  para `service_role` y `panel_anfitriones()` para el panel. Antes de aplicarla
  en producción, comprobar que ningún `anfitriones.correo` tenga mayúsculas o
  espacios (el 2026-10-06 había 0 de 1).
- **Edge Functions nuevas:** `admin-anfitriones` (`verify_jwt` activado) y
  `anfitrion-activar` (`verify_jwt` **desactivado**, pública a propósito).
  `admin-anfitriones` usa las reglas del núcleo (`_shared/reglas/`): conviene
  la CLI, `supabase functions deploy admin-anfitriones`.
- **Panel:** sección Anfitriones. **Web de visitas:** ingreso con contraseña y
  «Primer ingreso: tengo un código de activación».
- **Aplicado en staging** (2026-10-06): la migración (sin la línea de la
  restricción, que ya existía ahí) y las dos funciones, versión 1, fijadas al
  commit `038bd32` con la técnica de 2.6. Probado por HTTP contra staging:
  sin sesión `admin-anfitriones` → 401; todo código rechazado (inexistente,
  errado, ya usado, forma imposible, tras 5 fallos) → el mismo 400; contraseña
  corta → 422 sin gastar intento; activación correcta → 200 e ingreso con la
  contraseña nueva; la sesión del anfitrión no puede listar el panel (403) ni
  administrar (401); la API pública no puede llamar a las funciones internas.
- **Pendiente en staging (requiere confirmar sentencias destructivas en la
  sesión):** quitar los restos del primer enfoque, que nunca llegó a
  producción (`trigger anfitriones_sincronizar_cuentas`,
  `private.sincronizar_cuentas_anfitriones*()`,
  `public.secreto_cuentas_anfitriones_valido()` y la fila
  `alta_de_anfitriones_por_sql` de `schema_migrations`), y los datos de prueba
  (`prueba-activacion@example.invalid` en `anfitriones`, la bitácora y Auth).
- **Orden en producción:** 1) migración; 2) las dos funciones; 3) publicar el
  panel; 4) dashboard de Auth: apagar «Allow new users to sign up», activar
  «Prevent use of leaked passwords» (plan Pro) y dejar vacíos los «password
  requirements»; 5) publicar la web de visitas; 6) desde el panel, generar el
  código de cada anfitrión existente (figuran como «Sin contraseña»).
- **Administradores nuevos del panel con el registro apagado:** además del
  `insert` en `administradores_panel`, crear su cuenta en Authentication →
  Users → Add user → Create new user (con «Auto Confirm User»); después entra
  con Google y Supabase vincula la identidad por el correo. **Verificar en
  staging antes de apagar el registro en producción.**

### 2.6 Edge Function `admin-crear-contratista` (reglas compartidas, 2026-10-04)

Rama `feat/reglas-compartidas`. Contexto completo en
`docs/arquitectura/reglas-compartidas.md`.

- **Aplicado en staging:** la Edge Function `admin-crear-contratista`
  (versión 2 desde el 2026-10-05, `verify_jwt` activado). El `index.ts`
  desplegado es una sola línea que importa el de GitHub **fijado al commit
  `4f187b9`** (la versión 1, del 2026-10-04, estaba fijada a `f054c7b`; la 2
  busca la empresa a la vez que la autorización); Supabase lo
  empaqueta al desplegar, no consulta GitHub en cada petición. Para producción
  conviene desplegarla con `supabase functions deploy admin-crear-contratista`
  desde el repo.
- **Probado en staging** con un administrador temporal (borrado al terminar,
  junto con los dos contratistas de prueba; staging quedó con los mismos
  conteos que antes):
  - alta válida → 200, con cédula y nombre normalizados, y aviso a las 3
    unidades;
  - la misma cédula escrita distinto → 409 "Ya existe un contratista con esa
    cédula";
  - "POR CORREO" → 422 (la función SQL vieja lo aceptaba);
  - PRAIND vencido → 422; empresa inexistente → 404;
  - PRAIND sin fecha pero sin acceso → 200;
  - sin sesión → 401 de la puerta de Supabase; con la clave anónima → 401 de
    la función.
- **Aplicado en staging el 2026-10-05:** la migración
  `20261004170000_alta_de_contratistas_por_edge_function` (borra
  `panel_crear_contratista`). La ejecutó el dueño en el editor SQL de
  Supabase, así que no figura en el historial de migraciones. Verificado: la
  función ya no existe (ninguna sobrecarga), no tenía dependencias y
  `panel_crear_empresa` sigue. Desde ahora, en staging sólo el panel de esta
  rama puede crear contratistas.
- **Orden en producción:** 1) desplegar la función; 2) publicar el panel web;
  3) aplicar la migración. La función nueva puede convivir con la SQL vieja
  sin problema; sólo la migración rompe el panel viejo.

## 3. Cambios de la base local de los equipos (SQLite)

No se aplican a mano: cada equipo migra su base sola al abrir la app nueva.
Hay que tenerlos presentes al publicar versiones.

| Migración local | Qué agrega | Rama |
|---|---|---|
| 52 | `sincronizacion_estado.desfase_reloj_ms` (hora de internet guardada entre arranques) | `claude/hora-de-internet` → `claude/prueba-integral` |
| 57 | `movimientos_visita.placa` y `historial_visitas_sitio.placa` (medio de ingreso de las visitas agendadas) | `feat/escritorio-pendientes` |
| 58 | `movimientos_visita_remotos` (visitas abiertas por el otro equipo) y `movimientos_visita_remotos` permitido en `remotos_cerrados_aca` | `feat/visitas-movil` |

**Importante:** una vez que un equipo abre una versión con la migración 52,
su base queda en el esquema 52 y **una app anterior ya no la abre**: la
versión de esquema no coincide y la app se detiene al arrancar. Para volver
atrás hay que reinstalar y reconstruir el sitio desde la nube. Por eso,
antes de actualizar todas las porterías, se prueba primero en un solo
equipo.

## 4. Orden recomendado para producción

1. Respaldo (sección 1).
2. `cambio_nube_lleva_la_fila_de_ingresos`. Es independiente y ya está en
   la versión estable.
3. Publicar apps (escritorio y móvil) desde N1. Primero un equipo de
   prueba, luego el resto. El acceso negado en todas las puertas viaja en
   las apps y no necesita nada en Supabase.
4. Visitas y rutas: cada una con su rama, por separado. Visitas: la web de
   `feat/web-visitas` con su migración (sección 2.8), no la rama vieja.

## 5. Aplicado en producción

| Fecha | Migración | Autorizó | Verificación |
|---|---|---|---|
| 2026-09-29 | `cambio_nube_lleva_la_fila_de_ingresos` | Daniel Quintana (dueño), para el PR de `claude/fusion-integral` | `lleva_la_fila = true`; los 14 triggers siguen apuntando a la función. Definición anterior guardada: es la de `20260906044549_avisa_cambio_nube_segun_quien_escribe_no_quien_creo_la_fila.sql` (para deshacer). |
| 2026-09-29 | `hora_servidor_ms` | Daniel Quintana (dueño), para el PR de `claude/fusion-integral` | `diferencia_ms = 5`; ejecutable por `anon` y `authenticated`. |
| 2026-10-06 | Migración 1 (`ingresos_por_correo`) | Daniel Quintana (dueño), merge del PR #111 | La aplicó la integración de GitHub al mergear, con la versión del archivo (`20261003210000`). |
| 2026-10-06 | Migraciones 2 a 6 | Daniel Quintana (dueño), merge del PR #115 | Aplicadas por la integración con sus versiones exactas. Antes, 0 visitas abiertas duplicadas (0.5). |
| 2026-10-06 | Edge Functions `admin-crear-contratista` y `admin-editar-contratista` (nuevas, commit `88960f3`) y `admin-enviar-push` (versión 2, commit `aafae89`) | Daniel Quintana (dueño), antes del PR #116 | Desplegadas con un `index.ts` que importa el archivo del repo fijado al commit. Las tres responden 401 a quien no es administrador (arrancan y cargan el WebAssembly). La integración **no** despliega funciones: sólo migraciones. |
| 2026-10-06 | Migraciones 7 y 8 | Daniel Quintana (dueño), merge del PR #116 | Aplicadas por la integración con sus versiones exactas. La 7 borró `panel_crear_contratista` después de desplegar `admin-crear-contratista` (2.6). |
| 2026-10-06 | Migraciones 9 y 10 | Daniel Quintana (dueño), merge del PR #118 | Aplicadas por la integración, la 9 entera (funciones y `drop policy`) junto con la web de visitas nueva (2.8). |
| 2026-10-06 | Migración 11 | Daniel Quintana (dueño), merge del PR #119 | Aplicada por la integración, antes de las apps nuevas de escritorio y teléfono (2.9). |
| 2026-10-06 | Migraciones 12 y 13 | Daniel Quintana (dueño), merge del PR #121 | Aplicadas por la integración. Producción queda con 117 migraciones, las mismas y en el mismo orden que `main`. |
| 2026-10-06 | Edge Function `admin-create-usuario` (versión 4, commit `e9056b9`) | Daniel Quintana (dueño), después del PR #121 | Nombre del usuario en mayúscula con la regla del núcleo (2.11). Responde 401 a quien no es administrador. Las 13 funciones de producción quedan iguales al repo. En staging, `admin-create-usuario` (v8) y `admin-enviar-push` (v2) desde el mismo commit; `admin-suspend-device`, `admin-crear-codigo-vinculacion` y la función de base `panel_resumen_movimientos` (análisis de Syncfusion) quedan sin tocar. |
| 2026-10-06 | Edge Function `admin-provision-device` (versión 16, commit `470325a`) | Daniel Quintana (dueño), después del PR #125 | Si falla la emisión del código de vinculación ya no devuelve `String(error)` al panel: lo registra con `console.error` y responde un mensaje fijo (alerta #174 de CodeQL). Desplegada con la CLI de Supabase desde el archivo del repo, con `verify_jwt` activo. Responde 401 `{"error":"unauthorized"}` a un POST con la anon key y cuerpo `{}`. En staging, versión 8 con la misma prueba. |

Asesor de seguridad de Supabase después de aplicar: sólo avisos anteriores
(`plegar_texto` sin `search_path` fijo, `pg_net` en `public`, protección de
contraseñas filtradas desactivada), ninguno de estas migraciones.
