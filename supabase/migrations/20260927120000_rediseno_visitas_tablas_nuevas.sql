-- Rediseño de visitas (docs/auditorias/rediseno-visitas-2026-09-27.md,
-- sección 3.1) -- primer corte de V2: sólo las tablas nuevas + RLS.
-- Las RPCs (crear_visitas/editar_visita/cancelar_visita/duplicar_visita/
-- responder_solicitud), la vista `mis_visitas` y la función de
-- "visitantes anteriores" van en una migración aparte, sobre este esquema
-- ya confirmado en staging.
--
-- Nombres sin chocar con las tablas viejas de citas (`citas`,
-- `cita_sitios`, `cita_visitantes`, `movimientos_visita`), que quedan
-- intactas a propósito -- el núcleo Rust las sigue consultando en cada
-- sincronización hasta V1/V3 (ver el handoff, "no borrar ni alterar").
-- Por eso el espejo de movimientos de ESTE rediseño se llama
-- `visita_movimientos` (orden de palabras invertido a propósito), no
-- `movimientos_visita`.
--
-- Orden: TODAS las tablas primero (varias políticas más abajo necesitan
-- nombrar tablas que todavía no existirían si se intercalara tabla+RLS
-- una por una, ej. las políticas de `visitantes` nombran `visitas` y
-- `visita_invitados`) -- mismo criterio que
-- 20260909181150_crea_control_de_visitas.sql. RLS + políticas + triggers +
-- publicación de Realtime, recién después, al final.

-- =====================================================================
-- TABLAS
-- =====================================================================

-- anfitriones: agrega `id` -- las tablas nuevas referencian anfitriones
-- por id (uuid), no por correo como hacía el esquema viejo de citas.
-- `activo` ya existe (20260909192655_endurece_esquema_de_citas_para_la_rpc_atomica).
alter table public.anfitriones add column id uuid not null default gen_random_uuid() unique;

-- La persona como entidad, con historial propio (a diferencia de
-- cita_visitantes, que es un snapshot por cita sin relación entre sí). Sin
-- sitio_id: un visitante no pertenece a un sitio, sólo sus visitas.
create table public.visitantes (
  id uuid primary key default gen_random_uuid(),
  tipo_documento text not null check (tipo_documento in ('CEDULA', 'DIMEX', 'PASAPORTE', 'OTRO')),
  numero_documento text not null,
  nombre text not null,
  empresa text,
  telefono text,
  correo text,
  creado_en timestamptz not null default now(),
  actualizado_en timestamptz not null default now(),
  unique (tipo_documento, numero_documento)
);

comment on table public.visitantes is
  'Persona real -- una fila por visitante, reconocida al volver (vs. cita_visitantes del esquema viejo, que copiaba los datos por cita sin relacionarlos entre si).';

-- Lista de restricción, administrada sólo por admin (V5 del panel); los
-- dispositivos la leen para bloquear/pedir autorización en la llegada.
create table public.restricciones_visitante (
  id uuid primary key default gen_random_uuid(),
  visitante_id uuid not null references public.visitantes(id),
  nivel text not null check (nivel in ('BLOQUEADO', 'REQUIERE_AUTORIZACION')),
  motivo text not null,
  desde timestamptz not null default now(),
  hasta timestamptz,
  creado_por text not null,
  creado_en timestamptz not null default now(),
  check (hasta is null or hasta > desde)
);

create index restricciones_visitante_visitante_idx on public.restricciones_visitante(visitante_id);

-- En qué sitios puede recibir visitas cada anfitrión. Alta/baja por SQL
-- directo por ahora (V5 del panel la gestiona), mismo criterio que
-- administradores_panel/anfitriones: sólo lectura desde acá.
create table public.anfitrion_sitios (
  anfitrion_id uuid not null references public.anfitriones(id) on delete cascade,
  sitio_id uuid not null references public.sitios(id),
  primary key (anfitrion_id, sitio_id)
);

-- Qué exige cada sitio (aceptación de reglamento, consentimiento de
-- datos, inducción) y por cuánto tiempo vale. Alta por SQL directo / V5
-- del panel; `tipo_visita` vacío ('') significa "aplica a cualquier tipo".
create table public.requisitos_sitio (
  sitio_id uuid not null references public.sitios(id),
  tipo_visita text not null default '',
  requisito text not null check (requisito in ('CONSENTIMIENTO_DATOS', 'REGLAMENTO', 'INDUCCION')),
  version_vigente int not null default 1,
  validez_dias int,
  primary key (sitio_id, tipo_visita, requisito)
);

