-- `registrar_token_push` sin choques cuando dos equipos registran el mismo
-- token a la vez (revisión del 2026-10-04; ver
-- docs/auditorias/revision-carreras-2026-10-04.md).
--
-- La función borra el token de cualquier OTRO equipo y después inserta (o
-- actualiza) la fila del equipo que llama. Si dos equipos mandan el mismo
-- token al mismo tiempo (un teléfono revinculado como equipo nuevo mientras
-- la vinculación vieja todavía inicia sesión), ninguno de los dos `delete`
-- ve la fila sin confirmar del otro, los dos `insert` siguen, y el segundo
-- choca con `unique (token)`: el registro falla con un 409 en vez de
-- quedarse con el último en llegar.
--
-- Arreglo: un candado por token (`pg_advisory_xact_lock`) antes del
-- `delete`. El segundo espera a que el primero confirme y entonces su
-- `delete` sí ve (y quita) la fila del primero: gana el último en llegar,
-- que es lo que la función ya pretendía. Misma firma y mismos permisos.

create or replace function public.registrar_token_push(p_token text)
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_equipo public.dispositivos;
  v_token text := pg_catalog.btrim(p_token);
begin
  v_equipo := private.dispositivo_que_llama();
  if v_equipo.id is null then
    raise exception 'Sólo un equipo registrado puede registrar un token push' using errcode = '42501';
  end if;

  -- Los tokens de FCM rondan los 150-200 caracteres; el tope evita guardar
  -- basura arbitraria.
  if v_token is null or pg_catalog.length(v_token) not between 20 and 4096 then
    raise exception 'Token push inválido' using errcode = '22023';
  end if;

  -- Dos registros del mismo token se atienden de a uno (ver arriba).
  perform pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended('token_push:' || v_token, 0));

  delete from public.tokens_push
   where token = v_token and dispositivo_id <> v_equipo.id;

  insert into public.tokens_push (dispositivo_id, sitio_id, token)
  values (v_equipo.id, v_equipo.sitio_id, v_token)
  on conflict (dispositivo_id) do update
    set token = excluded.token,
        sitio_id = excluded.sitio_id,
        actualizado_en = pg_catalog.now();
end;
$$;
