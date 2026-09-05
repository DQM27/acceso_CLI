import type { RolAdminPanel } from "../api";

const CLAVE = "web:accion-pendiente";
// El link de confirmación en sí ya vence según la config de Supabase
// (Auth > Providers > Email > Email OTP Expiration) -- esto es sólo un
// resguardo extra para no reintentar una acción de hace días si alguien
// vuelve a esta pestaña mucho después sin haber llegado a confirmar.
const VIGENCIA_MS = 15 * 60 * 1000;

interface AccionAgregarAdmin {
  tipo: "agregar_admin";
  correoSolicitante: string;
  correoNuevo: string;
  rolNuevo: RolAdminPanel;
  creadaEn: number;
}

interface AccionQuitarAdmin {
  tipo: "quitar_admin";
  correoSolicitante: string;
  correoAQuitar: string;
  creadaEn: number;
}

export type AccionPendiente = AccionAgregarAdmin | AccionQuitarAdmin;

// `Omit<Union, K>` NO se distribuye por miembro (`keyof Union` es la
// intersección de claves, no la unión) -- perdería `correoNuevo`/
// `correoAQuitar` de cada variante. `T extends any ? ... : never` fuerza
// la distribución para que cada miembro conserve sus propios campos.
type SinFecha<T> = T extends AccionPendiente ? Omit<T, "creadaEn"> : never;

/**
 * Acción sensible confirmada por correo, pendiente de completarse cuando
 * la persona vuelva a abrir el panel después de hacer clic en el link de
 * confirmación (ver `useVerificacionPorCorreo` y `Administradores.tsx`).
 * `sessionStorage` (no `localStorage`): si el link se abre en otra pestaña
 * o dispositivo, no hay forma de retomarlo ahí -- mejor que desaparezca
 * solo a que quede pendiente para siempre en un lugar que nadie va a
 * revisar.
 */
export function guardarAccionPendiente(accion: SinFecha<AccionPendiente>) {
  try {
    sessionStorage.setItem(CLAVE, JSON.stringify({ ...accion, creadaEn: Date.now() }));
  } catch {
    // sessionStorage puede fallar (modo privado, cuota llena) -- sin
    // guardado, simplemente no hay nada que retomar al volver.
  }
}

export function leerAccionPendienteVigente(correoSolicitante: string): AccionPendiente | null {
  try {
    const crudo = sessionStorage.getItem(CLAVE);
    if (!crudo) return null;
    const accion = JSON.parse(crudo) as AccionPendiente;
    if (accion.correoSolicitante !== correoSolicitante) return null;
    if (Date.now() - accion.creadaEn > VIGENCIA_MS) return null;
    return accion;
  } catch {
    return null;
  }
}

export function borrarAccionPendiente() {
  try {
    sessionStorage.removeItem(CLAVE);
  } catch {
    // Ver comentario de guardarAccionPendiente.
  }
}
