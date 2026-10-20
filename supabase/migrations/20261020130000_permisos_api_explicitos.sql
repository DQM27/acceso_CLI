-- Permisos de la API explícitos: nada de lo que no se dio a propósito.
--
-- Hasta ahora toda tabla, vista, secuencia y función nueva de `public` recibía
-- sola todos los permisos para `anon`, `authenticated` y `service_role` (los
-- "default privileges" de Supabase). Supabase va a dejar de hacerlo
-- (https://github.com/orgs/supabase/discussions/45329) y lo adelantamos acá,
-- con lo que eso dejó sembrado:
--
-- 1. `anon` (la llave publicable, sin sesión) tenía SELECT/INSERT/UPDATE/
--    DELETE/TRUNCATE sobre ~20 tablas y EXECUTE sobre las funciones de
--    trigger y tres ayudantes. Lo frenaba sólo la RLS. Nadie lo usa en
--    producción: los equipos entran con su propio JWT (rol `authenticated`,
--    ver `_shared/dispositivos.ts`) y el panel y la web de visitas con sesión
--    de Supabase Auth. Realtime tampoco: corre con el rol del JWT de quien se
--    suscribe, así que no se toca.
-- 2. Las tablas y funciones nuevas ya no reciben nada solas: cada migración
--    da sus GRANT a mano, junto con su RLS (ver AGENTS.md, "Supabase").
--
-- Qué NO cambia: los permisos de `authenticated` y `service_role` sobre lo
-- que ya existe (están escritos en cada objeto, no dependen de la regla
-- automática), las políticas RLS y las publicaciones de Realtime.
--
-- Excepción, sólo de staging: `telemetria_diagnostico` (la app inserta como
-- anon) y `reglas_telemetria_vigentes` (la app la consulta como anon). Viven
-- en supabase/scripts/, no en producción; si existen, se dejan como están.

-- 1. Reglas automáticas: lo que cree `postgres` (las migraciones) en `public`
--    ya no recibe permisos solo. Las tres primeras son las que recomienda
--    Supabase.
alter default privileges for role postgres in schema public
  revoke select, insert, update, delete, truncate, references, trigger on tables
  from anon, authenticated, service_role;
alter default privileges for role postgres in schema public
  revoke usage, select, update on sequences from anon, authenticated, service_role;
alter default privileges for role postgres in schema public
  revoke execute on functions from anon, authenticated, service_role;
-- El EXECUTE de `public` sobre las funciones nuevas es una regla GLOBAL de
-- Postgres, y una regla por esquema no puede quitar lo que da la global (la
-- variante `in schema public ... from public` no hace nada; lo atrapó el test
-- 5). Se revoca a nivel global para lo que crea `postgres`, en todos los
-- esquemas: una función nueva de `private` que use la RLS también necesita su
-- `grant execute ... to authenticated` explícito.
alter default privileges for role postgres
  revoke execute on functions from public;

-- 2. Tablas, vistas y secuencias existentes: fuera `anon`.
do $$
declare
  v_objeto record;
begin
  for v_objeto in
    select c.oid::regclass as nombre, c.relkind
      from pg_catalog.pg_class c
      join pg_catalog.pg_namespace n on n.oid = c.relnamespace
      left join pg_catalog.pg_depend d on d.objid = c.oid and d.deptype = 'e'
     where n.nspname = 'public'
       and c.relkind in ('r', 'v', 'm', 'f', 'p', 'S')
       and d.objid is null  -- lo que trae una extensión es de la extensión
       and c.relname not in ('telemetria_diagnostico')
  loop
    if v_objeto.relkind = 'S' then
      execute pg_catalog.format('revoke all on sequence %s from anon', v_objeto.nombre);
    else
      execute pg_catalog.format('revoke all on table %s from anon', v_objeto.nombre);
    end if;
  end loop;
end
$$;

-- 3. Funciones existentes: fuera `anon` y `public` (todos los roles heredan de
--    `public`, así que quitárselo sólo a `anon` no alcanza). `authenticated` y
--    `service_role` conservan exactamente lo que tenían: se mira ANTES de
--    revocar y se devuelve explícito, para no reabrir funciones que una
--    migración anterior les cerró a propósito (p. ej. `sync_access_policy`).
--    Las de trigger no necesitan EXECUTE para dispararse.
do $$
declare
  v_funcion record;
  v_autenticado boolean;
  v_servicio boolean;
begin
  for v_funcion in
    select p.oid, p.oid::regprocedure as firma
      from pg_catalog.pg_proc p
      join pg_catalog.pg_namespace n on n.oid = p.pronamespace
      left join pg_catalog.pg_depend d on d.objid = p.oid and d.deptype = 'e'
     where n.nspname = 'public'
       and p.prokind in ('f', 'p')
       and d.objid is null
       and p.proname not in ('reglas_telemetria_vigentes')
       and (pg_catalog.has_function_privilege('anon', p.oid, 'execute')
            or pg_catalog.aclcontains(
                 coalesce(p.proacl, pg_catalog.acldefault('f', p.proowner)),
                 pg_catalog.makeaclitem(0, p.proowner, 'EXECUTE', false)))
  loop
    v_autenticado := pg_catalog.has_function_privilege('authenticated', v_funcion.oid, 'execute');
    v_servicio := pg_catalog.has_function_privilege('service_role', v_funcion.oid, 'execute');
    execute pg_catalog.format('revoke all on function %s from public, anon', v_funcion.firma);
    if v_autenticado then
      execute pg_catalog.format('grant execute on function %s to authenticated', v_funcion.firma);
    end if;
    if v_servicio then
      execute pg_catalog.format('grant execute on function %s to service_role', v_funcion.firma);
    end if;
  end loop;
end
$$;
