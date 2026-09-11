-- Control de visitas (docs/plan-control-visitas.md) -- espejo en la nube
-- del esquema local ya aplicado (MIGRACION_28/29 en src/database/schema.rs).
--
-- Cuatro tablas nuevas, no tres como el lado local: acá SÍ hace falta
-- `cita_sitios` (el puente muchos-a-muchos que permite una cita "tour" con
-- varios sitios) -- la nube es la que sirve a TODOS los sitios a la vez, y
-- necesita saber cuáles aplican para filtrar qué le manda a cada
-- dispositivo. El dispositivo local sólo ve su propio recorte ya filtrado,
-- por eso no necesita guardar el puente completo (ver el comentario de
-- MIGRACION_28).
--
-- `movimientos_visita` (el cruce real en el punto de acceso) queda
-- explícitamente FUERA de esta migración -- vive local en cada
-- dispositivo y todavía no tiene mecanismo de sincronización (próximo
-- corte, nube::sincronizacion.rs).
--
-- Orden: las cuatro tablas primero (una política de `citas` necesita
-- poder nombrar `cita_sitios` en un `EXISTS`, así que esa tabla ya tiene
-- que existir) y recién después RLS + políticas para todas, al final.

-- Quien puede agendar visitas en el subdominio nuevo -- distinto de
-- `administradores_panel` (panel completo) y de `usuarios` (guardias/admins
-- de sitio, login offline). Mismo molde que `administradores_panel`: el
-- login (Google/magic link) sólo confirma identidad, esta tabla decide
-- autorización real. Alta/baja queda fuera de esta migración a propósito,
-- administrada por fuera (dashboard/SQL directo) hasta que exista una
-- pantalla para eso -- mismo criterio que administradores_panel.
create table public.anfitriones (
  correo text primary key,
  nombre text not null,
  creado_en timestamptz not null default now()
);

comment on table public.anfitriones is
  'Quien puede agendar visitas en el subdominio nuevo (distinto de administradores_panel y de usuarios de sitio). El login con Google/magic link solo confirma identidad -- esta tabla decide autorizacion real, mismo patron que administradores_panel.';

-- La autorización con vigencia -- NO es un movimiento, ver el
-- doc-comment de MIGRACION_28 del lado local sobre por qué está separada de
-- `cita_visitantes`/`movimientos_visita`. `estado` sólo admite lo que
-- alguien decide de verdad ('VIGENTE'/'CANCELADA') -- 'VENCIDA' se calcula
-- comparando `fecha_hasta` contra hoy en el momento de la consulta, no se
-- persiste (mismo criterio que el lado local).
create table public.citas (
  id uuid primary key default gen_random_uuid(),
  anfitrion_correo text not null references public.anfitriones(correo),
  motivo text,
  fecha_desde date not null,
  fecha_hasta date not null,
  estado text not null default 'VIGENTE' check (estado in ('VIGENTE', 'CANCELADA')),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  check (fecha_hasta >= fecha_desde)
);

create index idx_citas_anfitrion on public.citas(anfitrion_correo);

create function public.citas_actualizar_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create trigger citas_set_updated_at
before update on public.citas
for each row execute function public.citas_actualizar_updated_at();

-- El puente muchos-a-muchos cita<->sitio (caso "tour"). `on delete cascade`
-- de citas: si una cita se cancela no hace falta borrar sus sitios, pero si
-- alguna vez se corrige una cita mal cargada, sus filas de sitio no quedan
-- huérfanas.
create table public.cita_sitios (
  cita_id uuid not null references public.citas(id) on delete cascade,
  sitio_id uuid not null references public.sitios(id),
  primary key (cita_id, sitio_id)
);

-- Una fila por persona del grupo -- una cita puede agendarse para varios
-- visitantes a la vez (ver doc-comment de MIGRACION_28 del lado local).
create table public.cita_visitantes (
  id uuid primary key default gen_random_uuid(),
  cita_id uuid not null references public.citas(id) on delete cascade,
  cedula text not null,
  nombre text not null,
  empresa text,
  placa_vehiculo text,
  created_at timestamptz not null default now()
);

create index idx_cita_visitantes_cita on public.cita_visitantes(cita_id);
create index idx_cita_visitantes_cedula on public.cita_visitantes(cedula);

