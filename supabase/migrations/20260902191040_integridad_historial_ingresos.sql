-- Espejo de las reglas de inmutabilidad que ya tiene registro_ingresos en
-- la base local (src/database/schema.rs) -- hasta ahora sólo las
-- respetaba nuestro propio código, no la base. Nada impide que se manden
-- estas mismas reglas más de una vez (son idempotentes: valores repetidos
-- no disparan la excepción), así que un reintento normal de "crear"
-- (upsert) sigue funcionando igual.

create or replace function ingresos_bloquear_cambios_de_entrada()
returns trigger
language plpgsql
as $$
begin
  if new.sitio_id is distinct from old.sitio_id
     or new.dispositivo_entrada_id is distinct from old.dispositivo_entrada_id
     or new.contratista_id is distinct from old.contratista_id
     or new.contratista_nombre is distinct from old.contratista_nombre
     or new.hora_entrada is distinct from old.hora_entrada
     or new.usuario_entrada_nombre is distinct from old.usuario_entrada_nombre
     or new.created_at is distinct from old.created_at
  then
    raise exception 'Los datos de entrada de un ingreso son inmutables';
  end if;
  return new;
end;
$$;

create trigger ingresos_entrada_inmutable
before update on ingresos
for each row execute function ingresos_bloquear_cambios_de_entrada();

create or replace function ingresos_bloquear_doble_cierre()
returns trigger
language plpgsql
as $$
begin
  if old.hora_salida is not null and new.hora_salida is distinct from old.hora_salida then
    raise exception 'La salida de un ingreso solo puede registrarse una vez';
  end if;
  return new;
end;
$$;

create trigger ingresos_salida_unica
before update on ingresos
for each row execute function ingresos_bloquear_doble_cierre();

-- Antes esto quedaba librado a que quien mandara el PATCH incluyera
-- updated_at a mano (no lo hacíamos) -- ahora lo pone la base siempre.
create or replace function ingresos_actualizar_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create trigger ingresos_set_updated_at
before update on ingresos
for each row execute function ingresos_actualizar_updated_at();
