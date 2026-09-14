export function hoyCostaRica(instante = new Date()): string {
  const partes = new Intl.DateTimeFormat("en-CA", {
    timeZone: "America/Costa_Rica",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(instante);
  const valor = (tipo: Intl.DateTimeFormatPartTypes) => {
    const parte = partes.find((p) => p.type === tipo);
    if (!parte) throw new Error(`No se pudo formatear la fecha: falta la parte "${tipo}"`);
    return parte.value;
  };
  return `${valor("year")}-${valor("month")}-${valor("day")}`;
}

export function fechaLegible(fecha: string): string {
  return new Intl.DateTimeFormat("es-CR", {
    day: "numeric",
    month: "short",
    year: "numeric",
    timeZone: "UTC",
  }).format(new Date(`${fecha}T12:00:00Z`));
}

/** `hora` viene como "HH:MM" (de nuestro `<input type="time">`) o "HH:MM:SS"
 * (columna `time` de Postgres) -- sólo se usan las dos primeras partes. `null`
 * si no hay hora cargada (es opcional, puramente informativa). */
export function horaLegible(hora: string | null | undefined): string | null {
  if (!hora) return null;
  const [horas, minutos] = hora.split(":");
  const fecha = new Date(Date.UTC(2000, 0, 1, Number(horas), Number(minutos)));
  return new Intl.DateTimeFormat("es-CR", {
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
    timeZone: "UTC",
  }).format(fecha);
}

export function estadoCita(
  cita: { estado: "VIGENTE" | "CANCELADA"; fecha_hasta: string },
  hoy = hoyCostaRica(),
) {
  if (cita.estado === "CANCELADA") return "CANCELADA";
  return cita.fecha_hasta < hoy ? "VENCIDA" : "VIGENTE";
}
