# Despliegue a producción: lo que está en staging y todavía no en producción

Registro de todo cambio que se aplicó en **staging**
(`control-acceso-staging`, `pmrytjktlyiuikxuuxpr`) y que falta reproducir en
**producción** (`control-acceso-nube`, `xidaepyaljzkpbsxrqsm`). Regla del
proyecto: nada se aplica en producción sin autorización explícita del dueño.

Mantener este archivo al día: cada cambio que se aplique en staging se anota
aquí en el mismo commit. Cuando se aplique en producción, se mueve a la
sección "Aplicado en producción" con fecha y quién lo autorizó.

Última revisión: 2026-09-29.

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
| 1 | `cambio_nube_lleva_la_fila_de_ingresos` | `supabase/migrations/20260927200000_cambio_nube_lleva_la_fila_de_ingresos.sql` | `claude/nucleo-n1-n3` (N1, estable) | No |
| 2 | `rediseno_visitas_tablas_nuevas` | `supabase/migrations/20260927120000_rediseno_visitas_tablas_nuevas.sql` | `claude/rediseno-web-visitas` | Revisar con esa rama |
| 3 | `optimiza_rls_e_indices_de_visitas` | `supabase/migrations/20260927121500_optimiza_rls_e_indices_de_visitas.sql` | `claude/rediseno-web-visitas` | Revisar con esa rama |
| 4 | `rediseno_visitas_rpcs_vista_busqueda` | `supabase/migrations/20260927150000_rediseno_visitas_rpcs_vista_busqueda.sql` | `claude/rediseno-web-visitas` | Revisar con esa rama |
| 5 | `rutas_documento_tramo_viaje` (+ `_fix_search_path`, `_indices_fk`) | **No existe en el repo** | `feat/rutas-documento-tramo-viaje` (19 sep, sin unir) | Desconocido |
| 6 | `hora_servidor_ms` | `supabase/migrations/20260929020000_hora_servidor_ms.sql` | `claude/fusion-integral` (reloj en ms) | No |

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

## 3. Cambios de la base local de los equipos (SQLite)

No se aplican a mano: cada equipo migra su base sola al abrir la app nueva.
Hay que tenerlos presentes al publicar versiones.

| Migración local | Qué agrega | Rama |
|---|---|---|
| 52 | `sincronizacion_estado.desfase_reloj_ms` (hora de internet guardada entre arranques) | `claude/hora-de-internet` → `claude/prueba-integral` |

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
4. Visitas y rutas: cada una con su rama, por separado.

## 5. Aplicado en producción

(Vacío. Anotar aquí: fecha, migración, quién autorizó y resultado de la
verificación.)
