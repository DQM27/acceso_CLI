import { invocar, esObjeto, loQueSea } from "./_invocar";

/**
 * Alta, vinculación, baja y suspensión de dispositivos -- Edge Functions
 * `admin-*` autenticadas con la sesión real de Supabase Auth de quien llama,
 * contra `administradores_panel` (mismo criterio que el resto del panel via
 * RLS). `supabase.functions.invoke` manda el JWT de la sesión activa solo.
 *
 * Ningún dispositivo recibe un secreto permanente: el panel emite un
 * código de vinculación de un solo uso (ver
 * docs/features-futuras/propuesta-registro-dispositivos.md) que el equipo
 * canjea, generando su propia clave.
 */
export interface Sitio {
  id: string;
  nombre: string;
  created_at: string;
}

export type TipoDispositivo = "pc" | "mobile" | "visor";

/** Cómo se autentica hoy el equipo ante la nube. */
export type CredencialDispositivo = "clave" | "sin_vincular";

export interface Dispositivo {
  id: string;
  sitio_id: string;
  tipo: TipoDispositivo;
  etiqueta: string;
  created_at: string;
  revoked_at: string | null;
  last_seen_at: string | null;
  oculto_en_panel: boolean;
  identificador_hardware: string | null;
  nombre_dispositivo: string | null;
  plataforma: string | null;
  version_build: string | null;
  app_version: string | null;
  last_ip: string | null;
  /** Huella RFC 7638 de la clave del equipo, si ya tiene. */
  clave_huella: string | null;
  vinculado_en: string | null;
  credencial: CredencialDispositivo;
}

export interface CodigoPendiente {
  dispositivo_id: string;
  expira_en: string;
  creado_en: string;
  creado_por: string;
}

export type TipoEventoSeguridad =
  | "codigo_inexistente"
  | "codigo_usado"
  | "codigo_vencido"
  | "codigo_anulado"
  | "firma_invalida"
  | "hardware_distinto";

export interface EventoSeguridad {
  id: number;
  ocurrido_en: string;
  dispositivo_id: string | null;
  tipo: TipoEventoSeguridad;
  ip: string | null;
  detalle: unknown;
}

export interface EstadoDispositivos {
  sitios: Sitio[];
  dispositivos: Dispositivo[];
  codigos_pendientes: CodigoPendiente[];
  eventos: EventoSeguridad[];
}

/** Código recién emitido: sólo se ve acá, al crearlo. */
export interface CodigoEmitido {
  dispositivo_id: string;
  codigo: string;
  expira_en: string;
}

export interface DispositivoProvisionado extends CodigoEmitido {
  sitio_id: string;
  sitio_nombre: string;
}

function esSitio(valor: unknown): valor is Sitio {
  return esObjeto(valor) && typeof valor.id === "string" && typeof valor.nombre === "string";
}

function esEstadoDispositivos(valor: unknown): valor is EstadoDispositivos {
  return (
    esObjeto(valor) &&
    Array.isArray(valor.sitios) &&
    Array.isArray(valor.dispositivos) &&
    Array.isArray(valor.codigos_pendientes) &&
    Array.isArray(valor.eventos) &&
    valor.sitios.every(esSitio) &&
    valor.dispositivos.every((d) => esObjeto(d) && typeof d.id === "string")
  );
}

function esCodigoEmitido(valor: unknown): valor is CodigoEmitido {
  return (
    esObjeto(valor) &&
    typeof valor.dispositivo_id === "string" &&
    typeof valor.codigo === "string" &&
    typeof valor.expira_en === "string"
  );
}

function esDispositivoProvisionado(valor: unknown): valor is DispositivoProvisionado {
  return (
    esCodigoEmitido(valor) &&
    esObjeto(valor) &&
    typeof valor.sitio_id === "string" &&
    typeof valor.sitio_nombre === "string"
  );
}

function esResultadoEliminar(valor: unknown): valor is { borrado: boolean } {
  return esObjeto(valor) && typeof valor.borrado === "boolean";
}

export function listarDispositivosYSitios(): Promise<EstadoDispositivos> {
  return invocar("admin-list-devices", esEstadoDispositivos);
}

/** Crea (o reutiliza, si ya existe por nombre) un sitio suelto -- para el
 * desplegable de "Unidad operativa" del alta de dispositivos. */
export function crearSitio(datos: { nombre: string }): Promise<Sitio> {
  return invocar("admin-create-site", esSitio, datos);
}

/** Crea el dispositivo y su primer código de vinculación. El sitio va por
 * id: un nombre mal escrito ya no puede crear un sitio nuevo en silencio. */
export function provisionarDispositivo(datos: {
  sitio_id: string;
  tipo: TipoDispositivo;
  etiqueta: string;
  vigencia_minutos?: number;
}): Promise<DispositivoProvisionado> {
  return invocar("admin-provision-device", esDispositivoProvisionado, datos);
}

export function revocarDispositivo(dispositivoId: string): Promise<void> {
  return invocar("admin-revoke-device", loQueSea, { dispositivo_id: dispositivoId });
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
