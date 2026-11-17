-- Gafetes con dos estados: DISPONIBLE y PERDIDO (pedido del usuario,
-- 2026-11-16; local: MIGRACION_65). DE_BAJA era un estado sin regreso y un
-- gafete siempre se puede reimprimir: los que estaban de baja pasan a
-- PERDIDO, de donde se recuperan como cualquier otro.
--
-- Una app sin actualizar todavía puede mandar DE_BAJA: en vez de rechazarle
-- el cambio (quedaría fallido en su cola), la base lo guarda como PERDIDO.
-- Un PERDIDO ahora puede no tener responsable (no se sabe quién lo tenía, o
-- ya pagó); el espejo nunca exigió responsable, así que eso no cambia acá.
create or replace function private.gafetes_baja_es_perdido()
returns trigger
language plpgsql
set search_path to ''
as $$
begin
  if new.estado = 'DE_BAJA' then
    new.estado := 'PERDIDO';
  end if;
  return new;
end;
$$;

revoke execute on function private.gafetes_baja_es_perdido() from public;

drop trigger if exists gafetes_baja_es_perdido on public.gafetes;
create trigger gafetes_baja_es_perdido
before insert or update of estado on public.gafetes
for each row execute function private.gafetes_baja_es_perdido();

update public.gafetes set estado = 'PERDIDO' where estado = 'DE_BAJA';

alter table public.gafetes drop constraint if exists gafetes_estado_check;
alter table public.gafetes add constraint gafetes_estado_check
  check (estado in ('DISPONIBLE', 'PERDIDO'));
