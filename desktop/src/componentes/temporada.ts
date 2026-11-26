// Detalles de temporada fuera de la nieve: el copo de los avisos de éxito
// y el saludo del inicio de sesión (pedido del usuario 2026-11-26, igual
// que en el teléfono). Cuelgan del mismo interruptor que la nieve
// (`NAVIDAD_ACTIVA`, que cambia `scripts/icono_navideno.py`).
import { NAVIDAD_ACTIVA } from "./NieveNavidena";

/** El texto de un aviso de éxito con un copo al final, sólo en temporada. */
export function textoDeTemporada(texto: string): string {
  return NAVIDAD_ACTIVA ? `${texto} ❄️` : texto;
}

/** El saludo de temporada del inicio de sesión; `null` fuera de temporada. */
export const SALUDO_DE_TEMPORADA: string | null = NAVIDAD_ACTIVA
  ? "¡Felices fiestas!"
  : null;
