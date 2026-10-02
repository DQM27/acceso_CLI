-- Al retirar o eliminar un equipo, sus sesiones abiertas se cierran en la
-- bitácora con el motivo 'equipo_retirado'. Antes quedaban "abiertas" para
-- siempre: el equipo retirado ya no puede llamar a la nube para avisar que
-- salió. Es un disparador (y no código en las Edge Functions) para que
-- valga sin importar desde dónde se retire o elimine el equipo.

alter table public.bitacora_sesiones
  drop constraint bitacora_sesiones_motivo_cierre_check;
alter table public.bitacora_sesiones
  add constraint bitacora_sesiones_motivo_cierre_check
  check (motivo_cierre in ('salida', 'otra_unidad', 'desplazada', 'sin_cierre', 'equipo_retirado'));

create or replace function private.cerrar_sesiones_de_equipo_retirado()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  update public.bitacora_sesiones b
     set cerrada_en = greatest(pg_catalog.now(), b.iniciada_en),
         motivo_cierre = 'equipo_retirado'
   where b.dispositivo_id = old.id
     and b.cerrada_en is null;
  delete from public.sesiones_usuario s where s.dispositivo_id = old.id;
  return case when tg_op = 'DELETE' then old else new end;
end;
$$;

revoke all on function private.cerrar_sesiones_de_equipo_retirado() from public;

-- Retirar: `revoked_at` pasa de nulo a una fecha.
create trigger cerrar_sesiones_al_retirar_equipo
  after update of revoked_at on public.dispositivos
  for each row
  when (old.revoked_at is null and new.revoked_at is not null)
  execute function private.cerrar_sesiones_de_equipo_retirado();

-- Eliminar: antes de borrar, mientras la bitácora todavía apunta al equipo
-- (después su `dispositivo_id` pasa a nulo y conserva la etiqueta y la unidad).
create trigger cerrar_sesiones_al_eliminar_equipo
  before delete on public.dispositivos
  for each row
  execute function private.cerrar_sesiones_de_equipo_retirado();
