import type { IncidenteGafete } from "../api";

export function textoMotivo(motivo: IncidenteGafete["motivo_resolucion"]): string {
  switch (motivo) {
    case "Pagado":
      return "Pagado";
    case "Aparecido":
      return "Apareció";
    case null:
      return "—";
  }
}
