-- Espejo remoto de `MIGRACION_49` en el núcleo Rust (`src/database/schema.rs`):
-- ahora se captura la placa del vehículo cuando un contratista entra con
-- `medio_ingreso = 'VEHICULO'`. La tabla local `registro_ingresos` protege
-- la correspondencia con un CHECK cruzado contra `medio_ingreso`; acá, igual
-- que ya pasa con `medio_ingreso`/`gafete_numero` (ver la migración
-- `agrega_columnas_historial_a_ingresos`), se deja simplemente nullable --
-- este espejo es de sólo lectura para el panel web y el sync entre
-- dispositivos, la validación real (placa obligatoria en Vehículo, prohibida
-- en Caminando) vive en `RegistroIngresoService::registrar_entrada`, del
-- lado del dispositivo que registra el ingreso.
alter table public.ingresos
  add column placa text;
