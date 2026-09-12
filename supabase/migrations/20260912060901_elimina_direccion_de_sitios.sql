-- La columna `direccion` de `sitios` se llenaba opcionalmente al crear un
-- sitio desde el panel (web/src/pantallas/Dispositivos.tsx), pero nunca se
-- mostraba ni editaba después -- write-only, dato inaccesible una vez
-- guardado. Las dos filas existentes ya la tenían en NULL. No se usa en
-- ningún otro lado del repo (núcleo Rust, desktop, mobile).
alter table public.sitios drop column direccion;
