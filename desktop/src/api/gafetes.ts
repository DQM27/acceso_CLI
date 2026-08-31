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

export type TipoIncidenteGafete = "Perdido" | "Resuelto";

/** Una fila del historial de un gafete puntual — espejo de `IncidenteGafete`
 * (`src/database/queries/gafetes_incidentes.rs`). */
export interface IncidenteGafete {
  id: number;
  tipo: TipoIncidenteGafete;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  fecha_hora: string;
  usuario_nombre: string;
  contratista_nombre: string | null;
  motivo_resolucion: MotivoResolucionGafete | null;
  /** A qué gafete pertenece — no hace falta cuando ya se sabe por contexto
   * (`historialGafete`), pero es indispensable en la vista global de
   * Auditoría (`listarAuditoriaGafetes`, `../pantallas/Auditoria.tsx`). */
  gafete_numero: number;
}

export function buscarGafetes(filtro: FiltroGafetes): Promise<GafeteResumen[]> {
  return invoke("buscar_gafetes", { filtro });
}

export function historialGafete(id: number): Promise<IncidenteGafete[]> {
  return invoke("historial_gafete", { id });
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
