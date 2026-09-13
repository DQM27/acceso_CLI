-- Una cita creada en la web (RPC `crear_cita_anfitrion`) inserta filas en
-- `cita_sitios` en la misma transacción, pero esa tabla nunca se enganchó
-- al aviso en vivo (`private.emitir_cambio_nube_sitio`, ver migración
-- `avisa_cambio_nube_segun_quien_escribe_no_quien_creo_la_fila`) que ya
-- tienen `usuarios` y `movimientos_visita` -- hoy una cita nueva sólo le
-- llega al dispositivo por el pulso periódico de sync (~2 min), nunca en
-- vivo. `cita_sitios` es la única tabla de la familia de citas con
-- `sitio_id` propio por fila (`citas` es agnóstica de sitio, sólo se
-- filtra vía RLS de `cita_sitios`), así que es la tabla correcta para
-- enganchar el trigger -- una fila por sitio incluido en la cita, que es
-- justo el evento que le importa a un dispositivo de ese sitio.
drop trigger if exists cita_sitios_emitir_cambio_nube on public.cita_sitios;
create trigger cita_sitios_emitir_cambio_nube
after insert or update or delete on public.cita_sitios
for each row execute function private.emitir_cambio_nube_sitio();
