import { invoke } from "./invocar";
import { solicitarSincronizacionNube } from "../eventosNube";
import { cerrarIngresoCorreoRemoto, listarIngresosCorreoRemotos } from "./nube";
import type { IngresoCorreoRemoto } from "./nube";

// Espejo de comandos/correo.rs y dto/correo.rs -- ingreso "por correo":
// visita autorizada por correo (generalmente entrevistas de RH), comodín
// mientras se termina el módulo de Visitas. Mismo molde que api/proveedores.ts,
// con el motivo en vez de la empresa y gafete de visita.

/** Fila de la pantalla "Por correo" (`RegistroIngresoCorreoActivoResumen`). */
export interface CorreoActivoResumen {
  id: number;
  cedula: string;
  nombre: string;
  motivo: string;
  placa: string | null;
  gafete_numero: number;
  /** ISO 8601 (UTC). */
  fecha_hora_ingreso: string;
  usuario_ingreso_nombre: string;
}

/** Espejo de `SolicitudIngresoCorreoEntrada` (dto/correo.rs). */
export interface SolicitudIngresoCorreo {
  cedula: string;
  nombre: string;
  motivo: string;
  /** `null` o vacía = llegó a pie. */
  placa: string | null;
  gafete_numero: number;
}

export async function registrarIngresoCorreo(solicitud: SolicitudIngresoCorreo): Promise<number> {
  const id = await invoke<number>("registrar_ingreso_correo", { solicitud });
  solicitarSincronizacionNube();
  return id;
}

export async function registrarSalidaCorreo(id: number): Promise<void> {
  await invoke("registrar_salida_correo", { id });
  solicitarSincronizacionNube();
}

export function listarCorreosActivos(): Promise<CorreoActivoResumen[]> {
  return invoke("listar_correos_activos");
}

// ---- Local + remoto fusionados (mismo criterio que api/proveedores.ts) ----

export interface FilaCorreoLocal extends CorreoActivoResumen {
  origen: "local";
}

export interface FilaCorreoRemota {
  origen: "remoto";
  uuid_remoto: string;
  id: null;
  cedula: string;
  nombre: string;
  motivo: string;
  placa: string | null;
  gafete_numero: number;
  fecha_hora_ingreso: string;
  usuario_ingreso_nombre: string;
}

export type FilaCorreoActiva = FilaCorreoLocal | FilaCorreoRemota;

function filaDesdeRemoto(remoto: IngresoCorreoRemoto): FilaCorreoActiva {
  return {
    origen: "remoto",
    uuid_remoto: remoto.uuid,
    id: null,
    cedula: remoto.cedula,
    nombre: remoto.nombre,
    motivo: remoto.motivo,
    placa: remoto.placa,
    gafete_numero: remoto.gafete_numero,
    fecha_hora_ingreso: remoto.hora_entrada,
    usuario_ingreso_nombre: remoto.usuario_entrada_nombre,
  };
}

/** Clave estable para la grilla: `id` es `null` en una remota. */
export function claveFilaCorreoActiva(fila: FilaCorreoActiva): string {
  return fila.origen === "local" ? `local-${fila.id}` : `remoto-${fila.uuid_remoto}`;
}

export async function listarTodosLosCorreosActivos(): Promise<FilaCorreoActiva[]> {
  const [locales, remotos] = await Promise.all([listarCorreosActivos(), listarIngresosCorreoRemotos()]);
  return [
    ...locales.map((fila): FilaCorreoActiva => ({ ...fila, origen: "local" })),
    ...remotos.map(filaDesdeRemoto),
  ];
}

/** Local: cierra en este equipo. Remota: cierra directo contra la nube. */
export async function cerrarFilaCorreoActiva(fila: FilaCorreoActiva): Promise<void> {
  if (fila.origen === "local") {
    await registrarSalidaCorreo(fila.id);
  } else {
    await cerrarIngresoCorreoRemoto(fila.uuid_remoto);
  }
}

// ---- Historial (exclusivo de escritorio) ----

/** Espejo de `comandos::correo::HistorialIngresoCorreoRemoto` (caché
 * `historial_ingresos_correo_sitio`, toda la unidad). */
export interface HistorialIngresoCorreoRemoto {
  uuid: string;
  cedula: string;
  nombre: string;
  motivo: string | null;
  placa: string | null;
  gafete_numero: number | null;
  /** ISO 8601 (UTC). */
  fecha_hora_ingreso: string;
  fecha_hora_salida: string | null;
  usuario_ingreso_nombre: string | null;
  usuario_salida_nombre: string | null;
}

export function listarHistorialIngresosCorreoSitio(
  desde?: string,
  hasta?: string,
): Promise<HistorialIngresoCorreoRemoto[]> {
  return invoke("listar_historial_ingresos_correo_sitio", {
    desde: desde ?? null,
    hasta: hasta ?? null,
  });
}
