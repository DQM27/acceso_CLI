export function hoyCostaRica(instante = new Date()): string {
  const partes = new Intl.DateTimeFormat("en-CA", {
    timeZone: "America/Costa_Rica",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(instante);
  const valor = (tipo: Intl.DateTimeFormatPartTypes) =>
    partes.find((p) => p.type === tipo)!.value;
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

export function estadoCita(
  cita: { estado: "VIGENTE" | "CANCELADA"; fecha_hasta: string },
  hoy = hoyCostaRica(),
) {
  if (cita.estado === "CANCELADA") return "CANCELADA";
  return cita.fecha_hasta < hoy ? "VENCIDA" : "VIGENTE";
}
