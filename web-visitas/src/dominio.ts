import { z } from "./lib/validacion";
import { hoyCostaRica } from "./fecha";

export const MAX_VISITANTES = 50;
export const MAX_SITIOS = 100;
// Intencional: valida que NO haya caracteres de control (requisito de
// seguridad del contrato de backend, ver docs/auditorias/contrato-web-visitas.md
// "Rechazar caracteres de control") -- no es una regex mal escrita.
// eslint-disable-next-line no-control-regex
const sinControl = /^[^\u0000-\u001f\u007f]*$/u;
const texto = (maximo: number) =>
  z
    .string()
    .trim()
    .max(maximo, `Usá hasta ${maximo} caracteres.`)
    .regex(
      sinControl,
      "Ese texto tiene un carácter que no podemos guardar (por ejemplo, pegado desde otro programa). Borralo y escribilo de nuevo.",
    );
const opcional = (maximo: number) => texto(maximo).transform((v) => v || null);
// "HH:MM" de un <input type="time">. Vacío -> null (es opcional). El tipo de
// entrada se queda en `string` (no `string | null`) a propósito -- es lo que
// siempre entrega un <input> controlado, y `FormularioCita` (z.input) lo
// necesita así para que `value={formulario.hora_estimada}` tipe bien.
const horaOpcional = z
  .string()
  .refine((v) => v === "" || /^([01]\d|2[0-3]):[0-5]\d$/.test(v), {
    message: "Ingresá una hora válida (HH:MM).",
  })
  .transform((v) => (v === "" ? null : v));

export function normalizarDocumento(valor: string) {
  return valor.trim().replace(/[\s-]/g, "").toUpperCase();
}

const visitanteEntrada = z.object({
  nombre: texto(150).min(2, "Ingresá el nombre completo."),
  cedula: texto(60)
    .transform(normalizarDocumento)
    .pipe(
      z
        .string()
        .min(3, "Ingresá un documento válido.")
        .max(30, "El documento admite hasta 30 caracteres.")
        .regex(/^[A-Z0-9]+$/, "Usá letras, números, espacios o guiones."),
    ),
  empresa: opcional(150),
  placa_vehiculo: opcional(20).transform((v) => v?.toUpperCase() ?? null),
});

/** Reglas de fecha compartidas entre la validación final (`esquemaNuevaCita`)
 * y la validación en vivo de `componentes/CampoFechas.tsx` (al tipear o
 * elegir en el calendario) -- una sola fuente de verdad, sin duplicar las
 * reglas en dos lugares. */
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

export function esquemaNuevaCita(hoy = hoyCostaRica()) {
  return z
    .object({
      fecha_desde: z.iso.date("Seleccioná una fecha válida."),
      fecha_hasta: z.iso.date("Seleccioná una fecha válida."),
      hora_estimada: horaOpcional,
      motivo: opcional(1000),
      sitios: z
        .array(z.uuid())
        .min(1, "Seleccioná al menos un sitio.")
        .max(MAX_SITIOS),
      visitantes: z.array(visitanteEntrada).min(1).max(MAX_VISITANTES),
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
      if (new Set(datos.sitios).size !== datos.sitios.length)
        contexto.addIssue({
          code: "custom",
          path: ["sitios"],
          message: "Hay sitios repetidos.",
        });
      const documentos = new Set<string>();
      datos.visitantes.forEach((visitante, i) => {
        if (documentos.has(visitante.cedula))
          contexto.addIssue({
            code: "custom",
            path: ["visitantes", i, "cedula"],
            message: "Este documento ya está en la lista.",
          });
        documentos.add(visitante.cedula);
      });
    });
}

export type FormularioCita = z.input<ReturnType<typeof esquemaNuevaCita>>;
export type NuevaCita = z.output<ReturnType<typeof esquemaNuevaCita>>;
export const sitioEsquema = z.object({
  id: z.uuid(),
  nombre: z.string(),
});
export type Sitio = z.infer<typeof sitioEsquema>;
export const citaEsquema = z.object({
  id: z.uuid(),
  anfitrion_correo: z.email(),
  motivo: z.string().nullable(),
  fecha_desde: z.iso.date(),
  fecha_hasta: z.iso.date(),
  // "HH:MM:SS" tal cual la devuelve Postgres (columna `time`) -- sólo se
  // muestra, nunca se re-envía, así que no hace falta validar el formato acá.
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
  cita_sitios: z.array(
    z.object({ sitio_id: z.uuid(), sitios: sitioEsquema.nullable() }),
  ),
});
export type Cita = z.infer<typeof citaEsquema>;
export type FiltroEstado = "TODAS" | "VIGENTE" | "CANCELADA" | "VENCIDA";
export const visitanteVacio = (): FormularioCita["visitantes"][number] => ({
  nombre: "",
  cedula: "",
  empresa: "",
  placa_vehiculo: "",
});
