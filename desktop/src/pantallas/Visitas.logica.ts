import type {
  AgendaVisitaResumen,
  MovimientoHistorialVisitaRemoto,
  MovimientoVisitaActivoResumen,
  MovimientoVisitaRemoto,
} from "../api";
import type { FilaCorreoActiva, HistorialIngresoCorreoRemoto } from "../api/correo";
import { textoMedioConPlaca } from "../api/ingresos";

/**
 * Visitas y ingresos "por correo" en una sola pantalla (pedido del dueño
 * 2026-10-05): para la portería son lo mismo, personas que no vienen todos
 * los días. La visita agendada en la web es el camino principal; "por
 * correo" es la que el guarda registra a mano con un correo que la respalde.
 * Cada una sigue en su tabla y se sincroniza por su camino: sólo se unen
 * acá, para mostrarlas.
 */
export type OrigenVisita = "AGENDADA" | "POR_CORREO";

export const TEXTO_ORIGEN: Record<OrigenVisita, string> = {
  AGENDADA: "Agendada",
  POR_CORREO: "Por correo",
};

/** Mismo criterio que la base para comparar cédulas: sin espacios, puntos
 * ni guiones, en mayúsculas. */
export function cedulaComparable(cedula: string): string {
  return cedula.replace(/[\s.-]/g, "").toUpperCase();
}

export interface FilaVisitaAdentro {
  clave: string;
  origen: OrigenVisita;
  cedula: string;
  nombre: string;
  empresa: string | null;
  anfitrion: string | null;
  motivo: string | null;
  placa: string | null;
  gafete_numero: number | null;
  /** ISO 8601 (UTC). */
  fecha_hora_entrada: string;
  /** Para dar la salida por el camino de cada una: la visita del otro
   * equipo (el teléfono) se cierra en la nube. */
  fuente:
    | { tipo: "visita"; id: number }
    | { tipo: "visita_remota"; uuid: string }
    | { tipo: "correo"; fila: FilaCorreoActiva };
}

const masReciente = (a: { fecha_hora_entrada: string }, b: { fecha_hora_entrada: string }) =>
  b.fecha_hora_entrada.localeCompare(a.fecha_hora_entrada);

export function unirAdentro(
  visitas: MovimientoVisitaActivoResumen[],
  visitasRemotas: MovimientoVisitaRemoto[],
  correos: FilaCorreoActiva[],
): FilaVisitaAdentro[] {
  return [
    ...visitas.map(
      (visita): FilaVisitaAdentro => ({
        clave: `visita-${visita.id}`,
        origen: "AGENDADA",
        cedula: visita.cedula,
        nombre: visita.nombre,
        empresa: visita.empresa,
        anfitrion: visita.anfitrion_nombre,
        motivo: visita.motivo,
        placa: visita.placa,
        gafete_numero: visita.gafete_numero,
        fecha_hora_entrada: visita.fecha_hora_entrada,
        fuente: { tipo: "visita", id: visita.id },
      }),
    ),
    ...visitasRemotas.map(
      (visita): FilaVisitaAdentro => ({
        clave: `visita-remota-${visita.uuid}`,
        origen: "AGENDADA",
        cedula: visita.cedula,
        nombre: visita.nombre,
        empresa: visita.empresa,
        anfitrion: visita.anfitrion_nombre,
        motivo: visita.motivo,
        placa: visita.placa,
        gafete_numero: visita.gafete_numero,
        fecha_hora_entrada: visita.hora_entrada,
        fuente: { tipo: "visita_remota", uuid: visita.uuid },
      }),
    ),
    ...correos.map(
      (correo): FilaVisitaAdentro => ({
        clave: correo.origen === "local" ? `correo-${correo.id}` : `correo-remoto-${correo.uuid_remoto}`,
        origen: "POR_CORREO",
        cedula: correo.cedula,
        nombre: correo.nombre,
        empresa: null,
        anfitrion: null,
        motivo: correo.motivo,
        placa: correo.placa,
        gafete_numero: correo.gafete_numero,
        fecha_hora_entrada: correo.fecha_hora_ingreso,
        fuente: { tipo: "correo", fila: correo },
      }),
    ),
  ].sort(masReciente);
}

export interface FilaVisitaHistorial {
  clave: string;
  origen: OrigenVisita;
  cedula: string;
  nombre: string;
  empresa: string | null;
  anfitrion: string | null;
  motivo: string | null;
  placa: string | null;
  gafete_numero: number | null;
  /** ISO 8601 (UTC). */
  fecha_hora_entrada: string;
  fecha_hora_salida: string | null;
  usuario_entrada_nombre: string | null;
  usuario_salida_nombre: string | null;
}