-- La autorización, UNA por sitio (decidido: un "tour" por varios sitios
-- genera una visita por sitio, unidas por grupo_id). `estado` sólo admite
-- lo que alguien decide de verdad ('VIGENTE'/'CANCELADA'), igual que
-- `citas` -- "programada/en curso/finalizada" se deriva de sus invitados,
-- no se persiste (ver 3.2).
create table public.visitas (
  id uuid primary key default gen_random_uuid(),
  sitio_id uuid not null references public.sitios(id),
  anfitrion_id uuid not null references public.anfitriones(id),
  tipo_visita text,
  motivo text,
  fecha_desde date not null,
  fecha_hasta date not null,
  hora_desde time not null,
  hora_hasta time not null,
  requiere_escolta boolean not null default false,
  grupo_id uuid,
  origen text not null default 'PRE_REGISTRO' check (origen in ('PRE_REGISTRO', 'WALK_IN')),
  estado text not null default 'VIGENTE' check (estado in ('VIGENTE', 'CANCELADA')),
  creado_por text not null,
  creado_en timestamptz not null default now(),
  actualizado_en timestamptz not null default now(),
  check (fecha_hasta >= fecha_desde),
  check (hora_hasta > hora_desde)
);

create index visitas_sitio_idx on public.visitas(sitio_id);
create index visitas_anfitrion_idx on public.visitas(anfitrion_id);
create index visitas_grupo_idx on public.visitas(grupo_id) where grupo_id is not null;

-- Persona dentro de una visita. `estado` cubre tanto el flujo normal
-- (PROGRAMADO..FINALIZADO) como el walk-in (SOLICITADO..APROBADO/
-- RECHAZADO), ver 3.2.
create table public.visita_invitados (
  id uuid primary key default gen_random_uuid(),
  visita_id uuid not null references public.visitas(id) on delete cascade,
  visitante_id uuid not null references public.visitantes(id),
  placa_vehiculo text,
  estado text not null default 'PROGRAMADO' check (estado in (
    'PROGRAMADO', 'EN_SITIO', 'FUERA', 'FINALIZADO', 'CANCELADA', 'NO_SE_PRESENTO',
    'SOLICITADO', 'APROBADO', 'RECHAZADO'
  )),
  aprobado_por text,
  aprobado_en timestamptz,
  motivo_rechazo text,
  creado_en timestamptz not null default now(),
  unique (visita_id, visitante_id)
);

create index visita_invitados_visita_idx on public.visita_invitados(visita_id);
create index visita_invitados_visitante_idx on public.visita_invitados(visitante_id);

-- Cada cruce por el puesto de control. Mismas garantías que
-- movimientos_visita (esquema viejo): entrada inmutable, salida se
-- registra una sola vez. Sin uso real hasta V3 -- por ahora se prueba
-- insertando movimientos de prueba en staging (ver el handoff).
create table public.visita_movimientos (
  id uuid primary key default gen_random_uuid(),
  invitado_id uuid not null references public.visita_invitados(id),
  sitio_id uuid not null references public.sitios(id),
  entrada_en timestamptz not null default now(),
  salida_en timestamptz,
  gafete_numero bigint,
  gafete_devuelto boolean not null default false,
  usuario_entrada_id uuid not null references public.usuarios(id),
  dispositivo_entrada_id uuid not null references public.dispositivos(id),
  usuario_salida_id uuid references public.usuarios(id),
  dispositivo_salida_id uuid references public.dispositivos(id),
  creado_en timestamptz not null default now(),
  actualizado_en timestamptz not null default now(),
  check (salida_en is null or salida_en >= entrada_en)
);

create index visita_movimientos_sitio_idx on public.visita_movimientos(sitio_id);
create index visita_movimientos_invitado_idx on public.visita_movimientos(invitado_id);

-- Requisitos cumplidos por un visitante, con versión (Ley 8968:
-- consentimiento informado). Se insertan desde una RPC/Edge Function
-- todavía no escrita en este corte; por ahora sólo tabla + RLS.
create table public.aceptaciones (
  id uuid primary key default gen_random_uuid(),
  visitante_id uuid not null references public.visitantes(id),
  requisito text not null check (requisito in ('CONSENTIMIENTO_DATOS', 'REGLAMENTO', 'INDUCCION')),
  version int not null,
  aceptado_en timestamptz not null default now(),
  sitio_id uuid not null references public.sitios(id),
  vence_en timestamptz
);

create index aceptaciones_visitante_idx on public.aceptaciones(visitante_id);

-- =====================================================================
-- FUNCIONES DE TRIGGER (updated_at / inmutabilidad)
-- =====================================================================

create function public.visitantes_actualizar_actualizado_en()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  new.actualizado_en = now();
  return new;
end;
$$;

create function public.visitas_actualizar_actualizado_en()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  new.actualizado_en = now();
  return new;
