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

/** "AAAA-MM-DD" más `dias` días (calendario, sin husos horarios). */
export function sumarDias(fecha: string, dias: number): string {
  const d = new Date(`${fecha}T12:00:00Z`);
  d.setUTCDate(d.getUTCDate() + dias);
  return d.toISOString().slice(0, 10);
}

const formato = (opciones: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat("es-CR", { ...opciones, timeZone: "UTC" });

/** "5 oct 2026" */
export function fechaLegible(fecha: string): string {
  return formato({ day: "numeric", month: "short", year: "numeric" }).format(new Date(`${fecha}T12:00:00Z`));
}

/** "lunes 5 de octubre" */
export function fechaLarga(fecha: string): string {
  return formato({ weekday: "long", day: "numeric", month: "long" }).format(new Date(`${fecha}T12:00:00Z`));
}

/** "mar 6 oct" */
export function fechaCorta(fecha: string): string {
  return formato({ weekday: "short", day: "numeric", month: "short" })
    .format(new Date(`${fecha}T12:00:00Z`))
    .replace(/[.,]/g, "");
}

/** { dia: "7", mes: "OCT" } para el bloque de fecha de la lista. */
export function diaYMes(fecha: string): { dia: string; mes: string } {
  const d = new Date(`${fecha}T12:00:00Z`);
  return {
    dia: String(d.getUTCDate()),
    mes: formato({ month: "short" }).format(d).replace(".", "").toUpperCase(),
  };
}

/** Rango legible: "5 oct" o "12 al 14 oct". */
export function rangoLegible(desde: string, hasta: string): string {
  if (desde === hasta) return fechaCorta(desde);
  return `${fechaCorta(desde)} al ${fechaCorta(hasta)}`;
}

/** `hora` "HH:MM" o "HH:MM:SS" (columna `time`) → "9:00". 24 h, como el
 * resto del sistema. `null` si no hay hora (es opcional). */
export function horaLegible(hora: string | null | undefined): string | null {
  if (!hora) return null;
  const [horas, minutos] = hora.split(":");
  return `${Number(horas)}:${minutos}`;
}

/** Hora local de Costa Rica de un instante ISO ("9:12"). */
export function horaDeInstante(iso: string): string {
  const partes = new Intl.DateTimeFormat("en-GB", {
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
    timeZone: "America/Costa_Rica",
  }).formatToParts(new Date(iso));
  const valor = (tipo: Intl.DateTimeFormatPartTypes) => partes.find((p) => p.type === tipo)?.value ?? "00";
  // Igual que `horaLegible`: "9:12", no "09:12".
  return `${Number(valor("hour"))}:${valor("minute")}`;
}

export function estadoCita(cita: { estado: "VIGENTE" | "CANCELADA"; fecha_hasta: string }, hoy = hoyCostaRica()) {
  if (cita.estado === "CANCELADA") return "CANCELADA";
  return cita.fecha_hasta < hoy ? "VENCIDA" : "VIGENTE";
}
