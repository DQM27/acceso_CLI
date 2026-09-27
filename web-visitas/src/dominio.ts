import { z } from "./lib/validacion";
import { hoyCostaRica } from "./fecha";

export const MAX_VISITANTES = 50;
export const MAX_SITIOS = 20;
// Intencional: valida que NO haya caracteres de control (mismo criterio que
// tenía el esquema viejo de citas) -- no es una regex mal escrita.
// eslint-disable-next-line no-control-regex
const sinControl = /^[^\u0000-\u001f\u007f]*$/u;
const texto = (maximo: number) =>
  z
    .string()
    .trim()
    .max(maximo, `Use hasta ${maximo} caracteres.`)
    .regex(
      sinControl,
      "Ese texto tiene un carácter que no podemos guardar (por ejemplo, pegado desde otro programa). Bórrelo y escríbalo de nuevo.",
    );
const opcional = (maximo: number) => texto(maximo).transform((v) => v || null);
const horaHHMM = z
  .string()
  .regex(/^([01]\d|2[0-3]):[0-5]\d$/, "Ingrese una hora válida (HH:MM).");

export function normalizarDocumento(valor: string) {
  return valor.trim().replace(/[\s-]/g, "").toUpperCase();
}

export const TIPOS_DOCUMENTO = ["CEDULA", "DIMEX", "PASAPORTE", "OTRO"] as const;
export type TipoDocumento = (typeof TIPOS_DOCUMENTO)[number];
export const ETIQUETAS_TIPO_DOCUMENTO: Record<TipoDocumento, string> = {
  CEDULA: "Cédula",
  DIMEX: "DIMEX",
  PASAPORTE: "Pasaporte",
  OTRO: "Otro documento",
};

const invitadoEntrada = z.object({
  tipo_documento: z.enum(TIPOS_DOCUMENTO),
  numero_documento: texto(30)
    .transform(normalizarDocumento)
    .pipe(
      z
        .string()
        .min(3, "Ingrese un documento válido.")
        .max(30, "El documento admite hasta 30 caracteres.")
        .regex(/^[A-Z0-9]+$/, "Use letras, números, espacios o guiones."),
    ),
  nombre: texto(150).min(2, "Ingrese el nombre completo."),
  empresa: opcional(150),
  telefono: opcional(30),
  correo: z
    .string()
    .trim()
    .transform((v) => (v === "" ? null : v))
    .pipe(z.email("Ingrese un correo válido.").nullable()),
  placa_vehiculo: opcional(20).transform((v) => v?.toUpperCase() ?? null),
});
export type InvitadoFormulario = z.input<typeof invitadoEntrada>;
export const invitadoVacio = (): InvitadoFormulario => ({
  tipo_documento: "CEDULA",
  numero_documento: "",
  nombre: "",
  empresa: "",
  telefono: "",
  correo: "",
  placa_vehiculo: "",
});

/** Reglas de fecha/hora compartidas entre la validación final y la de la
 * pantalla "Agendar visita" -- una sola fuente de verdad. */
export function validarRangoFechas(
  desde: string,
  hasta: string,
  hoy = hoyCostaRica(),
): { fecha_desde?: string; fecha_hasta?: string } {
  const errores: { fecha_desde?: string; fecha_hasta?: string } = {};
  if (desde < hoy)
    errores.fecha_desde = "La fecha de inicio no puede estar en el pasado.";
  if (hasta < desde)
    errores.fecha_hasta = "La fecha final debe ser igual o posterior al inicio.";
  return errores;
}

export function esquemaNuevaVisita(hoy = hoyCostaRica()) {
  return z
    .object({
      fecha_desde: z.iso.date("Seleccione una fecha válida."),
      fecha_hasta: z.iso.date("Seleccione una fecha válida."),
      hora_desde: horaHHMM,
      hora_hasta: horaHHMM,
      tipo_visita: opcional(80),
      motivo: opcional(1000),
      requiere_escolta: z.boolean(),
      sitios: z
        .array(z.uuid())
        .min(1, "Seleccione al menos un sitio.")
        .max(MAX_SITIOS),
      invitados: z.array(invitadoEntrada).min(1).max(MAX_VISITANTES),
    })
    .superRefine((datos, contexto) => {
      const erroresFecha = validarRangoFechas(
        datos.fecha_desde,
        datos.fecha_hasta,
        hoy,
      );
      if (erroresFecha.fecha_desde)
        contexto.addIssue({
          code: "custom",
          path: ["fecha_desde"],
          message: erroresFecha.fecha_desde,
        });
      if (erroresFecha.fecha_hasta)
        contexto.addIssue({
          code: "custom",
          path: ["fecha_hasta"],
          message: erroresFecha.fecha_hasta,
        });
      if (datos.hora_hasta <= datos.hora_desde)
        contexto.addIssue({
          code: "custom",
          path: ["hora_hasta"],
          message: "La hora de fin debe ser posterior a la de inicio.",
        });
      if (new Set(datos.sitios).size !== datos.sitios.length)
        contexto.addIssue({
          code: "custom",
          path: ["sitios"],
          message: "Hay sitios repetidos.",
        });
      const documentos = new Set<string>();
      datos.invitados.forEach((invitado, i) => {
        const clave = `${invitado.tipo_documento}|${invitado.numero_documento}`;
        if (documentos.has(clave))
          contexto.addIssue({
            code: "custom",
            path: ["invitados", i, "numero_documento"],
            message: "Este documento ya está en la lista.",
          });
        documentos.add(clave);
      });
    });
}
export type FormularioNuevaVisita = z.input<ReturnType<typeof esquemaNuevaVisita>>;
export type NuevaVisita = z.output<ReturnType<typeof esquemaNuevaVisita>>;

