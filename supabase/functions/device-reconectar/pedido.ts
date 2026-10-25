// Lectura del pedido de `device-reconectar`, sin red ni base de datos.
// Separada de `index.ts` para probarla con `deno test` (pedido.test.ts).

import { LARGO_CODIGO, normalizarCodigo } from "../_shared/codigos.ts";

/** Lo que el equipo confirma al reconectarse en otra unidad: va al evento. */
export interface DetalleReconexion {
  /** Registros sin subir que el equipo borra al cambiar de unidad. */
  pendientes_descartados: number;
  /** Quién confirmó en el equipo (nombre de la sesión abierta), si hubo. */
  confirmado_por: string | null;
}

export interface PedidoReconexion {
  codigo: string;
  /** `false`: sólo consultar a dónde va (no gasta el código). */
  confirmar: boolean;
  detalle: DetalleReconexion;
}

const MAXIMO_PENDIENTES = 1_000_000;

/**
 * El pedido normalizado, o `null` si no tiene un código con forma de código.
 * Lo demás se acota: un número de pendientes raro queda en 0 y el nombre se
 * recorta, porque sólo son datos para el registro, nunca deciden nada.
 */
export function leerPedidoReconexion(cuerpo: Record<string, unknown> | null): PedidoReconexion | null {
  if (!cuerpo || typeof cuerpo.codigo !== "string") return null;
  const codigo = normalizarCodigo(cuerpo.codigo);
  if (codigo.length !== LARGO_CODIGO) return null;

  const pendientes = cuerpo.pendientes_descartados;
  const nombre = cuerpo.confirmado_por;
  return {
    codigo,
    confirmar: cuerpo.confirmar === true,
    detalle: {
      pendientes_descartados:
        typeof pendientes === "number" && Number.isInteger(pendientes) && pendientes >= 0 && pendientes <= MAXIMO_PENDIENTES
          ? pendientes
          : 0,
      confirmado_por: typeof nombre === "string" && nombre.trim() ? nombre.trim().slice(0, 120) : null,
    },
  };
}
