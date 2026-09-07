-- Borra TODOS los datos de catálogo/movimiento (dispositivos, usuarios,
-- contratistas, empresas, gafetes, ingresos) -- sólo datos, nada de
-- estructura/RLS/triggers/funciones, eso vive en supabase/migrations/ y
-- este script no lo toca.
--
-- Se conserva a propósito:
--   - public.sitios: la fila "Brisas" (es el sitio real, no hay más).
--   - public.administradores_panel: SOLO daniel.bleach1@gmail.com.
--
-- Uso pensado: correr esto muchas veces mientras el proyecto sigue en
-- desarrollo. Después de esto, contratistas/empresas se repueblan con
-- `cargo run --example importar_catalogo_limpio` contra la base local
-- (ver supabase/scripts/poblar_catalogo.sql para el detalle) -- NO con
-- INSERTs de Postgres a mano. NO se ejecuta solo -- pegarlo a mano en el
-- SQL Editor de Supabase (o `supabase db execute -f este-archivo`) cuando
-- se decida de verdad borrar.
--
-- Envuelto en una transacción: si algo falla a mitad de camino, no queda
-- la base a medio borrar.
begin;

-- Orden por dependencias de FK: primero lo que depende de todo lo demás.
delete from public.ingresos;
delete from public.gafetes;
delete from public.usuarios;
delete from public.contratistas;
delete from public.empresas;
delete from public.dispositivos;

-- sitios NO se toca -- "Brisas" es el único y es real.

-- administradores_panel: sólo queda daniel.bleach1@gmail.com. Esto dispara
-- sync-access-policy (trigger trg_sync_access_policy) y actualiza la
-- política de Cloudflare Access sola.
delete from public.administradores_panel
where correo <> 'daniel.bleach1@gmail.com';

-- El ROOT real de referencia, para poder iniciar sesión apenas un
-- dispositivo pegue su secreto y sincronice (ver PrimerArranque.tsx /
-- PantallaPrimerArranque.kt). Requiere la migración
-- permite_root_en_usuarios_globales ya aplicada (rol ROOT permitido).
insert into public.usuarios (sitio_id, cedula, nombre, rol, activo)
select id, '155824395105', 'Daniel Quintana', 'ROOT', true
from public.sitios
where nombre = 'Brisas';

select
  (select count(*) from public.dispositivos) as dispositivos,
  (select count(*) from public.usuarios) as usuarios,
  (select count(*) from public.contratistas) as contratistas,
  (select count(*) from public.empresas) as empresas,
  (select count(*) from public.gafetes) as gafetes,
  (select count(*) from public.ingresos) as ingresos,
  (select count(*) from public.administradores_panel) as administradores;

-- Revisar el resultado del SELECT de arriba antes de decidir. Si está
-- bien, cambiar a `commit;`. Si algo se ve mal, `rollback;` y no se aplica
-- nada de lo anterior.
commit;
-- rollback;
