# Despliegue a producción: lo que está en staging y todavía no en producción

Registro de todo cambio que se aplicó en **staging**
(`control-acceso-staging`, `pmrytjktlyiuikxuuxpr`) y que falta reproducir en
**producción** (`control-acceso-nube`, `xidaepyaljzkpbsxrqsm`). Regla del
proyecto: nada se aplica en producción sin autorización explícita del dueño.

Mantener este archivo al día: cada cambio que se aplique en staging se anota
aquí en el mismo commit. Cuando se aplique en producción, se mueve a la
sección "Aplicado en producción" con fecha y quién lo autorizó.

Última revisión: 2026-09-28.

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
| 2 | `crea_personas_vetadas` | `supabase/migrations/20260928010000_crea_personas_vetadas.sql` | `claude/veto-por-persona` | No |
| 3 | `rediseno_visitas_tablas_nuevas` | `supabase/migrations/20260927120000_rediseno_visitas_tablas_nuevas.sql` | `claude/rediseno-web-visitas` | Revisar con esa rama |
| 4 | `optimiza_rls_e_indices_de_visitas` | `supabase/migrations/20260927121500_optimiza_rls_e_indices_de_visitas.sql` | `claude/rediseno-web-visitas` | Revisar con esa rama |
| 5 | `rediseno_visitas_rpcs_vista_busqueda` | `supabase/migrations/20260927150000_rediseno_visitas_rpcs_vista_busqueda.sql` | `claude/rediseno-web-visitas` | Revisar con esa rama |
| 6 | `rutas_documento_tramo_viaje` (+ `_fix_search_path`, `_indices_fk`) | **No existe en el repo** | `feat/rutas-documento-tramo-viaje` (19 sep, sin unir) | Desconocido |

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

### 2.2 Veto por persona (`crea_personas_vetadas`)

Plan: `docs/features-futuras/plan-veto-por-persona.md`.

- **Qué crea:**
  - la función `public.normalizar_cedula(text)`;
  - la tabla `public.personas_vetadas`, con RLS y permisos por columna;
  - las RPC `vetar_persona`, `levantar_veto` y `listar_personas_vetadas`,
    que sólo puede usar el administrador del panel
    (`private.es_admin_global()`);
  - el trigger que avisa por `cambio_nube` a todos los sitios.
- **Prerrequisito:** que exista `private.es_admin_global()` (la usan las
  políticas desde `20260912064233_cierra_escritura_de_usuarios_a_admin_global.sql`).
  Comprobar:

  ```sql
  select to_regproc('private.es_admin_global') is not null as existe;
  ```

- **Compatibilidad:** es una tabla nueva. Las apps viejas no la usan, así
  que no les afecta. Se puede aplicar en cualquier momento. El bloqueo en
  las porterías empieza cuando los equipos tienen la versión con el veto,
  que baja la tabla y aplica la regla.
- **Verificar después de aplicar:** correr completo
  `supabase/tests/personas_vetadas_autorizacion.sql` en el editor SQL. Todo
  corre dentro de una transacción que se revierte y no deja datos. Debe
  terminar sin errores.
- **Casos de la cédula:** `normalizar_cedula` debe coincidir con el núcleo
  en los casos de `tests/vectores_cedula.tsv` (en staging coinciden los 18).
- **Deshacer** (sólo si todavía no se usó; borra los vetos):

  ```sql
  drop trigger if exists personas_vetadas_emitir_cambio_nube on public.personas_vetadas;
  drop function if exists private.emitir_cambio_nube_personas_vetadas();
  drop function if exists public.listar_personas_vetadas(boolean);
  drop function if exists public.levantar_veto(text, text);
  drop function if exists public.vetar_persona(text, text, text);
  drop table if exists public.personas_vetadas;
  drop function if exists public.personas_vetadas_actualizar_updated_at();
  drop function if exists public.normalizar_cedula(text);
  ```

### 2.3 Rediseño de la web de visitas (filas 3 a 5)

Vienen de otra línea de trabajo (`claude/rediseno-web-visitas`, ver
`docs/handoff-rediseno-web-visitas.md`). Se despliegan junto con esa rama,
en el orden de sus archivos, siguiendo sus propias notas. No se aplican
sueltas.

### 2.4 Tramos de viaje de rutas (fila 6): riesgo

Estas tres migraciones están aplicadas en staging, pero **ningún archivo
del repo las contiene**: se aplicaron directo, desde la rama
`feat/rutas-documento-tramo-viaje` (19 sep, sin unir). Producción no se
puede reconstruir a partir del repo mientras siga así.

Antes de llevarlas a producción:
1. Recuperar su SQL desde staging (`supabase_migrations.schema_migrations`
   guarda el texto de cada una).
2. Versionarlas como archivos en esa rama.
3. Decidir si esa rama sigue viva.

## 3. Cambios de la base local de los equipos (SQLite)

No se aplican a mano: cada equipo migra su base sola al abrir la app nueva.
Hay que tenerlos presentes al publicar versiones.

| Migración local | Qué agrega | Rama |
|---|---|---|
| 52 | `sincronizacion_estado.desfase_reloj_ms` (hora de internet guardada entre arranques) | `claude/hora-de-internet` → `claude/prueba-integral` |
| 53 | Tabla `personas_vetadas` y marca `vetos_actualizado_hasta` | `claude/veto-por-persona` |

**Importante:** una vez que un equipo abre una versión con la migración 53,
su base queda en el esquema 53 y **una app anterior ya no la abre**: la
versión de esquema no coincide y la app se detiene al arrancar. Para volver
atrás hay que reinstalar y reconstruir el sitio desde la nube. Por eso,
antes de actualizar todas las porterías, se prueba primero en un solo
equipo.

**Orden de unión a N1:** `claude/prueba-integral` (trae la 52) y después
`claude/veto-por-persona` (trae la 53). Al revés chocarían los números.

## 4. Orden recomendado para producción

1. Respaldo (sección 1).
2. `cambio_nube_lleva_la_fila_de_ingresos`. Es independiente y ya está en
   la versión estable.
3. `crea_personas_vetadas`. No afecta a las apps actuales. Luego correr
   `supabase/tests/personas_vetadas_autorizacion.sql`.
4. Publicar apps (escritorio y móvil) desde N1, cuando ya tenga el veto
   unido. Primero un equipo de prueba, luego el resto.
5. Visitas y rutas: cada una con su rama, por separado.

## 5. Aplicado en producción

(Vacío. Anotar aquí: fecha, migración, quién autorizó y resultado de la
verificación.)
