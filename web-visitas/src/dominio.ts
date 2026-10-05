import { z } from "./lib/validacion";
import { hoyCostaRica } from "./fecha";
import { reglas } from "./reglas";
import type { CitaValidaReglas } from "./reglas";

/** Los mismos límites que `reglas/src/cita.rs` (lo comprueba
 * `pruebas/reglas.test.ts`): acá sólo deciden cuándo ocultar "Agregar
 * persona" y cuántos lugares pedir. */
export const MAX_VISITANTES = 50;
export const MAX_SITIOS = 100;
/** Lo que se escribe en el formulario, tal cual. */
export interface VisitanteFormulario {
  nombre: string;
  cedula: string;
  empresa: string;
  placa_vehiculo: string;
}

export interface FormularioCita {
  fecha_desde: string;
  fecha_hasta: string;
  /** "HH:MM" de un <input type="time">; vacía = sin hora. */
  hora_estimada: string;
  motivo: string;
  sitios: string[];
  visitantes: VisitanteFormulario[];
}

/** La cita ya validada y normalizada por el núcleo, lista para la base. */
export type DatosCita = CitaValidaReglas;

export type ResultadoValidacion =
  | { ok: true; datos: DatosCita }
  | { ok: false; errores: Record<string, string> };

/** Todas las reglas de una cita nueva, del núcleo (WebAssembly,
 * `reglas/src/cita.rs`): fechas en hora de Costa Rica, límites, cédula en su
 * forma única y repetidas aunque estén escritas distinto, placa y textos.
 * `errores` va por campo ("fecha_desde", "visitantes.1.cedula"...); si un
 * campo tiene más de un problema, se muestra el primero. */
export function validarCita(formulario: FormularioCita, hoy = hoyCostaRica()): ResultadoValidacion {
  const resultado = reglas.validarCita(formulario, hoy);
  if (resultado.ok) return { ok: true, datos: resultado.cita };
  const errores: Record<string, string> = {};
  for (const { campo, mensaje } of resultado.errores) errores[campo] ??= mensaje;
  return { ok: false, errores };
}

/** Error de validación para quien llama a la API sin pasar por el formulario. */
export class CitaInvalida extends Error {
  readonly errores: Record<string, string>;

  constructor(errores: Record<string, string>) {
    super(Object.values(errores)[0] ?? "Revise los datos de la visita.");
    this.name = "CitaInvalida";
    this.errores = errores;
  }
}

/** Documento en la forma única del núcleo (la que reconoce la portería). Si
 * las reglas todavía no cargaron, una aproximación: sin separadores y en
 * mayúsculas (sólo se usa para no sumar dos veces a la misma persona). */
export function normalizarDocumento(valor: string): string {
  return reglas.normalizarDocumento(valor) ?? valor.trim().replace(/[\s.-]/g, "").toUpperCase();
}

export const sitioEsquema = z.object({ id: z.uuid(), nombre: z.string() });
export type Sitio = z.infer<typeof sitioEsquema>;

export const citaEsquema = z.object({
  id: z.uuid(),
  anfitrion_correo: z.email(),
  motivo: z.string().nullable(),
  fecha_desde: z.iso.date(),
  fecha_hasta: z.iso.date(),
  // "HH:MM:SS" tal cual la devuelve Postgres (columna `time`).
  hora_estimada: z.string().nullable(),
  estado: z.enum(["VIGENTE", "CANCELADA"]),
  created_at: z.string(),
  cita_visitantes: z.array(
    z.object({
      id: z.uuid(),
      nombre: z.string(),
      cedula: z.string(),
      empresa: z.string().nullable(),
      placa_vehiculo: z.string().nullable(),
    }),
  ),
  cita_sitios: z.array(z.object({ sitio_id: z.uuid(), sitios: sitioEsquema.nullable() })),
});
export type Cita = z.infer<typeof citaEsquema>;

/** Último movimiento de un visitante en la portería
 * (`estado_visitantes_de_mis_citas`). */
export const llegadaEsquema = z.object({
  cita_visitante_id: z.uuid(),
  sitio_nombre: z.string(),
  hora_entrada: z.string(),
  hora_salida: z.string().nullable(),
  gafete_numero: z.number().nullable(),
});
export type Llegada = z.infer<typeof llegadaEsquema>;

/** Persona que el anfitrión ya agendó antes (`visitantes_anteriores`). */
export const visitanteAnteriorEsquema = z.object({
  cedula: z.string(),
  nombre: z.string(),
  empresa: z.string().nullable(),
  placa_vehiculo: z.string().nullable(),
  ultima_vez: z.iso.date(),
});
export type VisitanteAnterior = z.infer<typeof visitanteAnteriorEsquema>;

export type EstadoVisitante =
  | { tipo: "espera" }
  | { tipo: "adentro"; desde: string; gafete: number | null; sitio: string }
  | { tipo: "salio"; hora: string; sitio: string };

export function estadoVisitante(llegada: Llegada | undefined): EstadoVisitante {
  if (!llegada) return { tipo: "espera" };
  if (llegada.hora_salida) return { tipo: "salio", hora: llegada.hora_salida, sitio: llegada.sitio_nombre };
  return { tipo: "adentro", desde: llegada.hora_entrada, gafete: llegada.gafete_numero, sitio: llegada.sitio_nombre };
}

/** Separa las citas vigentes en "Hoy" (hoy cae dentro de su rango) y
 * "Próximas" (empiezan después), cada grupo ordenado por fecha y hora. Las
 * vencidas y canceladas van al historial. */
export function agruparCitas(citas: Cita[], hoy = hoyCostaRica()) {
  const orden = (a: Cita, b: Cita) =>
    a.fecha_desde.localeCompare(b.fecha_desde) || (a.hora_estimada ?? "99").localeCompare(b.hora_estimada ?? "99");
  const vigentes = citas.filter((c) => c.estado === "VIGENTE" && c.fecha_hasta >= hoy);
  return {
    hoy: vigentes.filter((c) => c.fecha_desde <= hoy).sort(orden),
    proximas: vigentes.filter((c) => c.fecha_desde > hoy).sort(orden),
  };
}

/** Título de una cita para la lista: el motivo, o quién viene. */
export function tituloCita(cita: Pick<Cita, "motivo" | "cita_visitantes">) {
  if (cita.motivo) return cita.motivo;
  const personas = cita.cita_visitantes;
  if (personas.length === 1) return `Visita de ${personas[0].nombre}`;
  return `Visita de ${personas.length} personas`;
}

export const visitanteVacio = (): VisitanteFormulario => ({
  nombre: "",
  cedula: "",
  empresa: "",
  placa_vehiculo: "",
});

/** Valores del formulario a partir de una cita guardada (editar o duplicar). */
export function formularioDesdeCita(cita: Cita, hoy = hoyCostaRica()): FormularioCita {
  const desde = cita.fecha_desde < hoy ? hoy : cita.fecha_desde;
  return {
    fecha_desde: desde,
    fecha_hasta: cita.fecha_hasta < desde ? desde : cita.fecha_hasta,
    hora_estimada: cita.hora_estimada ? cita.hora_estimada.slice(0, 5) : "",
    motivo: cita.motivo ?? "",
    sitios: cita.cita_sitios.map((s) => s.sitio_id),
    visitantes: cita.cita_visitantes.map((v) => ({
      nombre: v.nombre,
      cedula: v.cedula,
      empresa: v.empresa ?? "",
      placa_vehiculo: v.placa_vehiculo ?? "",
    })),
  };
}
