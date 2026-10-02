import type { CredencialDispositivo, TipoEventoSeguridad } from "../api/dispositivos";

/**
 * Lo que lleva el QR: sólo el código, con un esquema propio para que la app
 * del celular reconozca que es de vinculación (ver `CodigoVinculacion.kt`).
 * Ni secretos ni datos del sitio: el código vence en minutos y sirve una vez.
 */
export function contenidoQr(codigo: string): string {
  return `brisas-acceso://vincular?codigo=${codigo.replace(/[^0-9A-Z]/gi, "").toUpperCase()}`;
}

/** "14:59" hasta que vence, o `null` si ya venció. */
export function tiempoRestante(expiraEn: string, ahora: number): string | null {
  const restante = Math.floor((Date.parse(expiraEn) - ahora) / 1000);
  if (!(restante > 0)) return null;
  const horas = Math.floor(restante / 3600);
  const minutos = Math.floor((restante % 3600) / 60);
  const segundos = restante % 60;
  const dosCifras = (valor: number) => String(valor).padStart(2, "0");
  return horas > 0
    ? `${horas}:${dosCifras(minutos)}:${dosCifras(segundos)}`
    : `${minutos}:${dosCifras(segundos)}`;
}

export const TEXTO_CREDENCIAL: Record<CredencialDispositivo, string> = {
  clave: "Vinculado",
  sin_vincular: "Sin vincular",
};

export const TEXTO_EVENTO: Record<TipoEventoSeguridad, string> = {
  codigo_inexistente: "Código inexistente",
  codigo_usado: "Código ya usado",
  codigo_vencido: "Código vencido",
  codigo_anulado: "Código anulado",
  firma_invalida: "Firma inválida",
  hardware_distinto: "Hardware distinto al registrado",
  sesion_en_otra_unidad: "Sesión movida a otra unidad",
  sesion_en_duda: "Sesión en dos unidades (sin cerrar)",
};

/** Detalle legible de un evento, cuando lo tiene (lo arma
 * `sesion_usuario_en_unidad` en la base):
 * - `sesion_en_otra_unidad`: quién era y de qué unidad se le cerró la sesión.
 * - `sesion_en_duda`: quién era y en qué otra unidad sigue con sesión. Los
 *   dos ingresos fueron casi a la vez, así que no se cerró ninguno: ante la
 *   duda, ningún equipo queda sin poder operar. */
export function detalleEvento(tipo: TipoEventoSeguridad, detalle: unknown): string | null {
  if (tipo !== "sesion_en_otra_unidad" && tipo !== "sesion_en_duda") return null;
  if (!detalle || typeof detalle !== "object") return null;
  const datos = detalle as Record<string, unknown>;
  const nombre = typeof datos.nombre === "string" ? datos.nombre : null;
  const cedula = typeof datos.cedula === "string" ? datos.cedula : null;
  if (!nombre && !cedula) return null;
  const quien = nombre && cedula ? `${nombre} (${cedula})` : (nombre ?? cedula);
  if (tipo === "sesion_en_duda") {
    const otra = typeof datos.sitio_otro === "string" ? datos.sitio_otro : null;
    return otra ? `${quien}: sigue también con sesión en ${otra}` : quien;
  }
  const anterior = typeof datos.sitio_anterior === "string" ? datos.sitio_anterior : null;
  return anterior ? `${quien}: se cerró su sesión en ${anterior}` : quien;
}