export const sitioEsquema = z.object({
  id: z.uuid(),
  nombre: z.string(),
});
export type Sitio = z.infer<typeof sitioEsquema>;

export const ESTADOS_INVITADO = [
  "PROGRAMADO",
  "EN_SITIO",
  "FUERA",
  "FINALIZADO",
  "CANCELADA",
  "NO_SE_PRESENTO",
  "SOLICITADO",
  "APROBADO",
  "RECHAZADO",
] as const;
export type EstadoInvitado = (typeof ESTADOS_INVITADO)[number];

/** Una fila de la vista `mis_visitas` -- una por invitado (ver
 * supabase/migrations/20260927150000_..._vista_busqueda.sql). */
export const misVisitasFilaEsquema = z.object({
  visita_id: z.uuid(),
  sitio_id: z.uuid(),
  sitio_nombre: z.string(),
  anfitrion_id: z.uuid(),
  tipo_visita: z.string().nullable(),
  motivo: z.string().nullable(),
  fecha_desde: z.iso.date(),
  fecha_hasta: z.iso.date(),
  hora_desde: z.string(),
  hora_hasta: z.string(),
  requiere_escolta: z.boolean(),
  grupo_id: z.uuid().nullable(),
  origen: z.enum(["PRE_REGISTRO", "WALK_IN"]),
  visita_estado: z.enum(["VIGENTE", "CANCELADA"]),
  invitado_id: z.uuid(),
  visitante_id: z.uuid(),
  tipo_documento: z.enum(TIPOS_DOCUMENTO),
  numero_documento: z.string(),
  visitante_nombre: z.string(),
  visitante_empresa: z.string().nullable(),
  placa_vehiculo: z.string().nullable(),
  invitado_estado: z.enum(ESTADOS_INVITADO),
  aprobado_por: z.string().nullable(),
  aprobado_en: z.string().nullable(),
  motivo_rechazo: z.string().nullable(),
  ultima_entrada: z.string().nullable(),
  ultima_salida: z.string().nullable(),
  ultimo_gafete_numero: z.number().nullable(),
});
export type MisVisitasFila = z.infer<typeof misVisitasFilaEsquema>;

export const visitanteAnteriorEsquema = z.object({
  id: z.uuid(),
  tipo_documento: z.enum(TIPOS_DOCUMENTO),
  numero_documento: z.string(),
  nombre: z.string(),
  empresa: z.string().nullable(),
});
export type VisitanteAnterior = z.infer<typeof visitanteAnteriorEsquema>;

/** Estado de una VISITA (no de un invitado), derivado de sus invitados --
 * no se persigue en la base (ver 3.2 del rediseño): "programada" mientras
 * el rango no arrancó, "en curso" si el rango ya empezó y no terminó,
 * "finalizada" si el rango ya pasó. */
export function estadoVisita(
  visita: { visita_estado: "VIGENTE" | "CANCELADA"; fecha_desde: string; fecha_hasta: string },
  hoy = hoyCostaRica(),
): "CANCELADA" | "PROGRAMADA" | "EN_CURSO" | "FINALIZADA" {
  if (visita.visita_estado === "CANCELADA") return "CANCELADA";
  if (hoy < visita.fecha_desde) return "PROGRAMADA";
  if (hoy > visita.fecha_hasta) return "FINALIZADA";
  return "EN_CURSO";
}

/** Agrupa las filas (una por invitado) de `mis_visitas` en visitas -- la UI
 * trabaja por visita (una tarjeta) con su lista de personas adentro. */
export interface VisitaAgrupada {
  visita_id: string;
  sitio_id: string;
  sitio_nombre: string;
  anfitrion_id: string;
  tipo_visita: string | null;
  motivo: string | null;
  fecha_desde: string;
  fecha_hasta: string;
  hora_desde: string;
  hora_hasta: string;
  requiere_escolta: boolean;
  grupo_id: string | null;
  origen: "PRE_REGISTRO" | "WALK_IN";
  visita_estado: "VIGENTE" | "CANCELADA";
  invitados: MisVisitasFila[];
}
export function agruparVisitas(filas: MisVisitasFila[]): VisitaAgrupada[] {
  const porVisita = new Map<string, VisitaAgrupada>();
  for (const fila of filas) {
    let visita = porVisita.get(fila.visita_id);
    if (!visita) {
      visita = {
        visita_id: fila.visita_id,
        sitio_id: fila.sitio_id,
        sitio_nombre: fila.sitio_nombre,
        anfitrion_id: fila.anfitrion_id,
        tipo_visita: fila.tipo_visita,
        motivo: fila.motivo,
        fecha_desde: fila.fecha_desde,
        fecha_hasta: fila.fecha_hasta,
        hora_desde: fila.hora_desde,
        hora_hasta: fila.hora_hasta,
        requiere_escolta: fila.requiere_escolta,
        grupo_id: fila.grupo_id,
        origen: fila.origen,
        visita_estado: fila.visita_estado,
        invitados: [],
      };
      porVisita.set(fila.visita_id, visita);
    }
    visita.invitados.push(fila);
  }
  return [...porVisita.values()];
}
