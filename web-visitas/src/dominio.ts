import { z } from "./lib/validacion";
import { hoyCostaRica } from "./fecha";

export const MAX_VISITANTES = 50;
export const MAX_SITIOS = 100;
// Intencional: valida que NO haya caracteres de control (requisito de
// seguridad del contrato de backend, ver docs/contrato-web-visitas.md
// "Rechazar caracteres de control") -- no es una regex mal escrita.
// eslint-disable-next-line no-control-regex
const sinControl = /^[^\u0000-\u001f\u007f]*$/u;
const texto = (maximo: number) =>
  z
    .string()
    .trim()
    .max(maximo, `Usá hasta ${maximo} caracteres.`)
    .regex(sinControl, "El texto contiene caracteres no permitidos.");
const opcional = (maximo: number) => texto(maximo).transform((v) => v || null);

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

export function esquemaNuevaCita(hoy = hoyCostaRica()) {
  return z
    .object({
      fecha_desde: z.iso.date("Seleccioná una fecha válida."),
      fecha_hasta: z.iso.date("Seleccioná una fecha válida."),
      motivo: opcional(1000),
      sitios: z
        .array(z.uuid())
        .min(1, "Seleccioná al menos un sitio.")
        .max(MAX_SITIOS),
      visitantes: z.array(visitanteEntrada).min(1).max(MAX_VISITANTES),
    })
    .superRefine((datos, contexto) => {
      if (datos.fecha_desde < hoy)
        contexto.addIssue({
          code: "custom",
          path: ["fecha_desde"],
          message: "La fecha de inicio no puede estar en el pasado.",
        });
      if (datos.fecha_hasta < datos.fecha_desde)
        contexto.addIssue({
          code: "custom",
          path: ["fecha_hasta"],
          message: "La fecha final debe ser igual o posterior al inicio.",
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
  direccion: z.string().nullable(),
});
export type Sitio = z.infer<typeof sitioEsquema>;
export const citaEsquema = z.object({
  id: z.uuid(),
  anfitrion_correo: z.email(),
  motivo: z.string().nullable(),
  fecha_desde: z.iso.date(),
  fecha_hasta: z.iso.date(),
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
