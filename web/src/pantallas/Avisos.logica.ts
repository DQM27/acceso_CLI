import type { DestinoAviso } from "../api/avisos";

/** Lo mínimo de un equipo para elegir destinatarios. */
export interface EquipoAviso {
  id: string;
  sitio_id: string;
}

export interface SeleccionAviso {
  todos: boolean;
  /** Unidades marcadas completas: incluye a los equipos que se registren
   * después, no sólo a los de ahora. */
  sitios: ReadonlySet<string>;
  /** Equipos sueltos, de unidades no marcadas completas. */
  equipos: ReadonlySet<string>;
}

/**
 * Lo que se le manda a `admin-enviar-push`. Un equipo marcado suelto dentro
 * de una unidad marcada completa no se repite (la unidad ya lo incluye).
 * `null` si no hay nadie elegido.
 */
export function destinoDeSeleccion(seleccion: SeleccionAviso, equipos: readonly EquipoAviso[]): DestinoAviso | null {
  if (seleccion.todos) return { todos: true };
  const sitioDe = new Map(equipos.map((equipo) => [equipo.id, equipo.sitio_id]));
  const sitio_ids = [...seleccion.sitios].sort();
  const dispositivo_ids = [...seleccion.equipos]
    .filter((id) => {
      const sitio = sitioDe.get(id);
      return sitio !== undefined && !seleccion.sitios.has(sitio);
    })
    .sort();
  if (sitio_ids.length === 0 && dispositivo_ids.length === 0) return null;
  return { sitio_ids, dispositivo_ids };
}

/**
 * A cuántos equipos llegaría el aviso ahora mismo: los elegidos que ya
 * registraron su token de notificaciones (`conNotificaciones`). Mismo
 * criterio que la Edge Function, que sólo manda a equipos vigentes con token.
 */
export function contarDestinatarios(
  seleccion: SeleccionAviso,
  equipos: readonly EquipoAviso[],
  conNotificaciones: ReadonlySet<string>,
): number {
  return equipos.filter(
    (equipo) =>
      conNotificaciones.has(equipo.id) &&
      (seleccion.todos || seleccion.sitios.has(equipo.sitio_id) || seleccion.equipos.has(equipo.id)),
  ).length;
}

/** "Llegará a 3 equipos" / "Llegará a 1 equipo" / "No llegará a ningún equipo". */
export function textoAlcance(cantidad: number): string {
  if (cantidad === 0) return "No llegará a ningún equipo";
  return `Llegará a ${cantidad} ${cantidad === 1 ? "equipo" : "equipos"}`;
}

/** Resumen del envío para el aviso de resultado. Nombra a los equipos a los
 * que no llegó: en una emergencia hay que avisarles por otro medio. */
export function textoResultado(resultado: {
  destinatarios: number;
  enviados: number;
  fallidos: number;
  no_recibieron: readonly string[];
}): string {
  if (resultado.destinatarios === 0) return "No había equipos con notificaciones en ese destino.";
  const enviados = `${resultado.enviados} de ${resultado.destinatarios} ${resultado.destinatarios === 1 ? "equipo" : "equipos"}`;
  if (resultado.fallidos === 0) return `Aviso enviado a ${enviados}.`;
  if (resultado.no_recibieron.length > 0) {
    return `Aviso enviado a ${enviados}; no llegó a ${resultado.no_recibieron.join(", ")}.`;
  }
  const fallidos = resultado.fallidos === 1 ? "1 no se pudo enviar" : `${resultado.fallidos} no se pudieron enviar`;
  return `Aviso enviado a ${enviados}; ${fallidos}.`;
}

/**
 * A quién se mandó un aviso del historial, con nombres: "Todos los equipos",
 * o las unidades y equipos sueltos. Lo que ya no existe (unidad borrada,
 * equipo retirado) se cuenta aparte en vez de mostrar un id.
 */
export function textoDestino(
  destino: DestinoAviso,
  sitios: ReadonlyMap<string, string>,
  equipos: ReadonlyMap<string, string>,
): string {
  if ("todos" in destino) return "Todos los equipos";
  const partes: string[] = [];
  let desconocidos = 0;
  for (const id of destino.sitio_ids) {
    const nombre = sitios.get(id);
    if (nombre) partes.push(nombre);
    else desconocidos++;
  }
  for (const id of destino.dispositivo_ids) {
    const nombre = equipos.get(id);
    if (nombre) partes.push(nombre);
    else desconocidos++;
  }
  if (desconocidos > 0) partes.push(desconocidos === 1 ? "1 que ya no existe" : `${desconocidos} que ya no existen`);
  return partes.join(", ");
}

/** "Borrado por ANA PÉREZ en Celular A1 (18/10/2026 14:32)". `momento` ya
 * viene escrito en la hora del navegador. */
export function textoAccion(
  accion: { accion: "archivado" | "borrado"; usuario: string; dispositivo_etiqueta: string },
  momento: string,
): string {
  const que = accion.accion === "borrado" ? "Borrado" : "Archivado";
  return `${que} por ${accion.usuario} en ${accion.dispositivo_etiqueta} (${momento})`;
}

/** "Llegó a 3 de 3" / "No llegó a ninguno (0 de 2)" / "Sin destinatarios". */
export function textoEntrega(aviso: { destinatarios: number; enviados: number }): string {
  if (aviso.destinatarios === 0) return "Sin destinatarios";
  if (aviso.enviados === 0) return `No llegó a ninguno (0 de ${aviso.destinatarios})`;
  return `Llegó a ${aviso.enviados} de ${aviso.destinatarios}`;
}
