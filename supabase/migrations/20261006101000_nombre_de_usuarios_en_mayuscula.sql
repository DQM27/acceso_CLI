-- Todo nombre de persona o empresa en mayúscula (pedido del dueño
-- 2026-10-06; la regla vive en el núcleo, `reglas/src/nombre.rs`).
--
-- Los usuarios se crean con la Edge Function `admin-create-usuario` (que ya
-- aplica la regla del núcleo por WebAssembly) pero se editan desde el panel
-- con un `update` directo a la tabla, sin función de por medio. Este
-- trigger repite la regla en la base para los dos caminos: espacios de más
-- fuera y en mayúscula, igual que `nombre_en_mayusculas`.
--
-- Sólo cambia lo que se guarda de acá en adelante; no toca filas existentes.

create or replace function private.usuarios_nombre_en_mayuscula()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  new.nombre := upper(btrim(regexp_replace(new.nombre, '[[:space:]]+', ' ', 'g')));
  return new;
end;
$$;

revoke all on function private.usuarios_nombre_en_mayuscula() from public;

create trigger usuarios_nombre_en_mayuscula
  before insert or update of nombre on public.usuarios
  for each row execute function private.usuarios_nombre_en_mayuscula();
