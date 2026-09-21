-- Hallazgo 2026-09-21: al probar realtime en un proyecto de staging nuevo
-- (creado replicando sólo las migraciones de este repo), el aviso en vivo
-- (`private.emitir_cambio_nube_sitio`, ver
-- `avisa_cambio_nube_segun_quien_escribe_no_quien_creo_la_fila.sql`) no
-- llegaba para altas/bajas de CONTRATISTAS/EMPRESAS/GAFETES/INGRESOS -- el
-- flujo central de entrada/salida, "la joya de la corona" del sistema.
--
-- Causa: estos 4 triggers existen en producción, pero se crearon en algún
-- momento fuera de una migración versionada (probablemente a mano, antes de
-- que el resto de las tablas nuevas adoptara el patrón de siempre agregar su
-- propio trigger en la misma migración que las crea -- ver
-- `avisa_cambio_nube_en_usuarios.sql`, `avisa_cambio_nube_en_cita_sitios.sql`,
-- etc.). El repo nunca los tuvo documentados, así que un proyecto
-- reconstruido desde cero desde `supabase/migrations/` (staging, o
-- producción misma en un desastre) los pierde en silencio -- ver
-- docs/recuperacion-sitio-staging.md.
--
-- `drop trigger if exists` + `create trigger` (no sólo `create`) para que
-- esta migración sea segura de aplicar también en producción, donde ya
-- existen: los deja declarados explícitamente en el historial sin duplicar
-- ni fallar por "ya existe".
drop trigger if exists contratistas_emitir_cambio_nube on public.contratistas;
create trigger contratistas_emitir_cambio_nube
after insert or update or delete on public.contratistas
for each row execute function private.emitir_cambio_nube_sitio();

drop trigger if exists empresas_emitir_cambio_nube on public.empresas;
create trigger empresas_emitir_cambio_nube
after insert or update or delete on public.empresas
for each row execute function private.emitir_cambio_nube_sitio();

drop trigger if exists gafetes_emitir_cambio_nube on public.gafetes;
create trigger gafetes_emitir_cambio_nube
after insert or update or delete on public.gafetes
for each row execute function private.emitir_cambio_nube_sitio();

drop trigger if exists ingresos_emitir_cambio_nube on public.ingresos;
create trigger ingresos_emitir_cambio_nube
after insert or update or delete on public.ingresos
for each row execute function private.emitir_cambio_nube_sitio();
