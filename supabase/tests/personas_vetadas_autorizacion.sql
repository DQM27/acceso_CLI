-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Veto por persona: ver supabase/migrations/20260928010000_crea_personas_vetadas.sql
-- y docs/features-futuras/plan-veto-por-persona.md.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

-- La normalización coincide con la del núcleo (tests/vectores_cedula.tsv).
do $$
begin
  if public.normalizar_cedula('01-1234-0567') is distinct from '112340567'
     or public.normalizar_cedula(' 1.1234.0567 ') is distinct from '112340567'
     or public.normalizar_cedula('ab 123-456') is distinct from 'AB123456'
     or public.normalizar_cedula('1234/5678') is not null
     or public.normalizar_cedula('  - . ') is not null then
    raise exception 'normalizar_cedula no coincide con el núcleo';
  end if;
end $$;

set local role authenticated;

-- Alguien que NO es administrador (un equipo, o un usuario cualquiera): no
-- veta, no lista, no escribe directo y no lee el motivo. Sí lee lo mínimo
-- para bloquear.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  begin
    perform public.vetar_persona('112340567', 'X', 'motivo de prueba');
    raise exception 'un no administrador pudo vetar';
  exception when insufficient_privilege then null;
  end;
  begin
    perform public.listar_personas_vetadas(true);
    raise exception 'un no administrador pudo listar los vetos con motivo';
  exception when insufficient_privilege then null;
  end;
  begin
    perform motivo from public.personas_vetadas limit 1;
    raise exception 'un no administrador pudo leer el motivo';
  exception when insufficient_privilege then null;
  end;
  begin
    insert into public.personas_vetadas (cedula, motivo, vetado_por, vetado_por_correo)
    values ('112340567', 'abc', gen_random_uuid(), 'x');
    raise exception 'un no administrador pudo insertar directo';
  exception when insufficient_privilege then null;
  end;
  perform id, cedula, levantado_en, updated_at from public.personas_vetadas limit 1;
end $$;

-- El administrador: veta (cualquier formato de cédula), no duplica, levanta,
-- conserva la historia y puede volver a vetar.
select set_config('request.jwt.claims',
  json_build_object(
    'role', 'authenticated',
    'sub', '00000000-0000-0000-0000-0000000000aa',
    'email', current_setting('diagnostico.correo_admin')
  )::text,
  true);
do $$
begin
  perform public.vetar_persona('01-1234-0567', 'Persona Prueba', 'motivo de prueba');
  begin
    perform public.vetar_persona('112340567', 'Persona Prueba', 'otra vez');
    raise exception 'se permitió un segundo veto vigente para la misma persona';
  exception when unique_violation then null;
  end;
  if (select count(*) from public.listar_personas_vetadas(false) where cedula = '112340567') <> 1 then
    raise exception 'el veto vigente no aparece en la lista';
  end if;
  perform public.levantar_veto('1-1234-0567', 'se aclaró la situación');
  begin
    perform public.levantar_veto('112340567', 'de nuevo');
    raise exception 'se levantó un veto que ya no estaba vigente';
  exception when no_data_found then null;
  end;
  if (select count(*) from public.listar_personas_vetadas(true) where cedula = '112340567') <> 1 then
    raise exception 'levantar el veto no conservó la historia';
  end if;
  perform public.vetar_persona('112340567', null, 'vuelve a vetarse');
end $$;

rollback;
