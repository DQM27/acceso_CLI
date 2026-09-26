import type { GafeteResumen } from "../api";

export function textoEstado(estado: GafeteResumen["estado"]): string {
  switch (estado) {
    case "Disponible":
      return "Disponible";
    case "Perdido":
      return "Perdido";
    case "DeBaja":
      return "De baja";
  }
}
