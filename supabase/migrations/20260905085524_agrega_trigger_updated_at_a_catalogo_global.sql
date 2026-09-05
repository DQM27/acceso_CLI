-- contratistas/empresas/usuarios tenían `updated_at DEFAULT now()` pero
-- SIN trigger que lo actualice en UPDATE (a diferencia de ingresos/gafetes,
-- que sí lo tienen -- ver ingresos_actualizar_updated_at/
-- gafetes_actualizar_updated_at). Confirmado en vivo: un UPDATE real no
-- movía `updated_at` para nada. Esto es necesario para que el sync
-- incremental de src/nube/sincronizacion.rs (filtro `updated_at=gt.…`)
-- no se pierda cambios silenciosamente.

create or replace function public.contratistas_actualizar_updated_at()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create or replace function public.empresas_actualizar_updated_at()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create or replace function public.usuarios_actualizar_updated_at()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create trigger contratistas_actualizar_updated_at
before update on public.contratistas
for each row execute function public.contratistas_actualizar_updated_at();

create trigger empresas_actualizar_updated_at
before update on public.empresas
for each row execute function public.empresas_actualizar_updated_at();

create trigger usuarios_actualizar_updated_at
before update on public.usuarios
for each row execute function public.usuarios_actualizar_updated_at();
