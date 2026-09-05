/**
 * Placeholder — la auth real (Supabase Auth: Google OAuth + Email OTP, ver
 * docs/plan-panel-administrativo-web.md) todavía no está implementada acá.
 * `UsuarioSesion` sólo existe por ahora para que `MenuUsuario.tsx` compile;
 * se reemplaza cuando se conecte el login de verdad.
 */
export type RolAdminPanel = "admin_global" | "admin_regional";

export interface UsuarioSesion {
  nombre: string;
  rol: RolAdminPanel;
}
