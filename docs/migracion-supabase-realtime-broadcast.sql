-- Broadcast privado por sitio para la sincronización en vivo.
-- Preparado para el proyecto Supabase xidaepyaljzkpbsxrqsm.
-- Pendiente de aplicar en remoto si el MCP sigue en modo de solo lectura.

drop policy if exists "dispositivos reciben broadcast de su sitio"
on realtime.messages;

create policy "dispositivos reciben broadcast de su sitio"
on realtime.messages
for select
to authenticated
using (
  realtime.messages.private = true
  and realtime.messages.extension = 'broadcast'
  and (select realtime.topic()) = ('sitio:' || ((select auth.jwt()) ->> 'sitio_id'))
);

create schema if not exists private;
revoke all on schema private from public, anon, authenticated;

create or replace function private.emitir_cambio_nube_sitio()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_sitio_id uuid;
  v_dispositivo_id uuid;
begin
  if tg_op = 'DELETE' then
    v_sitio_id := old.sitio_id;
  else
    v_sitio_id := new.sitio_id;
  end if;

  if v_sitio_id is null then
    return null;
  end if;

  if tg_table_name = 'ingresos' then
    if tg_op = 'DELETE' then
      v_dispositivo_id := coalesce(old.dispositivo_salida_id, old.dispositivo_entrada_id);
    else
      v_dispositivo_id := coalesce(new.dispositivo_salida_id, new.dispositivo_entrada_id);
    end if;
  else
    if tg_op = 'DELETE' then
      v_dispositivo_id := old.dispositivo_origen_id;
    else
      v_dispositivo_id := new.dispositivo_origen_id;
    end if;
  end if;

  perform realtime.send(
    pg_catalog.jsonb_build_object(
      'schema', tg_table_schema,
      'table', tg_table_name,
      'operation', tg_op,
      'sitio_id', v_sitio_id,
      'dispositivo_id', v_dispositivo_id,
      'changed_at', pg_catalog.now()
    ),
    'cambio_nube',
    'sitio:' || v_sitio_id::text,
    true
  );

  return null;
end;
$$;

revoke all on function private.emitir_cambio_nube_sitio() from public, anon, authenticated;

drop trigger if exists empresas_emitir_cambio_nube on public.empresas;
create trigger empresas_emitir_cambio_nube
after insert or update or delete on public.empresas
for each row execute function private.emitir_cambio_nube_sitio();

drop trigger if exists contratistas_emitir_cambio_nube on public.contratistas;
create trigger contratistas_emitir_cambio_nube
after insert or update or delete on public.contratistas
for each row execute function private.emitir_cambio_nube_sitio();

drop trigger if exists gafetes_emitir_cambio_nube on public.gafetes;
create trigger gafetes_emitir_cambio_nube
after insert or update or delete on public.gafetes
for each row execute function private.emitir_cambio_nube_sitio();

drop trigger if exists ingresos_emitir_cambio_nube on public.ingresos;
create trigger ingresos_emitir_cambio_nube
after insert or update or delete on public.ingresos
for each row execute function private.emitir_cambio_nube_sitio();
