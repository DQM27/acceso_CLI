alter table public.contratistas
  add column tipo_ingreso text check (tipo_ingreso in ('PRAIND','IN_HOUSE','POR_CORREO','SWAT')),
  add column fecha_vencimiento_praind date,
  add column es_personal_ruta boolean;
