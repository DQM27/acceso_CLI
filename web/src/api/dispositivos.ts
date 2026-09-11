import { invocar, esObjeto, loQueSea } from "./_invocar";

/**
 * Alta/baja/suspensión de dispositivos -- llama a las Edge Functions
 * admin-list-devices/admin-provision-device/admin-revoke-device/
 * admin-suspend-device (las tres primeras ya las usaba el panel viejo,
 * `admin-panel/panel-dispositivos.html`). Ya no con la clave compartida
 * `x-admin-key`: ahora verifican la sesión real de Supabase Auth de quien
 * llama contra `administradores_panel`
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
  suspended_at: string | null;
  last_seen_at: string | null;
  oculto_en_panel: boolean;
  identificador_hardware: string | null;
  nombre_dispositivo: string | null;
  plataforma: string | null;
  version_build: string | null;
  app_version: string | null;
  last_ip: string | null;
}

export interface DispositivoProvisionado {
  sitio_id: string;
  sitio_nombre: string;
  dispositivo_id: string;
  secret: string;
}

function esSitio(valor: unknown): valor is Sitio {
  return esObjeto(valor) && typeof valor.id === "string" && typeof valor.nombre === "string";
}

function esListaDispositivosYSitios(
  valor: unknown,
): valor is { sitios: Sitio[]; dispositivos: Dispositivo[] } {
  return (
    esObjeto(valor) &&
    Array.isArray(valor.sitios) &&
    Array.isArray(valor.dispositivos) &&
    valor.sitios.every(esSitio) &&
    valor.dispositivos.every((d) => esObjeto(d) && typeof d.id === "string")
  );
}

function esDispositivoProvisionado(valor: unknown): valor is DispositivoProvisionado {
  return (
    esObjeto(valor) &&
    typeof valor.sitio_id === "string" &&
    typeof valor.sitio_nombre === "string" &&
    typeof valor.dispositivo_id === "string" &&
    typeof valor.secret === "string"
  );
}

function esResultadoEliminar(valor: unknown): valor is { borrado: boolean } {
  return esObjeto(valor) && typeof valor.borrado === "boolean";
}

export function listarDispositivosYSitios(): Promise<{ sitios: Sitio[]; dispositivos: Dispositivo[] }> {
  return invocar("admin-list-devices", esListaDispositivosYSitios);
}

/** Crea (o reutiliza, si ya existe por nombre) un sitio suelto -- para el
 * desplegable de "Sitio" del alta de dispositivos, sin tener que crear un
 * dispositivo a la vez. */
export function crearSitio(datos: { nombre: string; direccion?: string }): Promise<Sitio> {
  return invocar("admin-create-site", esSitio, datos);
}

export function provisionarDispositivo(datos: {
  sitio_nombre: string;
  sitio_direccion?: string;
  tipo: TipoDispositivo;
  etiqueta: string;
}): Promise<DispositivoProvisionado> {
  return invocar("admin-provision-device", esDispositivoProvisionado, datos);
}

export function revocarDispositivo(dispositivoId: string): Promise<void> {
  return invocar("admin-revoke-device", loQueSea, { dispositivo_id: dispositivoId });
}

export function suspenderDispositivo(dispositivoId: string, suspendido: boolean): Promise<void> {
  return invocar("admin-suspend-device", loQueSea, { dispositivo_id: dispositivoId, suspendido });
}

/**
 * Intenta un borrado definitivo. Si el dispositivo ya generó historial real
 * (contratistas/ingresos/usuarios/gafetes con `dispositivo_origen_id`
 * apuntando a él), Postgres rechaza el borrado -- en ese caso
 * admin-delete-device no falla, lo marca `oculto_en_panel` y devuelve
 * `borrado: false`, así igual desaparece de la lista sin perder su
 * historial. Recuperar uno oculto es a propósito solo por SQL directo en
 * Supabase (`update dispositivos set oculto_en_panel = false where ...`),
 * no hay botón para eso en el panel.
 */
export function eliminarDispositivo(dispositivoId: string): Promise<{ borrado: boolean }> {
  return invocar("admin-delete-device", esResultadoEliminar, { dispositivo_id: dispositivoId });
}
