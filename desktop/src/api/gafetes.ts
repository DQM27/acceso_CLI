import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/gafetes.rs y dto/gafetes.rs.

export type EstadoGafete = "Disponible" | "Perdido" | "DeBaja";

/** Pool físico al que pertenece el gafete — espejo de `TipoGafete` (Rust).
 * Sin "Proveedor" todavía del lado de entrada: el backend ya acepta ese
 * valor en el CHECK de la base a futuro, pero no hay comando de alta para
 * esa categoría hasta que exista la entidad `proveedor`. */
export type TipoGafete = "Contratista" | "Visita";

export type MotivoResolucionGafete = "Pagado" | "Aparecido";

export interface GafeteResumen {
  id: number;
  numero: number;
  tipo: TipoGafete;
  estado: EstadoGafete;
  contratista_portador_id: number | null;
  contratista_portador_nombre: string | null;
  visita_portador_id: number | null;
  visita_portador_nombre: string | null;
  fecha_marcado_perdido: string | null;
}

/** A quién se le asignó este gafete la última vez, sea cual sea su tipo —
 * a lo sumo uno de los dos campos de `GafeteResumen`/`IncidenteGafete`
 * tiene valor, nunca los dos. */
export function nombrePortador(
  fila: Pick<GafeteResumen, "contratista_portador_nombre" | "visita_portador_nombre">,
): string | null {
  return fila.contratista_portador_nombre ?? fila.visita_portador_nombre;
}

// snake_case a propósito — espejo exacto de `TipoGafeteEntrada` (Rust,
// `#[serde(rename_all = "snake_case")]`). Sin "proveedor" -- mismo motivo
// que `TipoGafete` arriba.
export type TipoGafeteEntrada = "contratista" | "visita";

export interface FiltroGafetes {
  numero?: number;
  tipo?: TipoGafeteEntrada;
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
  visita_portador_nombre: string | null;
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

export function crearGafete(numero: number, tipo: TipoGafeteEntrada): Promise<number> {
  return invoke("crear_gafete", { numero, tipo });
}

export function crearGafetesRango(
  desde: number,
  hasta: number,
  tipo: TipoGafeteEntrada,
): Promise<number[]> {
  return invoke("crear_gafetes_rango", { desde, hasta, tipo });
}

export function darDeBajaGafete(id: number): Promise<void> {
  return invoke("dar_de_baja_gafete", { id });
}

export function marcarGafetePerdidoContratista(id: number, contratistaId: number): Promise<void> {
  return invoke("marcar_gafete_perdido_contratista", { id, contratistaId });
}

export function marcarGafetePerdidoVisita(id: number, citaVisitanteId: number): Promise<void> {
  return invoke("marcar_gafete_perdido_visita", { id, citaVisitanteId });
}

export function resolverGafete(id: number, motivo: MotivoResolucionGafete): Promise<void> {
  return invoke("resolver_gafete", { id, motivo });
}
