import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/gafetes.rs y dto/gafetes.rs.

export type EstadoGafete = "Disponible" | "Perdido" | "DeBaja";

export type MotivoResolucionGafete = "Pagado" | "Aparecido";

export interface GafeteResumen {
  id: number;
  numero: number;
  estado: EstadoGafete;
  contratista_deudor_id: number | null;
  contratista_deudor_nombre: string | null;
  fecha_marcado_perdido: string | null;
}

export interface FiltroGafetes {
  numero?: number;
  // snake_case y capitalizado a propósito — espejo exacto de
  // `EstadoGafeteEntrada` (Rust, `#[serde(rename_all = "snake_case")]`).
  estado?: "disponible" | "perdido" | "de_baja";
}

export function buscarGafetes(filtro: FiltroGafetes): Promise<GafeteResumen[]> {
  return invoke("buscar_gafetes", { filtro });
}

export function crearGafete(numero: number): Promise<number> {
  return invoke("crear_gafete", { numero });
}

export function crearGafetesRango(desde: number, hasta: number): Promise<number[]> {
  return invoke("crear_gafetes_rango", { desde, hasta });
}

export function darDeBajaGafete(id: number): Promise<void> {
  return invoke("dar_de_baja_gafete", { id });
}

export function marcarGafetePerdido(id: number, contratistaId: number): Promise<void> {
  return invoke("marcar_gafete_perdido", { id, contratistaId });
}

export function resolverGafete(id: number, motivo: MotivoResolucionGafete): Promise<void> {
  return invoke("resolver_gafete", { id, motivo });
}
