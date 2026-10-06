-- Todo nombre de persona o empresa en mayúscula (pedido del dueño
-- 2026-10-06). La regla vive en el núcleo (`reglas/src/nombre.rs`): la web de
-- visitas ya manda el nombre y la empresa de cada visitante en mayúscula
-- (`validarCita` por WebAssembly). `guardar_cita` repite las reglas del núcleo
-- para que nada se guarde distinto aunque alguien se salte la web: ahora
-- también pasa a mayúscula el nombre y la empresa del visitante.
--
-- Sólo cambia lo que se guarda de acá en adelante; no toca filas existentes.
-- Copia de `20261005133000_guardar_cita_reglas_del_nucleo.sql` con ese único
-- cambio.

create or replace function private.guardar_cita(
  p_correo text, p_id uuid, p_fecha_desde date, p_fecha_hasta date, p_motivo text,
  p_sitios uuid[], p_visitantes jsonb, p_hora_estimada time, p_hash text
)
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_hoy date := (now() at time zone 'America/Costa_Rica')::date;
  v_motivo text := nullif(btrim(p_motivo), '');
  v_n_sitios int := coalesce(array_length(p_sitios, 1), 0);
  v_n_visitantes int := jsonb_array_length(p_visitantes);
  v_visitante jsonb;
  v_cedula text;
  v_nombre text;
  v_empresa text;
  v_placa text;
  v_vistas text[] := '{}';
begin
  if v_motivo is not null and (length(v_motivo) > 1000 or v_motivo ~ '[[:cntrl:]]') then
    raise exception 'Motivo inválido';
  end if;
  if v_n_sitios < 1 or v_n_sitios > 100 then
    raise exception 'Debe indicar entre 1 y 100 sitios';
  end if;
  if v_n_sitios <> (select count(distinct s) from unnest(p_sitios) as s) then
    raise exception 'Sitios duplicados en la solicitud';
  end if;
  if exists (select 1 from unnest(p_sitios) as s where not exists (select 1 from public.sitios where id = s)) then
    raise exception 'Uno de los sitios indicados no existe';
  end if;
  if v_n_visitantes is null or v_n_visitantes < 1 or v_n_visitantes > 50 then
    raise exception 'Debe indicar entre 1 y 50 visitantes';
  end if;
  if p_fecha_desde < v_hoy then
    raise exception 'La fecha de inicio no puede ser anterior a hoy';
  end if;
  if p_fecha_hasta < p_fecha_desde then
    raise exception 'La fecha de fin debe ser posterior o igual a la de inicio';
  end if;

  insert into public.citas (id, anfitrion_correo, motivo, fecha_desde, fecha_hasta, hora_estimada, estado, contenido_hash)
  values (p_id, p_correo, v_motivo, p_fecha_desde, p_fecha_hasta, p_hora_estimada, 'VIGENTE', p_hash);

  insert into public.cita_sitios (cita_id, sitio_id)
  select p_id, s from unnest(p_sitios) as s;

  for v_visitante in select * from jsonb_array_elements(p_visitantes) loop
    -- Como el núcleo (`reglas::nombre`): espacios repetidos colapsados y en
    -- mayúscula, el nombre y la empresa.
    v_nombre := upper(btrim(regexp_replace(v_visitante ->> 'nombre', '[[:space:]]+', ' ', 'g')));
    v_empresa := upper(nullif(btrim(regexp_replace(v_visitante ->> 'empresa', '[[:space:]]+', ' ', 'g')), ''));
    v_placa := upper(nullif(btrim(v_visitante ->> 'placa_vehiculo'), ''));
    -- Forma única, igual que el núcleo y el check-in de la portería. Lo que
    -- no se puede normalizar (más de 20 caracteres, símbolos) se rechaza: la
    -- portería nunca lo reconocería.
    v_cedula := public.normalizar_cedula(v_visitante ->> 'cedula');

    if v_cedula is null or length(v_cedula) < 3 then
      raise exception 'Documento de visitante inválido: use de 3 a 20 letras o números.';
    end if;
    if v_nombre is null or length(v_nombre) < 2 or length(v_nombre) > 150 or v_nombre ~ '[[:cntrl:]]' then
      raise exception 'Nombre de visitante inválido';
    end if;
    if v_empresa is not null and (length(v_empresa) > 150 or v_empresa ~ '[[:cntrl:]]') then
      raise exception 'Empresa de visitante inválida';
    end if;
    if v_placa is not null and (length(v_placa) > 20 or v_placa ~ '[[:cntrl:]]') then
      raise exception 'Placa de visitante inválida';
    end if;
    if v_cedula = any(v_vistas) then
      raise exception 'Cédula duplicada dentro del mismo grupo: %', v_cedula;
    end if;
    v_vistas := array_append(v_vistas, v_cedula);

    insert into public.cita_visitantes (cita_id, cedula, nombre, empresa, placa_vehiculo)
    values (p_id, v_cedula, v_nombre, v_empresa, v_placa);
  end loop;
end;
$$;

revoke all on function private.guardar_cita(text, uuid, date, date, text, uuid[], jsonb, time, text) from public;
