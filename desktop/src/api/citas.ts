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

/** Espejo de `comandos::citas::MovimientoHistorialVisitaRemoto` -- un
 * movimiento de visita (abierto o cerrado) leído de la caché local
 * `historial_visitas_sitio`, la misma que sincroniza
 * `nube::recibir_historial_visitas_del_sitio`. Sólo se llena en PC -- el
 * celular no trae esto (ver el comentario en `mobile/rust-core/src/lib.rs`),
 * así que este comando ni siquiera existe del lado móvil. */
export interface MovimientoHistorialVisitaRemoto {
  uuid: string;
  cedula: string;
  nombre: string;
  empresa: string | null;
  anfitrion_nombre: string | null;
  motivo: string | null;
  gafete_numero: number | null;
  /** ISO 8601 (UTC). */
  fecha_hora_entrada: string;
  fecha_hora_salida: string | null;
  usuario_entrada_nombre: string | null;
  usuario_salida_nombre: string | null;
}

export function listarHistorialVisitasSitio(
  desde?: string,
  hasta?: string,
): Promise<MovimientoHistorialVisitaRemoto[]> {
  return invoke("listar_historial_visitas_sitio", { desde: desde ?? null, hasta: hasta ?? null });
}

/** Espejo de `comandos::citas::AgendaVisitaResumen` -- lectura pura de
 * `citas`/`cita_visitantes` local (ya sincronizadas, sin viaje a la nube).
 * Trae toda cita vigente o cancelada cuyo rango no haya terminado todavía
 * ("hoy en adelante") -- no oculta las canceladas, se muestran con su
 * `estado` para que el filtro por columna decida qué ver. */
export interface AgendaVisitaResumen {
  cita_id: number;
  cedula: string;
  nombre: string;
  empresa: string | null;
  placa_vehiculo: string | null;
  motivo: string | null;
  anfitrion_nombre: string;
  /** `YYYY-MM-DD`. */
  fecha_desde: string;
  /** `YYYY-MM-DD`. */
  fecha_hasta: string;
  estado: EstadoCita;
}

export function listarAgendaVisitas(): Promise<AgendaVisitaResumen[]> {
  return invoke("listar_agenda_visitas");
}
