import type { GafeteResumen, IncidenteGafete } from "../api";

export function textoEstado(estado: GafeteResumen["estado"]): string {
  switch (estado) {
    case "Disponible":
      return "Disponible";
    case "Perdido":
      return "Perdido";
  }
}

/** Largo máximo del detalle de un reporte (mismo tope que el núcleo). */
export const LARGO_MAXIMO_DETALLE = 1000;

/** El detalle tal como se manda: sin espacios a los costados, `null` si
 * queda vacío. */
export function detalleLimpio(texto: string): string | null {
  const limpio = texto.trim();
  return limpio.length > 0 ? limpio : null;
}

/** Nombre del evento de un reporte del historial. */
export function textoTipoIncidente(incidente: Pick<IncidenteGafete, "tipo" | "motivo_resolucion">): string {
  switch (incidente.tipo) {
    case "Perdido":
      return "Reportado perdido";
    case "Pagado":
      return "Pago registrado";
    case "Recuperado":
      return "Recuperado";
    case "Resuelto":
      return incidente.motivo_resolucion === "Pagado" ? "Resuelto · pagado" : "Resuelto · apareció";
  }
}
