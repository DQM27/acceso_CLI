-- Plantilla para poblar el catálogo (empresas/contratistas/gafetes) tras
-- correr resetear_datos_prueba.sql. Pensado para reemplazar los bloques de
-- ejemplo por datos reales del cliente cuando estén listos -- hoy sólo
-- deja la estructura correcta (columnas NOT NULL, constraints, orden de
-- inserción por FK) para no perder tiempo redescubriéndola cada vez.
--
-- NO se ejecuta solo -- pegarlo a mano en el SQL Editor de Supabase (o
-- `supabase db execute -f este-archivo`). Reemplazar los VALUES de ejemplo
-- antes de correrlo con datos que sirvan de verdad.
--
-- Requiere: la fila "Brisas" en sitios (siempre existe, nunca se borra) y
-- al menos un dispositivo -- contratistas/empresas/gafetes tienen
-- dispositivo_origen_id NOT NULL (registra qué dispositivo originó cada
-- fila). Si se corre esto recién después de resetear_datos_prueba.sql
-- (que borra todos los dispositivos), el bloque de abajo crea uno
-- "Semilla de datos (script)" para poder insertar -- no es un dispositivo
-- real, nadie se autentica con su secreto (secret_hash es basura
-- intencional), sólo sirve como origen de estas filas. Si ya provisionaste
-- un dispositivo real desde el panel antes de correr esto, usá su id en
-- vez de crear este de más (cambiar la referencia en la sección
-- "variables" de abajo).
begin;

-- === Variables (ajustar acá, se usan en todo el resto del script) ===
with variables as (
  select
    (select id from public.sitios where nombre = 'Brisas') as sitio_id
),
dispositivo_semilla as (
  insert into public.dispositivos (sitio_id, tipo, etiqueta, secret_hash)
  select sitio_id, 'pc', 'Semilla de datos (script)', 'no-usar-' || gen_random_uuid()::text
  from variables
  on conflict do nothing
  returning id, sitio_id
)
select * from dispositivo_semilla;

-- Si el insert de arriba no corrió (ya existía un dispositivo semilla de
-- una corrida anterior de este mismo script), resolvelo a mano:
--   select id from public.dispositivos where etiqueta = 'Semilla de datos (script)';
-- y reemplazá `(select id from dispositivo_semilla)` de abajo por ese uuid literal.

-- === Empresas (ejemplo -- reemplazar) ===
with sitio as (select id from public.sitios where nombre = 'Brisas'),
     dispositivo as (
       select id from public.dispositivos where etiqueta = 'Semilla de datos (script)' limit 1
     )
insert into public.empresas (sitio_id, dispositivo_origen_id, nombre, activa)
select sitio.id, dispositivo.id, valores.nombre, true
from sitio, dispositivo,
  (values
    ('Constructora Ejemplo S.A.'),
    ('Servicios Generales Ejemplo Ltda.')
  ) as valores(nombre)
on conflict (nombre) do nothing;

-- === Contratistas (ejemplo -- reemplazar) ===
-- tipo_ingreso: 'PRAIND' | 'IN_HOUSE' | 'POR_CORREO' | 'SWAT'
-- fecha_vencimiento_praind sólo aplica (y sólo tiene sentido) para PRAIND.
with sitio as (select id from public.sitios where nombre = 'Brisas'),
     dispositivo as (
       select id from public.dispositivos where etiqueta = 'Semilla de datos (script)' limit 1
     )
insert into public.contratistas (
  sitio_id, dispositivo_origen_id, nombre, identificacion, empresa_id, empresa_nombre,
  activo, tipo_ingreso, fecha_vencimiento_praind, es_personal_ruta
)
select
  sitio.id, dispositivo.id, datos.nombre, datos.identificacion, empresa.id, empresa.nombre,
  true, datos.tipo_ingreso, datos.fecha_vencimiento_praind, datos.es_personal_ruta
from sitio, dispositivo,
  (values
    ('Contratista Ejemplo Uno', '100000001', 'Constructora Ejemplo S.A.', 'PRAIND', '2027-01-01'::date, false),
    ('Contratista Ejemplo Dos', '100000002', 'Servicios Generales Ejemplo Ltda.', 'IN_HOUSE', null::date, false)
  ) as datos(nombre, identificacion, empresa_nombre, tipo_ingreso, fecha_vencimiento_praind, es_personal_ruta)
join public.empresas empresa
  on empresa.nombre = datos.empresa_nombre and empresa.sitio_id = sitio.id
on conflict (identificacion) do nothing;

-- === Gafetes (ejemplo -- reemplazar) ===
-- estado: 'DISPONIBLE' | 'PERDIDO' | 'DE_BAJA'. Un gafete PERDIDO exige
-- contratista_deudor_id (constraint de la tabla) -- no aplica a este
-- ejemplo, todos arrancan DISPONIBLE.
with sitio as (select id from public.sitios where nombre = 'Brisas'),
     dispositivo as (
       select id from public.dispositivos where etiqueta = 'Semilla de datos (script)' limit 1
     )
insert into public.gafetes (sitio_id, dispositivo_origen_id, numero, estado)
select sitio.id, dispositivo.id, numero, 'DISPONIBLE'
from sitio, dispositivo,
  (values (1), (2), (3), (4), (5)) as valores(numero)
on conflict (sitio_id, numero) do nothing;

select
  (select count(*) from public.empresas) as empresas,
  (select count(*) from public.contratistas) as contratistas,
  (select count(*) from public.gafetes) as gafetes;

-- Revisar el resultado del SELECT de arriba antes de decidir.
commit;
-- rollback;
