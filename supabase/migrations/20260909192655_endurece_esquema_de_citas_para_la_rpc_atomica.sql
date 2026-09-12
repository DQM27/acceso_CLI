-- Soporte de esquema para `crear_cita_anfitrion` (docs/contrato-web-visitas.md)
-- y defensa en profundidad para el camino de escritura directa REST, que
-- las políticas de citas/cita_sitios/cita_visitantes siguen permitiendo.

-- Baja lógica de anfitriones -- la FK `citas.anfitrion_correo` impide
-- borrar físicamente una fila con historial (ver hallazgo del reporte de
-- seguridad). `activo` se revisa en vivo en cada operación (RLS y la RPC
-- vuelven a leer esta tabla en cada llamada, nunca cachean "autorizado"),
-- así que revocar afecta de inmediato incluso a un token de Supabase Auth
-- todavía vigente -- no hace falta invalidar el JWT en sí.
alter table public.anfitriones add column activo boolean not null default true;

drop policy "cada anfitrion lee su propia fila" on public.anfitriones;
create policy "cada anfitrion lee su propia fila"
  on public.anfitriones
  for select
  to authenticated
  using (auth.email() = correo and activo);

-- Clave de idempotencia de `crear_cita_anfitrion`: un reintento con el
-- mismo `p_id` compara este hash contra el contenido normalizado ya
-- guardado antes de decidir si es el mismo pedido (devuelve el mismo id)
-- o uno distinto que choca con un id ya usado (se rechaza). `text`, no
-- `bytea`: `md5()` ya devuelve hex, no hace falta la extensión pgcrypto.
alter table public.citas add column contenido_hash text;

alter table public.citas
  add constraint citas_motivo_razonable
  check (motivo is null or (length(motivo) <= 1000 and motivo !~ '[[:cntrl:]]'));

alter table public.cita_visitantes
  add constraint cita_visitantes_cedula_razonable
  check (length(cedula) between 1 and 30 and cedula !~ '[[:cntrl:]]'),
  add constraint cita_visitantes_nombre_razonable
  check (length(nombre) between 1 and 150 and nombre !~ '[[:cntrl:]]'),
  add constraint cita_visitantes_empresa_razonable
  check (empresa is null or (length(empresa) <= 150 and empresa !~ '[[:cntrl:]]')),
  add constraint cita_visitantes_placa_razonable
  check (placa_vehiculo is null or (length(placa_vehiculo) <= 20 and placa_vehiculo !~ '[[:cntrl:]]'));

-- Una cita cancelada es un estado terminal (mismo criterio que "los
-- movimientos de acceso no se eliminan, se cierran"): sin esto, la
-- política UPDATE ya permitía a un anfitrión reactivar su propia cita
-- cancelada con un PATCH directo, aunque la web nunca ofrezca ese botón.
create or replace function public.citas_bloquea_reactivacion()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  if old.estado = 'CANCELADA' and new.estado is distinct from 'CANCELADA' then
    raise exception 'No se puede reactivar una cita cancelada';
  end if;
  return new;
end;
$$;

create trigger citas_bloquea_reactivacion
  before update on public.citas
  for each row execute function public.citas_bloquea_reactivacion();
