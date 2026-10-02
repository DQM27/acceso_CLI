-- Revocación efectiva de dispositivos (ver
-- docs/features-futuras/propuesta-registro-dispositivos.md, hallazgo H2).
--
-- Un dispositivo tiene dos estados en la nube: vinculado o retirado
-- (`revoked_at`, definitivo). Antes de esto, retirar un dispositivo sólo
-- impedía que `device-auth` emitiera tokens NUEVOS: ninguna política RLS
-- consultaba `revoked_at`, así que el JWT ya emitido seguía leyendo y
-- escribiendo hasta vencer (12 h). Esta migración agrega:
--
-- 1. `clave_huella` en `dispositivos` (huella RFC 7638 de la clave pública
--    del equipo; la usa la migración siguiente). Se crea acá porque
--    `private.dispositivo_vigente()` ya la compara contra el claim `huella`
--    del JWT: sólo sirve un token emitido a la clave de ese dispositivo.
--    Un token sin `huella` (o un dispositivo sin clave) nunca es vigente.
-- 2. `private.dispositivo_vigente()` y una política RESTRICTIVA
--    "solo dispositivos vigentes" en cada tabla de `public` con RLS. Las
--    políticas restrictivas se combinan con AND sobre las permisivas que ya
--    existen, así que no cambian lo que cada rol puede hacer: sólo le quitan
--    todo acceso a un dispositivo retirado. Para
--    cualquier JWT sin claim `sitio_id` (sesiones humanas del panel, web de
--    visitas) la función devuelve `true` y la política no interviene.
--    Se aplica en bucle a TODAS las tablas con RLS (no una lista fija) para
--    que staging y producción queden iguales aunque tengan tablas distintas;
--    una tabla nueva debe agregar la misma política en su propia migración
--    (ver docs/arquitectura/arquitectura-supabase.md, sección 4.4).
-- 3. Un trigger que avisa por Realtime (`dispositivo_expulsado`, canal
--    `sitio:<id>`) cuando un dispositivo se retira, para que la app corte
--    el canal y deje de usar su token en el acto.
-- 4. Se elimina la suspensión temporal (`suspended_at`, migración
--    20260906103046): decisión 2026-09-30, sólo existen Registrar y Retirar.
--    Un dispositivo que estuviera suspendido al migrar queda retirado.

alter table public.dispositivos
  add column if not exists clave_huella text;

create unique index if not exists dispositivos_clave_huella_key
  on public.dispositivos (clave_huella)
  where clave_huella is not null;

-- `sub` sólo se castea a uuid cuando el JWT es de dispositivo (trae
-- `sitio_id`): los tokens de dispositivo siempre llevan el id como `sub`.
create or replace function private.dispositivo_vigente()
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select case
    when ((select auth.jwt()) ->> 'sitio_id') is null then true
    else exists (
      select 1
      from public.dispositivos d
      where d.id = ((select auth.jwt()) ->> 'sub')::uuid
        and d.revoked_at is null
        and d.clave_huella = ((select auth.jwt()) ->> 'huella')
    )
  end
$$;

revoke all on function private.dispositivo_vigente() from public;
grant execute on function private.dispositivo_vigente() to authenticated, service_role;

do $$
declare
  v_tabla record;
begin
  for v_tabla in
    select c.relname
    from pg_class c
    join pg_namespace n on n.oid = c.relnamespace
    where n.nspname = 'public'
      and c.relkind = 'r'
      and c.relrowsecurity
  loop
    execute format(
      'drop policy if exists "solo dispositivos vigentes" on public.%I',
      v_tabla.relname
    );
    execute format(
      'create policy "solo dispositivos vigentes" on public.%I
         as restrictive for all to authenticated
         using ((select private.dispositivo_vigente()))
         with check ((select private.dispositivo_vigente()))',
      v_tabla.relname
    );
  end loop;
end $$;

create or replace function private.avisar_dispositivo_expulsado()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  if not (old.revoked_at is null and new.revoked_at is not null) then
    return null;
  end if;

  perform realtime.send(
    pg_catalog.jsonb_build_object(
      'dispositivo_id', new.id,
      'motivo', 'revocado',
      'ocurrido_en', pg_catalog.now()
    ),
    'dispositivo_expulsado',
    'sitio:' || new.sitio_id::text,
    true
  );
  return null;
end;
$$;

revoke all on function private.avisar_dispositivo_expulsado() from public;

drop trigger if exists dispositivos_avisar_expulsion on public.dispositivos;
create trigger dispositivos_avisar_expulsion
  after update of revoked_at on public.dispositivos
  for each row execute function private.avisar_dispositivo_expulsado();

-- Suspensión temporal retirada (ver punto 4 del encabezado). Va al final:
-- las funciones de arriba ya no la leen.
update public.dispositivos
   set revoked_at = coalesce(revoked_at, suspended_at)
 where suspended_at is not null;
alter table public.dispositivos drop column if exists suspended_at;
