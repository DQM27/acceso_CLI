-- Hueco encontrado al preparar la sincronización incremental de citas: el
-- filtro va a ser `citas.updated_at > marca` (mismo patrón que
-- contratistas/empresas/usuarios/gafetes). Si alguien agrega/edita/quita un
-- visitante de una cita ya existente, la fila de `citas` en sí no cambia --
-- nada toca su `updated_at` -- así que ese cambio nunca calificaría en el
-- filtro incremental y el dispositivo se quedaría sin enterarse nunca del
-- visitante nuevo. Este trigger hace que tocar `cita_visitantes` también
-- toque a su `citas` padre, para que el mecanismo de sync que ya existe
-- (pensado para la cabecera) también capture cambios en el detalle.
create function public.cita_visitantes_toca_cita()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  update public.citas set updated_at = now()
  where id = coalesce(new.cita_id, old.cita_id);
  return coalesce(new, old);
end;
$$;

create trigger cita_visitantes_toca_cita
after insert or update or delete on public.cita_visitantes
for each row execute function public.cita_visitantes_toca_cita();
