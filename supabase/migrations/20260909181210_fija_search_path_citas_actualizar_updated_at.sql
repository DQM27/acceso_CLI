-- El advisor de seguridad de Supabase marcó citas_actualizar_updated_at
-- (migración anterior, crea_control_de_visitas) sin search_path fijo --
-- las demás funciones *_actualizar_updated_at (ingresos/gafetes/usuarios/
-- contratistas/empresas) ya lo tienen en 'public'. Corrige la
-- inconsistencia (verificado con mcp__supabase__get_advisors antes y
-- después: el hallazgo desaparece).
alter function public.citas_actualizar_updated_at() set search_path = public;