end;
$$;

create function public.visita_movimientos_actualizar_actualizado_en()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  new.actualizado_en = now();
  return new;
end;
$$;

-- Mismas dos garantías que movimientos_visita: entrada inmutable, salida
-- se registra una sola vez (sin bloquear cambios de gafete_devuelto tras
-- la salida, a propósito -- devolver el gafete puede confirmarse después).
create function public.visita_movimientos_bloquear_cambios_de_entrada()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  if new.invitado_id is distinct from old.invitado_id
     or new.sitio_id is distinct from old.sitio_id
     or new.entrada_en is distinct from old.entrada_en
     or new.usuario_entrada_id is distinct from old.usuario_entrada_id
     or new.dispositivo_entrada_id is distinct from old.dispositivo_entrada_id
     or new.creado_en is distinct from old.creado_en
  then
    raise exception 'Los datos de entrada de un movimiento de visita son inmutables';
  end if;
  return new;
end;
$$;

create function public.visita_movimientos_bloquear_doble_cierre()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  if old.salida_en is not null and new.salida_en is distinct from old.salida_en then
    raise exception 'La salida de un movimiento de visita solo puede registrarse una vez';
  end if;
  return new;
end;
$$;

create trigger visitantes_set_actualizado_en
  before update on public.visitantes
  for each row execute function public.visitantes_actualizar_actualizado_en();

create trigger visitas_set_actualizado_en
  before update on public.visitas
  for each row execute function public.visitas_actualizar_actualizado_en();

create trigger visita_movimientos_set_actualizado_en
  before update on public.visita_movimientos
  for each row execute function public.visita_movimientos_actualizar_actualizado_en();

create trigger visita_movimientos_entrada_inmutable
  before update on public.visita_movimientos
  for each row execute function public.visita_movimientos_bloquear_cambios_de_entrada();

create trigger visita_movimientos_salida_unica
  before update on public.visita_movimientos
  for each row execute function public.visita_movimientos_bloquear_doble_cierre();

-- =====================================================================
-- RLS + POLÍTICAS
-- =====================================================================

alter table public.visitantes enable row level security;

-- Lectura: admin, un dispositivo del sitio de alguna de sus visitas, o el
-- anfitrión que lo invitó alguna vez -- mismo patrón que
-- "leer visitantes de citas propias, del sitio, o admin" del esquema
-- viejo, adaptado a que ahora el visitante no cuelga directo de una cita
-- sino de un invitado dentro de una visita.
create policy "leer visitantes propios, del sitio, o admin"
  on public.visitantes for select to authenticated
  using (
    private.es_admin_global()
    or exists (
      select 1
      from public.visita_invitados vi
      join public.visitas v on v.id = vi.visita_id
      where vi.visitante_id = visitantes.id
        and (
          v.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
          or v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email())
        )
    )
  );

-- Cualquier anfitrión activo puede registrar una persona nueva (todavía
-- sin ninguna visita asociada) -- crear un visitante no expone datos de
-- nadie, sólo los declara.
create policy "anfitrion activo crea visitantes"
  on public.visitantes for insert to authenticated
  with check (
    exists (select 1 from public.anfitriones a where a.correo = auth.email() and a.activo)
  );

-- Corregir datos: el anfitrión que lo invitó, o admin (V5 del panel) --
-- sin `delete`, mismo criterio que el resto de la app: una persona no se
-- borra, se corrige.
create policy "anfitrion o admin corrige visitantes que invito"
  on public.visitantes for update to authenticated
  using (
    private.es_admin_global()
    or exists (
      select 1
      from public.visita_invitados vi
      join public.visitas v on v.id = vi.visita_id
      where vi.visitante_id = visitantes.id
        and v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email())
    )
  )
  with check (
    private.es_admin_global()
    or exists (
      select 1
      from public.visita_invitados vi
      join public.visitas v on v.id = vi.visita_id
      where vi.visitante_id = visitantes.id
        and v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email())
    )
  );

alter table public.restricciones_visitante enable row level security;

create policy "leer restricciones (dispositivo o admin)"
  on public.restricciones_visitante for select to authenticated
  using (
    private.es_admin_global()
    or ((select auth.jwt()) ->> 'sitio_id') is not null
  );

create policy "solo admin escribe restricciones"
  on public.restricciones_visitante for insert to authenticated
  with check (private.es_admin_global());

create policy "solo admin actualiza restricciones"
  on public.restricciones_visitante for update to authenticated
  using (private.es_admin_global())
  with check (private.es_admin_global());

alter table public.anfitrion_sitios enable row level security;

create policy "leer anfitrion_sitios propios o admin"
  on public.anfitrion_sitios for select to authenticated
  using (
    private.es_admin_global()
    or anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email())
  );

