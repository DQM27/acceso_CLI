-- Mensajes de las funciones de gafetes PRAIND con el nombre que usa la
-- portería: "gafete PRAIND", no "carné" (pedido del usuario 2026-11-17: el
-- teléfono mostraba "No tiene permiso para cargar carnés PRAIND.").
--
-- Mismas funciones de 20261115120000_carnes_praind, sin cambiar nada más que
-- el texto de los errores (los códigos de error, que es lo que miran el
-- Worker y las pruebas, quedan iguales). `create or replace` conserva los
-- permisos (`grant`/`revoke`) que ya tenían.

create or replace function public.praind_registrar(
  p_contratista_id uuid,
  p_clave text,
  p_sha256 text,
  p_bytes bigint,
  p_tipo_mime text,
  p_enlace_origen text default null,
  p_nombre_en_origen text default null
)
returns text
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_anterior text;
  v_extension text := case p_tipo_mime when 'image/png' then 'png' when 'image/jpeg' then 'jpg' end;
begin
  if not (select private.praind_puede_cargar()) then
    raise exception 'No tiene permiso para cargar gafetes PRAIND.' using errcode = '42501';
  end if;
  if v_extension is null then
    raise exception 'El gafete PRAIND debe ser una imagen PNG o JPEG.' using errcode = '22023';
  end if;
  if p_sha256 is null or p_sha256 !~ '^[0-9a-f]{64}$'
     or p_clave is distinct from 'contratistas/' || p_contratista_id || '/' || p_sha256 || '.' || v_extension then
    raise exception 'La ruta del gafete PRAIND no corresponde al contratista ni al archivo.' using errcode = '22023';
  end if;

  select c.praind_foto into v_anterior
    from public.contratistas c
   where c.id = p_contratista_id
   for update;
  if not found then
    raise exception 'El contratista no existe.' using errcode = 'P0002';
  end if;

  if v_anterior = p_clave then
    return null;
  end if;

  update public.fotos_praind
     set reemplazada_en = now()
   where contratista_id = p_contratista_id and reemplazada_en is null;

  insert into public.fotos_praind
    (contratista_id, clave, sha256, bytes, tipo_mime, enlace_origen, nombre_en_origen, cargado_por)
  values
    (p_contratista_id, p_clave, p_sha256, p_bytes, p_tipo_mime,
     nullif(btrim(p_enlace_origen), ''), nullif(btrim(p_nombre_en_origen), ''),
     coalesce((select auth.email()), 'dispositivo:' || ((select auth.jwt()) ->> 'sub')))
  on conflict (clave) do update
     set contratista_id = excluded.contratista_id,
         reemplazada_en = null,
         cargado_por = excluded.cargado_por,
         cargado_en = now();

  update public.contratistas set praind_foto = p_clave where id = p_contratista_id;

  return v_anterior;
end;
$$;

create or replace function public.praind_quitar(p_contratista_id uuid)
returns text
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_anterior text;
begin
  if not (select private.es_admin_global()) then
    raise exception 'Sólo el panel puede quitar gafetes PRAIND.' using errcode = '42501';
  end if;

  select c.praind_foto into v_anterior
    from public.contratistas c
   where c.id = p_contratista_id
   for update;
  if not found then
    raise exception 'El contratista no existe.' using errcode = 'P0002';
  end if;
  if v_anterior is null then
    return null;
  end if;

  update public.fotos_praind
     set reemplazada_en = now()
   where contratista_id = p_contratista_id and reemplazada_en is null;
  update public.contratistas set praind_foto = null where id = p_contratista_id;

  return v_anterior;
end;
$$;
