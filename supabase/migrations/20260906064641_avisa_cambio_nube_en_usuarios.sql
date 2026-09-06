-- A la tabla usuarios nunca se le agregó el trigger de aviso en vivo que sí
-- tienen contratistas/empresas/gafetes/ingresos (ver
-- private.emitir_cambio_nube_sitio, migración
-- avisa_cambio_nube_segun_quien_escribe_no_quien_creo_la_fila) -- una
-- activación/desactivación de usuario nunca disparaba el resync
-- instantáneo, sólo se enteraban los dispositivos en el próximo pulso
-- periódico (2 min) o sync manual.
drop trigger if exists usuarios_emitir_cambio_nube on public.usuarios;
create trigger usuarios_emitir_cambio_nube
after insert or update or delete on public.usuarios
for each row execute function private.emitir_cambio_nube_sitio();
