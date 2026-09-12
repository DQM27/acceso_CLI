-- Seguridad (hallazgo crítico, auditoría web 2026-09-06): las políticas
-- "(global)" de contratistas/usuarios/empresas (`to authenticated using
-- (true)`) se escribieron pensando en "cualquier DISPOSITIVO autenticado"
-- (móvil/escritorio de cualquier sitio, para que el catálogo global se
-- sincronice sin importar el sitio de origen -- ver
-- crea_usuarios_globales.sql y globaliza_contratistas_y_empresas.sql).
-- Nunca se pensaron para "cualquier cuenta de Google que complete el login
-- OAuth del panel web" -- pero `to authenticated` no distingue una cosa de
-- la otra: el panel web autentica con Supabase Auth ANTES de chequear si
-- el correo está en administradores_panel (ver AuthContexto.tsx), así que
-- cualquier cuenta de Google, esté o no autorizada para el panel, obtiene
-- la MISMA sesión `authenticated` que un dispositivo legítimo.
--
-- Resultado antes de este fix: cualquiera con una cuenta de Google podía
-- autenticarse contra este proyecto de Supabase (sin pasar nunca por la
-- UI del panel ni por administradores_panel) y, con ese JWT, leer la tabla
-- completa de usuarios (cédulas, nombres, roles de todos los sitios) y
-- escribir cualquier fila -- incluido usuarios.rol, lo que permite
-- promoverse a ROOT y, combinado con SIN_PASSWORD_LOCAL (el hash de
-- contraseña se fija en el primer login local de esa cédula en CUALQUIER
-- dispositivo), tomar control de cualquier sitio físico.
--
-- Un dispositivo se identifica porque su JWT trae el claim `sitio_id`
-- (ver las políticas "*_del_propio_sitio" y las de ingresos, que ya usan
-- exactamente esta comprobación) -- una sesión humana del panel (Google
-- OAuth) nunca trae ese claim. El fix agrega
-- "es dispositivo (tiene sitio_id) O es_admin_global()" a las seis
-- políticas "(global)": preserva EXACTAMENTE el mismo acceso para
-- dispositivos (recibir_catalogo_del_sitio/enviar_contratista/
-- enviar_usuario en src/nube/sincronizacion.rs siguen hablando directo
-- contra estas tablas con el JWT del dispositivo, sin cambios) y para el
-- panel (admin_global), pero cierra el acceso a cualquier otra sesión
-- `authenticated` que no sea ninguna de las dos cosas.
--
-- Mismo criterio de "envolver auth.jwt() en select" que
-- envuelve_auth_jwt_en_select_para_rls.sql, para no perder esa
-- optimización ya aplicada.

alter policy "leer contratistas (global)" on public.contratistas
using (
  ((select auth.jwt()) ->> 'sitio_id') is not null
  or public.es_admin_global()
);

alter policy "actualizar contratistas (global)" on public.contratistas
using (
  ((select auth.jwt()) ->> 'sitio_id') is not null
  or public.es_admin_global()
)
with check (
  ((select auth.jwt()) ->> 'sitio_id') is not null
  or public.es_admin_global()
);

alter policy "leer empresas (global)" on public.empresas
using (
  ((select auth.jwt()) ->> 'sitio_id') is not null
  or public.es_admin_global()
);

alter policy "actualizar empresas (global)" on public.empresas
using (
  ((select auth.jwt()) ->> 'sitio_id') is not null
  or public.es_admin_global()
)
with check (
  ((select auth.jwt()) ->> 'sitio_id') is not null
  or public.es_admin_global()
);

alter policy "leer usuarios (global)" on public.usuarios
using (
  ((select auth.jwt()) ->> 'sitio_id') is not null
  or public.es_admin_global()
);

alter policy "actualizar usuarios (global)" on public.usuarios
using (
  ((select auth.jwt()) ->> 'sitio_id') is not null
  or public.es_admin_global()
)
with check (
  ((select auth.jwt()) ->> 'sitio_id') is not null
  or public.es_admin_global()
);
