import { supabase } from "../lib/supabase";

/**
 * Alta/baja/reasignación de dispositivos -- llama a las Edge Functions
 * admin-list-devices/admin-provision-device/admin-revoke-device (mismas
 * que usaba el panel viejo, `admin-panel/panel-dispositivos.html`) más
 * admin-move-device (nueva, para cambiar un dispositivo de sitio -- hueco
 * que no existía antes). Ya no con la clave compartida `x-admin-key`:
 * ahora verifican la sesión real de Supabase Auth de quien llama contra
 * `administradores_panel`
 * (mismo criterio que el resto del panel via RLS). `supabase.functions.invoke`
 * manda el JWT de la sesión activa solo -- el panel viejo deja de
 * funcionar a partir de este cambio, a propósito (ver
 * docs/plan-panel-administrativo-web.md).
 */
export interface Sitio {
  id: string;
  nombre: string;
  direccion: string | null;
  created_at: string;
}

export type TipoDispositivo = "pc" | "mobile" | "visor";

export interface Dispositivo {
  id: string;
  sitio_id: string;
  tipo: TipoDispositivo;
  etiqueta: string;
  created_at: string;
  revoked_at: string | null;
}

export interface DispositivoProvisionado {
  sitio_id: string;
  sitio_nombre: string;
  dispositivo_id: string;
  secret: string;
}

async function invocar<T>(nombre: string, body?: Record<string, unknown>): Promise<T> {
  const { data, error } = await supabase.functions.invoke<T>(nombre, { body });
  if (error) {
    let detalle: string | undefined;
    const contexto = (error as { context?: Response }).context;
    if (contexto instanceof Response) {
      try {
        const cuerpo = await contexto.clone().json();
        detalle = cuerpo?.detail ?? cuerpo?.error;
      } catch {
        // Sin cuerpo JSON legible -- se usa error.message más abajo.
      }
    }
    throw new Error(detalle ?? error.message);
  }
  return data as T;
}

export function listarDispositivosYSitios(): Promise<{ sitios: Sitio[]; dispositivos: Dispositivo[] }> {
  return invocar("admin-list-devices");
}

export function provisionarDispositivo(datos: {
  sitio_nombre: string;
  sitio_direccion?: string;
  tipo: TipoDispositivo;
  etiqueta: string;
}): Promise<DispositivoProvisionado> {
  return invocar("admin-provision-device", datos);
}

export function revocarDispositivo(dispositivoId: string): Promise<void> {
  return invocar("admin-revoke-device", { dispositivo_id: dispositivoId });
}

export function moverDispositivo(
  dispositivoId: string,
  datos: { sitio_nombre: string; sitio_direccion?: string },
): Promise<{ sitio_id: string; sitio_nombre: string }> {
  return invocar("admin-move-device", { dispositivo_id: dispositivoId, ...datos });
}
