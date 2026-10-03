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

/** Resumen del envío para el aviso de resultado. */
export function textoResultado(resultado: { destinatarios: number; enviados: number; fallidos: number }): string {
  if (resultado.destinatarios === 0) return "No había equipos con notificaciones en ese destino.";
  const enviados = `${resultado.enviados} de ${resultado.destinatarios} ${resultado.destinatarios === 1 ? "equipo" : "equipos"}`;
  if (resultado.fallidos === 0) return `Aviso enviado a ${enviados}.`;
  const fallidos = resultado.fallidos === 1 ? "1 no se pudo enviar" : `${resultado.fallidos} no se pudieron enviar`;
  return `Aviso enviado a ${enviados}; ${fallidos}.`;
}
