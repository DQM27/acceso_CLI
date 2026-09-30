-- Bloqueo cruzado de gafete en proveedores y gafetes provisionales KOF, igual
-- que ya existía para contratistas (`ingresos_gafete_activo_sitio_idx`).
--
-- Antes de registrar, cada equipo pregunta a la nube si el gafete está en uso
-- en otro equipo de la misma unidad (`gafete_de_proveedor_ocupado_en_otro_dispositivo`,
-- `gafete_provisional_ocupado_en_otro_dispositivo`). Eso no cubre dos equipos
-- que lo entregan casi al mismo tiempo ni un equipo que registró sin
-- conexión. Estos índices son la garantía final: el segundo en llegar a la
-- nube es rechazado, la cola de salida lo marca como conflicto
-- (`es_conflicto_gafete_activo`, por el nombre del índice) y la app avisa al
-- operador que ese movimiento no quedó registrado.
--
-- En producción y en staging no hay gafetes activos repetidos.

create unique index if not exists ingresos_proveedor_gafete_activo_sitio_idx
  on public.ingresos_proveedor (sitio_id, gafete_numero)
  where hora_salida is null;

create unique index if not exists prestamos_gafete_provisional_gafete_activo_sitio_idx
  on public.prestamos_gafete_provisional (sitio_id, gafete_numero)
  where hora_devolucion is null;

-- Sobra: en proveedores `gafete_numero` es obligatorio, así que el índice
-- nuevo cubre exactamente las mismas filas ("activos por unidad").
drop index if exists public.ingresos_proveedor_activos_idx;
