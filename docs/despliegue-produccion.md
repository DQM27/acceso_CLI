# Despliegue a producción: lo que está en staging y todavía no en producción

Registro de todo cambio que se aplicó en **staging**
(`control-acceso-staging`, `pmrytjktlyiuikxuuxpr`) y que falta reproducir en
**producción** (`control-acceso-nube`, `xidaepyaljzkpbsxrqsm`). Regla del
proyecto: nada se aplica en producción sin autorización explícita del dueño.

Mantener este archivo al día: cada cambio que se aplique en staging se anota
aquí en el mismo commit. Cuando se aplique en producción, se mueve a la
sección "Aplicado en producción" con fecha y quién lo autorizó.

Última revisión: 2026-10-05.

## 0. Lista de verificación para el PR a main (auditada el 2026-10-05)

Cruce hecho por nombre entre el repo (114 migraciones de nube), staging (53
en su historial) y producción (104 en su historial, sólo lectura de la lista).

### 0.1 Qué lleva el PR

Las ramas forman una sola cadena, cada una contiene a la anterior:

`main` → `feat/ingreso-por-correo` → `fix/carreras-ingresos-y-gafetes` →
`feat/reglas-compartidas` → `feat/web-visitas` → `feat/escritorio-pendientes`

Un PR de **`feat/escritorio-pendientes` contra `main`** lleva todo junto, sin
choques: main no tiene ningún commit que la rama no tenga. No hay versiones de
migración repetidas en el repo. (`prueba/radix-visitas` sale de
`feat/web-visitas` y es aparte: no forma parte de este PR.)

### 0.2 Migraciones de nube pendientes en producción (11, en este orden)

Producción tiene las 104 primeras del repo; le faltan exactamente estas. En
staging se comparan por **nombre** (staging anota la hora de aplicación, no la
versión del archivo).

| # | Archivo del repo | En staging | Cuándo va en producción |
|---|---|---|---|
| 1 | `20261003210000_ingresos_por_correo.sql` | En el historial | Antes de las apps nuevas |
| 2 | `20261004120000_conflictos_misma_unidad_y_gafete_de_visita.sql` | En el historial | Antes de las apps nuevas (ver 2.5b) |
| 3 | `20261004130000_visitas_ven_otras_unidades.sql` | En el historial | Igual que la 2. Antes, revisar 0.5 |
| 4 | `20261004140000_gafete_de_visita_unico_en_visitas.sql` | En el historial | Igual que la 2. Antes, revisar 0.5 |
| 5 | `20261004150000_persona_adentro_por_una_sola_via.sql` | En el historial | Igual que la 2 |
| 6 | `20261004160000_registrar_token_push_sin_choques.sql` | **A mano**, no figura en el historial. Verificado: la función tiene el candado `token_push:` | Cuando sea; no rompe nada |
| 7 | `20261004170000_alta_de_contratistas_por_edge_function.sql` | **A mano**, no figura. Verificado: `panel_crear_contratista` ya no existe | **Después** de desplegar la Edge Function `admin-crear-contratista` y publicar el panel nuevo: borra la función SQL que usa el panel viejo (ver 2.6) |
| 8 | `20261005120000_usuarios_nombre_obligatorio.sql` | En el historial | Cuando sea (ver 2.7) |
| 9 | `20261005130000_visitas_web_anfitrion.sql` | **En dos partes:** las funciones en el historial como `visitas_web_anfitrion_funciones`; los 7 `drop policy`, a mano. Verificado: 4 funciones nuevas y 0 políticas de escritura directa del anfitrión | Las funciones, cuando sea; los `drop policy`, **después** de publicar la web de visitas nueva (ver 2.8). Si se aplica el archivo entero, tiene que ser junto con la web |
| 10 | `20261005133000_guardar_cita_reglas_del_nucleo.sql` | En el historial | Después de la 9 (reemplaza `guardar_cita`, que crea la 9). No rompe nada: la web nueva ya valida igual. Antes, la consulta de sólo lectura de 2.8 |
| 11 | `20261005140000_movimientos_visita_placa.sql` | En el historial | **Antes** de la app de escritorio nueva (ver 2.9) |

Las tres "a mano" (6, 7 y la segunda parte de la 9) existen en staging aunque
su historial no las nombre. Para producción conviene aplicarlas con la
herramienta de migraciones, así quedan anotadas; si la herramienta se cuelga
con los `drop` (le pasó a staging), el editor SQL sirve igual, y se verifican
con las mismas consultas.

### 0.3 Edge Functions

| Función | Estado | Producción |
|---|---|---|
| `admin-crear-contratista` | Nueva. En staging, versión 3 | Desplegar antes de publicar el panel y antes de la migración 7 |
| `admin-editar-contratista` | Nueva. En staging, versión 1 | Desplegar antes de publicar el panel |
| Las otras 10 (`device-auth`, `admin-*`, `device-vincular`, `sync-access-policy`) | Sólo cambia la línea de tipos (`functions-js@2`), sin cambio de comportamiento | Volver a desplegar es opcional |

### 0.4 Base local de los equipos (SQLite)

Main está en el esquema **54**; la rama llega al **57**:
- **55:** ingreso por correo;
- **56:** lápidas de cierres de remotos;
- **57:** placa de las visitas.

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

1. Respaldo (sección 1).
2. Migración 1 (`ingresos_por_correo`).
3. Revisar 0.5 y aplicar las migraciones 2 a 5.
4. Migraciones 6 y 8, y la parte de funciones de la 9.
5. Migración 10 (`movimientos_visita_placa`).
6. Desplegar `admin-crear-contratista` y `admin-editar-contratista`.
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
o en vehículo), la salida (si ya está adentro en ese equipo) o un aviso (azul
si la cita es para otro día). Sin listas ni historial.

- **Sin migraciones nuevas.** El teléfono ya bajaba las citas y su cola ya
  sabía subir movimientos de visita.
- **Las reglas son las de escritorio:** los chequeos contra la nube de una
  entrada (gafete en uso en el otro equipo, visitante adentro en otra unidad)
  se pasaron del comando de escritorio a
  `application::registrar_entrada_visita_verificada`, y ahora la usan los dos.
- **Orden en producción:** igual que el escritorio nuevo, **después** de la
  migración `20261005140000_movimientos_visita_placa.sql` (fila 11 de 0.2): el
  teléfono también sube la placa con cada movimiento de visita.

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

Asesor de seguridad de Supabase después de aplicar: sólo avisos anteriores
(`plegar_texto` sin `search_path` fijo, `pg_net` en `public`, protección de
contraseñas filtradas desactivada), ninguno de estas migraciones.
