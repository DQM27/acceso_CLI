-- El aviso en vivo `cambio_nube` lleva la fila completa también para las
-- otras tres clases de ingreso: proveedores, ingresos por correo y préstamos
-- de gafete provisional KOF.
--
-- Hasta ahora sólo `ingresos` (contratistas) traía `registro`
-- (`20260929075806_cambio_nube_lleva_la_fila_de_ingresos`). En las otras
-- tablas el aviso sólo decía "cambió" y cada equipo tenía que ir a buscar la
-- fila: un viaje más, y mientras tanto su caché del otro equipo
-- (`*_remotos`) seguía atrasada. Esa caché es la que mira la verificación
-- local de un ingreso (persona adentro, gafete ocupado) antes de ir a la
-- nube, así que cuanto antes se actualiza, menos depende el ingreso de la
-- consulta en vivo. Las apps la guardan directo (`nube::en_vivo` en el
-- núcleo Rust).
--
-- Compatible hacia atrás: las apps que no conocen `registro` en estas tablas
-- lo ignoran y siguen sincronizando la tabla como antes. El canal sigue
-- siendo privado por sitio (RLS en `realtime.messages`) y la fila es la
-- misma que cada equipo del sitio ya puede leer por REST: ninguna de estas
-- tablas tiene permisos por columna.
--
-- Único cambio respecto de la versión anterior: la lista de tablas del
-- `case` de `registro`.

create or replace function private.emitir_cambio_nube_sitio()
 returns trigger
 language plpgsql
 security definer
 set search_path to ''
as $function$
declare
  v_sitio_id uuid;
  v_dispositivo_id text;
  v_fila jsonb;
begin
  if tg_op = 'DELETE' then
    v_sitio_id := old.sitio_id;
    v_fila := pg_catalog.to_jsonb(old);
  else
    v_sitio_id := new.sitio_id;
    v_fila := pg_catalog.to_jsonb(new);
  end if;

  if v_sitio_id is null then
    return null;
  end if;

  v_dispositivo_id := (select auth.jwt() ->> 'sub');

  perform realtime.send(
    pg_catalog.jsonb_build_object(
      'schema', tg_table_schema,
      'table', tg_table_name,
      'operation', tg_op,
      'sitio_id', v_sitio_id,
      'dispositivo_id', v_dispositivo_id,
      'changed_at', pg_catalog.now(),
      'id', v_fila ->> 'id',
      'registro', case
        when tg_table_name in (
          'ingresos',
          'ingresos_proveedor',
          'ingresos_correo',
          'prestamos_gafete_provisional'
        ) and tg_op <> 'DELETE' then v_fila
        else null
      end
    ),
    'cambio_nube',
    'sitio:' || v_sitio_id::text,
    true
  );

  return null;
end;
$function$;
