import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import { invocar, esObjeto } from "./_invocar";

/**
 * Cuentas de anfitriones de la web de visitas (visitas.megabrisas.com). Ver
 * la migración `activacion_de_anfitriones_desde_el_panel` y
 * docs/auditorias/investigacion-login-correo-web-visitas-2026-10-06.md.
 *
 * El alta y el "restablecer" devuelven un CÓDIGO DE ACTIVACIÓN que se
 * muestra una sola vez: la persona lo usa en la web de visitas («Primer
 * ingreso») para elegir su contraseña. Vence a las 72 h y se agota con 5
 * intentos fallidos. En la base solo queda su hash.
 */
export type EstadoAnfitrion =
  | "activa"
  | "pendiente"
  | "codigo_vencido"
  | "sin_contrasena"
  | "sin_cuenta"
  | "deshabilitada";

export interface Anfitrion {
  correo: string;
  nombre: string;
  activo: boolean;
  estado: EstadoAnfitrion;
  codigo_vence: string | null;
  creado_en: string;
}

const filaEsquema = z.object({
  correo: z.string(),
  nombre: z.string(),
  activo: z.boolean(),
  estado: z.enum(["activa", "pendiente", "codigo_vencido", "sin_contrasena", "sin_cuenta", "deshabilitada"]),
  codigo_vence: z.string().nullable(),
  creado_en: z.string(),
});

/** Listado con el estado de cada cuenta (RPC `panel_anfitriones`, exige
 * administrador del panel; nunca devuelve hashes). */
export async function listarAnfitriones(): Promise<Anfitrion[]> {
  const { data, error } = await supabase.rpc("panel_anfitriones");
  if (error) throw new Error(error.message);
  return z.array(filaEsquema).parse(data);
}

export interface CodigoEmitido {
  correo: string;
  nombre: string;
  /** Se muestra una sola vez: la función no lo vuelve a devolver. */
  codigo: string;
  /** ISO; 72 h después de emitido. */
  vence: string;
}

function esCodigoEmitido(valor: unknown): valor is CodigoEmitido {
  return (
    esObjeto(valor) &&
    typeof valor.correo === "string" &&
    typeof valor.codigo === "string" &&
    typeof valor.vence === "string"
  );
}

function esCambioDeEstado(valor: unknown): valor is { correo: string; activo: boolean } {
  return esObjeto(valor) && typeof valor.activo === "boolean";
}

/** Alta: crea el anfitrión y su cuenta, y devuelve el código de activación. */
export function crearAnfitrion(datos: { correo: string; nombre: string }): Promise<CodigoEmitido> {
  return invocar("admin-anfitriones", esCodigoEmitido, { accion: "crear", ...datos });
}

/** Olvidó la contraseña (o nunca activó): invalida la contraseña actual,
 * cierra sus sesiones y emite un código nuevo. */
export function restablecerAnfitrion(correo: string): Promise<CodigoEmitido> {
  return invocar("admin-anfitriones", esCodigoEmitido, { accion: "restablecer", correo });
}

/** Deshabilitar bloquea la cuenta y cierra sus sesiones; habilitar la
 * desbloquea (vuelve a entrar con su contraseña de siempre). */
export function cambiarEstadoAnfitrion(correo: string, activo: boolean): Promise<{ correo: string; activo: boolean }> {
  return invocar("admin-anfitriones", esCambioDeEstado, {
    accion: activo ? "habilitar" : "deshabilitar",
    correo,
  });
}