alter table public.requisitos_sitio enable row level security;

create policy "leer requisitos_sitio (anfitrion con acceso, dispositivo, o admin)"
  on public.requisitos_sitio for select to authenticated
  using (
    private.es_admin_global()
    or sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or exists (
      select 1 from public.anfitrion_sitios ans
      join public.anfitriones a on a.id = ans.anfitrion_id
      where a.correo = auth.email() and ans.sitio_id = requisitos_sitio.sitio_id
    )
  );

create policy "solo admin escribe requisitos_sitio"
  on public.requisitos_sitio for insert to authenticated
  with check (private.es_admin_global());

create policy "solo admin actualiza requisitos_sitio"
  on public.requisitos_sitio for update to authenticated
  using (private.es_admin_global())
  with check (private.es_admin_global());

alter table public.visitas enable row level security;

create policy "leer visitas propias, del sitio, o admin"
  on public.visitas for select to authenticated
  using (
    private.es_admin_global()
    or sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email())
  );

create policy "anfitrion crea sus propias visitas"
  on public.visitas for insert to authenticated
  with check (anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email()));

-- Cubre editar (mientras nadie llegó, regla de negocio en la RPC, no acá)
-- y cancelar -- sin `delete`, una visita se cancela, no se borra.
create policy "anfitrion actualiza sus propias visitas"
  on public.visitas for update to authenticated
  using (anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email()))
  with check (anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email()));

alter table public.visita_invitados enable row level security;

create policy "leer visita_invitados de visitas propias, del sitio, o admin"
  on public.visita_invitados for select to authenticated
  using (
    private.es_admin_global()
    or exists (
      select 1 from public.visitas v
      where v.id = visita_invitados.visita_id
        and (
          v.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
          or v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email())
        )
    )
  );

-- Anfitrión agrega invitados a sus propias visitas (pre-registro); un
-- dispositivo del sitio los agrega cuando registra un walk-in sin cita
-- (V3, todavía no ejercido desde ninguna app).
create policy "anfitrion o dispositivo agrega invitados"
  on public.visita_invitados for insert to authenticated
  with check (
    exists (
      select 1 from public.visitas v
      where v.id = visita_invitados.visita_id
        and (
          v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email())
          or (
            v.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
            and ((select auth.jwt()) ->> 'tipo') <> 'visor'
          )
        )
    )
  );

-- Anfitrión aprueba/rechaza walk-ins y edita su propia visita; dispositivo
-- del sitio marca EN_SITIO/FUERA/FINALIZADO/NO_SE_PRESENTO al hacer
-- check-in/check-out (V3).
create policy "anfitrion o dispositivo actualiza invitados"
  on public.visita_invitados for update to authenticated
  using (
    exists (
      select 1 from public.visitas v
      where v.id = visita_invitados.visita_id
        and (
          v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email())
          or v.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
        )
    )
  )
  with check (
    exists (
      select 1 from public.visitas v
      where v.id = visita_invitados.visita_id
        and (
          v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email())
          or (
            v.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
            and ((select auth.jwt()) ->> 'tipo') <> 'visor'
          )
        )
    )
  );

alter table public.visita_movimientos enable row level security;

create policy "crear visita_movimientos del propio sitio"
  on public.visita_movimientos for insert to authenticated
  with check (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create policy "leer visita_movimientos del propio sitio, del anfitrion, o admin"
  on public.visita_movimientos for select to authenticated
  using (
    private.es_admin_global()
    or sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or exists (
      select 1
      from public.visita_invitados vi
      join public.visitas v on v.id = vi.visita_id
      where vi.id = visita_movimientos.invitado_id
        and v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = auth.email())
    )
  );

create policy "actualizar visita_movimientos del propio sitio"
  on public.visita_movimientos for update to authenticated
  using (sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid)
  with check (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

alter table public.aceptaciones enable row level security;

create policy "leer aceptaciones del propio sitio o admin"
  on public.aceptaciones for select to authenticated
  using (
    private.es_admin_global()
    or sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
  );

create policy "crear aceptaciones del propio sitio"
  on public.aceptaciones for insert to authenticated
  with check (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

-- =====================================================================
-- Realtime: la web (postgres_changes) necesita escuchar visitas/
-- visita_invitados/visita_movimientos para el estado en vivo de 3.8
-- ("Juan Pérez llegó 9:12") -- las tablas viejas de citas nunca se
-- agregaron a esta publicación (por eso MisCitas.tsx no tiene estado en
-- vivo hoy).
-- =====================================================================
alter publication supabase_realtime add table
  public.visitas, public.visita_invitados, public.visita_movimientos;
