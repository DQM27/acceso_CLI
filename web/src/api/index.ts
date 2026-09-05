export type RolAdminPanel = "admin_global" | "admin_regional";

export interface UsuarioSesion {
  nombre: string;
  correo: string;
  rol: RolAdminPanel;
}
