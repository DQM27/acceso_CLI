-- Medio de ingreso de las visitas agendadas (pedido del dueño 2026-10-05):
-- igual que el ingreso de contratistas y el "por correo", la portería anota
-- si entró en vehículo (la placa) o caminando (NULL). La placa que escribió
-- el anfitrión en la cita (`cita_visitantes.placa_vehiculo`) es sólo la
-- sugerencia; ésta es la que se vio en la portería.
--
-- ORDEN DE DESPLIEGUE: antes que la app de escritorio que la manda. La app
-- nueva incluye `placa` al subir el movimiento y al bajar el historial: sin
-- la columna, la nube rechazaría las dos cosas. Las apps viejas no la
-- mandan ni la piden, así que esta migración no las afecta.

alter table public.movimientos_visita add column if not exists placa text;

-- Mismo límite que la placa del resto del sistema.
alter table public.movimientos_visita
  drop constraint if exists movimientos_visita_placa_valida;
alter table public.movimientos_visita
  add constraint movimientos_visita_placa_valida
  check (placa is null or (length(placa) between 1 and 20 and placa !~ '[[:cntrl:]]'));

-- La placa es dato de entrada: inmutable como los demás.
create or replace function public.movimientos_visita_bloquear_cambios_de_entrada()
returns trigger
language plpgsql
set search_path to 'public'
as $$
begin
  if new.sitio_id is distinct from old.sitio_id
     or new.dispositivo_entrada_id is distinct from old.dispositivo_entrada_id
     or new.cita_visitante_id is distinct from old.cita_visitante_id
     or new.visitante_cedula is distinct from old.visitante_cedula
     or new.visitante_nombre is distinct from old.visitante_nombre
     or new.gafete_numero is distinct from old.gafete_numero
     or new.placa is distinct from old.placa
     or new.hora_entrada is distinct from old.hora_entrada
     or new.usuario_entrada_nombre is distinct from old.usuario_entrada_nombre
     or new.created_at is distinct from old.created_at
  then
    raise exception 'Los datos de entrada de un movimiento de visita son inmutables';
  end if;
  return new;
end;
$$;
