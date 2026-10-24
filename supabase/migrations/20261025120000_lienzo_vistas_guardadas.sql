-- Vistas guardadas del lienzo (plan del asistente, Parte A, etapa 4): un
-- administrador del panel guarda con nombre el documento de una vista (armada
-- por el asistente o editada a mano) y la vuelve a abrir sin llamar al modelo.
--
-- Cada vista es de quien la guardó: nadie ve las de otros. Importa porque el
-- documento de una vista con fuente "consulta" lleva su SQL, que sólo debe ver
-- quien la armó. El documento se valida en el panel al abrirlo (validarLienzo);
-- acá sólo se acota el tamaño.

create table public.lienzo_vistas_guardadas (
  id uuid primary key default gen_random_uuid(),
  correo text not null,
  nombre text not null check (char_length(btrim(nombre)) between 1 and 80),
  lienzo jsonb not null check (jsonb_typeof(lienzo) = 'object' and pg_column_size(lienzo) <= 65536),
  creado_en timestamptz not null default now(),
  actualizado_en timestamptz not null default now(),
  unique (correo, nombre)
);

comment on table public.lienzo_vistas_guardadas is
  'Vistas del lienzo guardadas con nombre por cada administrador del panel. Cada uno ve y edita sólo las suyas.';

create index lienzo_vistas_guardadas_correo_idx on public.lienzo_vistas_guardadas (correo, actualizado_en desc);

alter table public.lienzo_vistas_guardadas enable row level security;

-- Sólo las propias, y sólo si sigue siendo administrador del panel.
create policy "cada admin ve sus vistas guardadas"
  on public.lienzo_vistas_guardadas for select to authenticated
  using (
    correo = (select auth.email())
    and exists (select 1 from public.administradores_panel a where a.correo = (select auth.email()))
  );
create policy "cada admin guarda vistas a su nombre"
  on public.lienzo_vistas_guardadas for insert to authenticated
  with check (
    correo = (select auth.email())
    and exists (select 1 from public.administradores_panel a where a.correo = (select auth.email()))
  );
create policy "cada admin cambia sus vistas guardadas"
  on public.lienzo_vistas_guardadas for update to authenticated
  using (correo = (select auth.email()))
  with check (
    correo = (select auth.email())
    and exists (select 1 from public.administradores_panel a where a.correo = (select auth.email()))
  );
create policy "cada admin borra sus vistas guardadas"
  on public.lienzo_vistas_guardadas for delete to authenticated
  using (correo = (select auth.email()));

-- Permisos explícitos (AGENTS.md).
revoke all on public.lienzo_vistas_guardadas from public, anon, authenticated;
grant select, insert, update, delete on public.lienzo_vistas_guardadas to authenticated;
grant select, insert, update, delete on public.lienzo_vistas_guardadas to service_role;
