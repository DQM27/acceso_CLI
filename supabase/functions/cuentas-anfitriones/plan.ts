// Decisión pura de `cuentas-anfitriones`: qué cuentas de Supabase Auth crear,
// bloquear o desbloquear según la tabla `anfitriones`. Sin red ni Supabase,
// para poder probarla entera (ver plan.test.ts).

/** Marca en `app_metadata` (solo la escribe el service role). */
export const ORIGEN = "anfitriones";

export interface FilaAnfitrion {
  correo: string;
  activo: boolean;
}

/** Lo que hace falta de una cuenta de Auth. */
export interface Cuenta {
  id: string;
  email?: string;
  banned_until?: string | null;
  app_metadata?: Record<string, unknown>;
}

export type Accion =
  | { tipo: "crear"; correo: string }
  | { tipo: "bloquear"; id: string; app_metadata: Record<string, unknown> }
  | { tipo: "desbloquear"; id: string; app_metadata: Record<string, unknown> };

function bloqueada(cuenta: Cuenta, ahora: number): boolean {
  return !!cuenta.banned_until && new Date(cuenta.banned_until).getTime() > ahora;
}

/**
 * - Anfitrión activo sin cuenta → crear.
 * - Cuenta de anfitriones (creada acá, o de un anfitrión existente, p. ej.
 *   que venía de Google) cuyo anfitrión está inactivo o ya no está en la
 *   tabla → bloquear.
 * - Bloqueo puesto acá cuyo anfitrión volvió a estar activo, o que pasó a
 *   ser administrador → desbloquear.
 * - Nunca toca administradores del panel, operadores `@brisas.local` ni
 *   bloqueos puestos a mano por otra razón.
 */
export function planificar(
  anfitriones: FilaAnfitrion[],
  administradores: string[],
  cuentas: Cuenta[],
  ahora = Date.now(),
): Accion[] {
  const esAdmin = new Set(administradores.map((correo) => correo.toLowerCase()));
  const deseado = new Map(anfitriones.map((fila) => [fila.correo.toLowerCase(), fila.activo]));
  const porCorreo = new Map<string, Cuenta>();
  for (const cuenta of cuentas) if (cuenta.email) porCorreo.set(cuenta.email.toLowerCase(), cuenta);

  const acciones: Accion[] = [];
  for (const [correo, activo] of deseado) {
    if (activo && !porCorreo.has(correo)) acciones.push({ tipo: "crear", correo });
  }
  for (const [correo, cuenta] of porCorreo) {
    const meta = cuenta.app_metadata ?? {};
    const activo = deseado.get(correo);
    const esDeAnfitriones = meta.origen === ORIGEN || activo !== undefined;
    const debeBloquear =
      esDeAnfitriones && activo !== true && !esAdmin.has(correo) && !correo.endsWith("@brisas.local");

    if (debeBloquear && !bloqueada(cuenta, ahora)) {
      acciones.push({ tipo: "bloquear", id: cuenta.id, app_metadata: { ...meta, bloqueada_por: ORIGEN } });
    } else if (!debeBloquear && meta.bloqueada_por === ORIGEN) {
      acciones.push({ tipo: "desbloquear", id: cuenta.id, app_metadata: { ...meta, bloqueada_por: null } });
    }
  }
  return acciones;
}
