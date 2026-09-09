import { invoke } from "@tauri-apps/api/core";
import { solicitarSincronizacionNube } from "../eventosNube";

// Espejo de comandos/citas.rs — ver también src/application/citas.rs y
// src/models/cita.rs del núcleo. Dominio de "Control de visitas"
// (docs/plan-control-visitas.md), separado a propósito del de contratistas
// aunque comparta la misma infraestructura de sync.

export type EstadoCita = "Vigente" | "Cancelada";

export interface Cita {
  id: number;
  motivo: string | null;
  /** `YYYY-MM-DD`. */
  fecha_desde: string;
  /** `YYYY-MM-DD`. */
  fecha_hasta: string;
  anfitrion_nombre: string;
  anfitrion_correo: string;
  estado: EstadoCita;
}

export interface CitaVisitante {
  id: number;
  cita_id: number;
  cedula: string;
  nombre: string;
  empresa: string | null;
  placa_vehiculo: string | null;
}

export interface PreparacionVisita {
  cita: Cita;
  visitante: CitaVisitante;
}

export interface MovimientoVisitaActivoResumen {
  id: number;
  cedula: string;
  nombre: string;
  empresa: string | null;
  gafete_numero: number | null;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  fecha_hora_entrada: string;
  anfitrion_nombre: string;
  motivo: string | null;
}

export function verificarCheckInVisita(cedula: string): Promise<PreparacionVisita> {
  return invoke("verificar_check_in_visita", { cedula });
}

export async function registrarEntradaVisita(
  cedula: string,
  gafete: number | null,
): Promise<number> {
  const id = await invoke<number>("registrar_entrada_visita", { cedula, gafete });
  solicitarSincronizacionNube();
  return id;
}

export async function registrarSalidaVisita(movimientoId: number): Promise<void> {
  await invoke("registrar_salida_visita", { movimientoId });
  solicitarSincronizacionNube();
}

export function listarVisitasActivas(): Promise<MovimientoVisitaActivoResumen[]> {
  return invoke("listar_visitas_activas");
}
