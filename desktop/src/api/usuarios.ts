import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/usuarios.rs. Crear/editar/buscar usuarios globales u
// otorgarles/resetearles la contraseña desde el escritorio ya no existe --
// esa capacidad quedó exclusiva del panel administrativo web (ver
// docs/planes-implementados/plan-autenticacion-supabase-auth.md). El único comando que sigue
// vivo acá es el cambio de la propia contraseña.

/** Cambia la contraseña de la sesión actual — revalida `passwordActual`
 * antes de aceptar la nueva, nunca la de otro usuario. */
export function cambiarMiPassword(passwordActual: string, nuevaPassword: string): Promise<void> {
  return invoke("cambiar_mi_password", { passwordActual, nuevaPassword });
}
