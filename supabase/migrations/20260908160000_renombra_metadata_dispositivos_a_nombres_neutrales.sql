-- Las columnas de metadata forense (migracion 20260908150100) nacieron con
-- nombres pensados solo para Android (android_id, modelo, fabricante,
-- fingerprint) -- ahora que escritorio tambien las llena (con Machine GUID
-- de Windows, nombre de PC, sistema operativo), esos nombres quedaban
-- enganosos. Renombradas a algo neutral que sirve para cualquier
-- plataforma; los datos ya guardados se conservan (RENAME COLUMN no los
-- toca).
alter table public.dispositivos rename column android_id to identificador_hardware;
alter table public.dispositivos rename column modelo to nombre_dispositivo;
alter table public.dispositivos rename column fabricante to plataforma;
alter table public.dispositivos rename column fingerprint to version_build;