export function unirHistorial(
  visitas: MovimientoHistorialVisitaRemoto[],
  correos: HistorialIngresoCorreoRemoto[],
): FilaVisitaHistorial[] {
  return [
    ...visitas.map(
      (visita): FilaVisitaHistorial => ({
        clave: `visita-${visita.uuid}`,
        origen: "AGENDADA",
        cedula: visita.cedula,
        nombre: visita.nombre,
        empresa: visita.empresa,
        anfitrion: visita.anfitrion_nombre,
        motivo: visita.motivo,
        placa: visita.placa,
        gafete_numero: visita.gafete_numero,
        fecha_hora_entrada: visita.fecha_hora_entrada,
        fecha_hora_salida: visita.fecha_hora_salida,
        usuario_entrada_nombre: visita.usuario_entrada_nombre,
        usuario_salida_nombre: visita.usuario_salida_nombre,
      }),
    ),
    ...correos.map(
      (correo): FilaVisitaHistorial => ({
        clave: `correo-${correo.uuid}`,
        origen: "POR_CORREO",
        cedula: correo.cedula,
        nombre: correo.nombre,
        empresa: null,
        anfitrion: null,
        motivo: correo.motivo,
        placa: correo.placa,
        gafete_numero: correo.gafete_numero,
        fecha_hora_entrada: correo.fecha_hora_ingreso,
        fecha_hora_salida: correo.fecha_hora_salida,
        usuario_entrada_nombre: correo.usuario_ingreso_nombre,
        usuario_salida_nombre: correo.usuario_salida_nombre,
      }),
    ),
  ].sort(masReciente);
}

export type EstadoEsperada = { tipo: "sin_llegar" } | { tipo: "salio"; hora: string };

export interface FilaEsperada extends AgendaVisitaResumen {
  clave: string;
  /** Si ya vino hoy y salió (una cita de varios días le deja volver). */
  llegada: EstadoEsperada;
}

/**
 * Las visitas que se esperan hoy: citas vigentes cuyo rango incluye `hoy`
 * (`AAAA-MM-DD`), sin quienes ya están adentro (por cualquier vía). Quien ya
 * vino y salió hoy queda al final, marcado: una cita de varios días le deja
 * volver. Las que siguen sin llegar van por hora estimada (sin hora, al
 * final) y nombre. Una persona con dos citas hoy aparece una sola vez.
 */
export function visitasEsperadasHoy(
  agenda: AgendaVisitaResumen[],
  adentro: { cedula: string }[],
  historialHoy: { cedula: string; fecha_hora_salida: string | null }[],
  hoy: string,
): FilaEsperada[] {
  const yaAdentro = new Set(adentro.map((fila) => cedulaComparable(fila.cedula)));
  const salidas = new Map<string, string>();
  for (const movimiento of historialHoy) {
    if (!movimiento.fecha_hora_salida) continue;
    const cedula = cedulaComparable(movimiento.cedula);
    const anterior = salidas.get(cedula);
    if (!anterior || movimiento.fecha_hora_salida > anterior) salidas.set(cedula, movimiento.fecha_hora_salida);
  }

  const vistas = new Set<string>();
  const filas: FilaEsperada[] = [];
  for (const cita of agenda) {
    const cedula = cedulaComparable(cita.cedula);
    if (cita.estado !== "Vigente" || cita.fecha_desde > hoy || cita.fecha_hasta < hoy) continue;
    if (yaAdentro.has(cedula) || vistas.has(cedula)) continue;
    vistas.add(cedula);
    const salida = salidas.get(cedula);
    filas.push({
      ...cita,
      clave: `${cita.cita_id}-${cedula}`,
      llegada: salida ? { tipo: "salio", hora: salida } : { tipo: "sin_llegar" },
    });
  }

  const orden = (fila: FilaEsperada) => (fila.llegada.tipo === "salio" ? 1 : 0);
  return filas.sort(
    (a, b) =>
      orden(a) - orden(b) ||
      (a.hora_estimada ?? "99").localeCompare(b.hora_estimada ?? "99") ||
      a.nombre.localeCompare(b.nombre, "es"),
  );
}


/** "Sin placa = caminando", la misma regla del formulario. */
export function textoMedioDePlaca(placa: string | null): string {
  const conPlaca = placa?.trim() ? placa.trim() : null;
  return textoMedioConPlaca(conPlaca ? "Vehiculo" : "Caminando", conPlaca);
}
