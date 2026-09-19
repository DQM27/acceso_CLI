-- Contraparte en la nube de `MIGRACION_47` del núcleo Rust
-- (`src/database/schema.rs`): `UNIQUE(nombre)` sólo bloqueaba un choque
-- exacto -- "Dos Pinos" y "DOS PINOS" se colaban como dos empresas
-- distintas. `plegar_texto` (minúsculas + sin tildes, vía la extensión
-- `unaccent`) es el equivalente Postgres de `PLEGAR`/`plegar_para_busqueda`
-- del lado Rust. Los índices `UNIQUE(nombre)` existentes se dejan intactos
-- a propósito -- este índice nuevo es una restricción adicional, no un
-- reemplazo.
create or replace function public.plegar_texto(texto text)
returns text
language sql
immutable parallel safe
as $$
  select lower(extensions.unaccent('extensions.unaccent'::regdictionary, texto));
$$;

create unique index if not exists idx_empresas_nombre_plegado
  on public.empresas (plegar_texto(nombre));

create unique index if not exists idx_empresas_proveedor_nombre_plegado
  on public.empresas_proveedor (plegar_texto(nombre));
