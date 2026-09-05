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
 * `localStorage`, no `sessionStorage` -- el link de confirmación casi
 * siempre abre en una pestaña NUEVA (comportamiento típico de clientes de
 * correo), y `sessionStorage` es exclusivo de cada pestaña: la pestaña
 * nueva nunca vería lo que guardó la original. `localStorage` se comparte
 * entre pestañas del mismo sitio, así que no importa cuál de las dos
 * retoma la acción. Sigue expirando sola (`VIGENCIA_MS`) si nadie llega a
 * confirmar, para no quedar pendiente para siempre en un dispositivo
 * compartido.
 */
export function guardarAccionPendiente(accion: SinFecha<AccionPendiente>) {
  try {
    localStorage.setItem(CLAVE, JSON.stringify({ ...accion, creadaEn: Date.now() }));
  } catch {
    // localStorage puede fallar (modo privado, cuota llena) -- sin
    // guardado, simplemente no hay nada que retomar al volver.
  }
}

export function leerAccionPendienteVigente(correoSolicitante: string): AccionPendiente | null {
  try {
    const crudo = localStorage.getItem(CLAVE);
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
    localStorage.removeItem(CLAVE);
  } catch {
    // Ver comentario de guardarAccionPendiente.
  }
}
