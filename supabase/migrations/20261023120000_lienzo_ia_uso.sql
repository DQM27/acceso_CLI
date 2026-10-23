-- Registro de cada consulta al asistente del lienzo (plan del asistente,
-- Parte A, etapa 2): auditoría, tope diario de uso y costo real por consulta.
--
-- Lo escribe sólo la Edge Function `lienzo-ia` (con `service_role`), después
-- de cada pedido; nadie lo escribe desde el panel. Cada administrador del panel
-- puede leer sus propias filas. No guarda filas de datos ni cédulas: la
-- pregunta, el documento del lienzo resultante (filtros y gráficos) y la
-- respuesta en texto.

create table public.lienzo_ia_uso (
  id uuid primary key default gen_random_uuid(),
  creado_en timestamptz not null default now(),
  correo text not null,
  pregunta text not null check (char_length(pregunta) between 1 and 2000),
  -- editado: el modelo cambió el lienzo; respondido: sólo texto (pregunta o aclaración);
  -- rechazado: lo que devolvió no pasó el validador ni con el reintento;
  -- tope: no se llamó al modelo porque se llegó al tope diario; error: falló la llamada.
  resultado text not null check (resultado in ('editado', 'respondido', 'rechazado', 'tope', 'error')),
  lienzo jsonb,
  respuesta text check (char_length(respuesta) <= 4000),
  modelo text,
  tokens_entrada integer not null default 0 check (tokens_entrada >= 0),
  tokens_salida integer not null default 0 check (tokens_salida >= 0),
  tokens_cache integer not null default 0 check (tokens_cache >= 0),
  costo_usd numeric(10, 6) not null default 0 check (costo_usd >= 0),
  reintentos smallint not null default 0 check (reintentos between 0 and 1),
  duracion_ms integer check (duracion_ms >= 0),
  error text check (char_length(error) <= 2000)
);

comment on table public.lienzo_ia_uso is
  'Una fila por consulta al asistente del lienzo (Edge Function lienzo-ia): auditoría, tope diario y costo.';

-- Tope diario por persona y global: cuentan las filas de hoy.
create index lienzo_ia_uso_correo_creado_idx on public.lienzo_ia_uso (correo, creado_en desc);
create index lienzo_ia_uso_creado_idx on public.lienzo_ia_uso (creado_en desc);

alter table public.lienzo_ia_uso enable row level security;

-- Cada administrador del panel lee sólo lo suyo.
create policy "cada admin lee sus consultas al asistente"
  on public.lienzo_ia_uso for select to authenticated
  using (
    correo = (select auth.email())
    and exists (select 1 from public.administradores_panel a where a.correo = (select auth.email()))
  );

-- Permisos explícitos (AGENTS.md): el panel sólo lee; la Edge Function inserta.
revoke all on public.lienzo_ia_uso from public, anon, authenticated;
grant select on public.lienzo_ia_uso to authenticated;
grant select, insert on public.lienzo_ia_uso to service_role;