-- RLS + políticas, ahora que las cuatro tablas ya existen.

alter table public.anfitriones enable row level security;

create policy "cada anfitrion lee su propia fila"
  on public.anfitriones
  for select
  to authenticated
  using (auth.email() = correo);

alter table public.citas enable row level security;

-- Tres audiencias distintas pueden leer una cita: el anfitrión que la creó,
-- un dispositivo de un sitio que la cita incluye (via cita_sitios), o un
-- admin global -- mismo criterio que ya usa "leer ingresos del propio
-- sitio" (sitio_id = mi sitio OR es_admin_global()), extendido acá porque
-- el sitio no es una columna directa sino que pasa por el puente.
create policy "leer citas propias, del sitio, o admin"
  on public.citas
  for select
  to authenticated
  using (
    anfitrion_correo = auth.email()
    or public.es_admin_global()
    or exists (
      select 1 from public.cita_sitios cs
      where cs.cita_id = citas.id
        and cs.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    )
  );

create policy "anfitrion crea sus propias citas"
  on public.citas
  for insert
  to authenticated
  with check (anfitrion_correo = auth.email());

-- Cubre cancelar (estado -> CANCELADA) y cualquier otra edición futura --
-- sin `delete` a propósito, mismo criterio que "los movimientos de acceso
-- no se pueden eliminar" del lado local: una cita se cancela, no se borra.
create policy "anfitrion actualiza sus propias citas"
  on public.citas
  for update
  to authenticated
  using (anfitrion_correo = auth.email())
  with check (anfitrion_correo = auth.email());

alter table public.cita_sitios enable row level security;

create policy "leer cita_sitios propios, del sitio, o admin"
  on public.cita_sitios
  for select
  to authenticated
  using (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or public.es_admin_global()
    or exists (
      select 1 from public.citas c
      where c.id = cita_sitios.cita_id and c.anfitrion_correo = auth.email()
    )
  );

create policy "anfitrion agrega sitios a sus propias citas"
  on public.cita_sitios
  for insert
  to authenticated
  with check (
    exists (
      select 1 from public.citas c
      where c.id = cita_sitios.cita_id and c.anfitrion_correo = auth.email()
    )
  );

create policy "anfitrion quita sitios de sus propias citas"
  on public.cita_sitios
  for delete
  to authenticated
  using (
    exists (
      select 1 from public.citas c
      where c.id = cita_sitios.cita_id and c.anfitrion_correo = auth.email()
    )
  );

alter table public.cita_visitantes enable row level security;

create policy "leer visitantes de citas propias, del sitio, o admin"
  on public.cita_visitantes
  for select
  to authenticated
  using (
    public.es_admin_global()
    or exists (
      select 1 from public.citas c
      where c.id = cita_visitantes.cita_id and c.anfitrion_correo = auth.email()
    )
    or exists (
      select 1 from public.cita_sitios cs
      where cs.cita_id = cita_visitantes.cita_id
        and cs.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    )
  );

-- `insert, update, delete` (no `select`, ya no hace falta: ya lo cubre la
-- política de lectura de arriba, que agregar acá también sólo duplicaría
-- sin cambiar el resultado).
create policy "anfitrion agrega visitantes a sus propias citas"
  on public.cita_visitantes
  for insert
  to authenticated
  with check (
    exists (
      select 1 from public.citas c
      where c.id = cita_visitantes.cita_id and c.anfitrion_correo = auth.email()
    )
  );

create policy "anfitrion edita visitantes de sus propias citas"
  on public.cita_visitantes
  for update
  to authenticated
  using (
    exists (
      select 1 from public.citas c
      where c.id = cita_visitantes.cita_id and c.anfitrion_correo = auth.email()
    )
  )
  with check (
    exists (
      select 1 from public.citas c
      where c.id = cita_visitantes.cita_id and c.anfitrion_correo = auth.email()
    )
  );

create policy "anfitrion quita visitantes de sus propias citas"
  on public.cita_visitantes
  for delete
  to authenticated
  using (
    exists (
      select 1 from public.citas c
      where c.id = cita_visitantes.cita_id and c.anfitrion_correo = auth.email()
    )
  );
