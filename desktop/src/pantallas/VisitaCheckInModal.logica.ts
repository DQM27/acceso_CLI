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
