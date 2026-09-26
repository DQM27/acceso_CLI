import type { MovimientoVisitaActivoResumen } from "../api";

/** El gafete es opcional acá (a diferencia de `NuevoIngresoModal`, donde
 * puede ser requerido) -- vacío siempre es válido con `numero: null`. */
export function validarGafeteOpcional(
  texto: string,
): { valido: true; numero: number | null } | { valido: false; mensaje: string } {
  const recortado = texto.trim();
  if (!recortado) return { valido: true, numero: null };
  const numero = Number.parseInt(recortado, 10);
  if (Number.isNaN(numero)) return { valido: false, mensaje: "Ingrese un número de gafete válido" };
  return { valido: true, numero };
}

/** `verificar_check_in_visita` sólo confirma que la cita es válida hoy, no
 * si el visitante ya entró (ver el doc-comment de `visitasActivas` más
 * abajo) -- este es el filtro de UI que sí lo revisa. */
export function estaYaAdentro(
  visitasActivas: MovimientoVisitaActivoResumen[],
  cedula: string,
): boolean {
  return visitasActivas.some((fila) => fila.cedula === cedula);
}
