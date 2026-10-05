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
    .max(maximo, `Use hasta ${maximo} caracteres.`)
    .regex(
      sinControl,
      "Ese texto tiene un carácter que no se puede guardar (por ejemplo, pegado desde otro programa). Bórrelo y escríbalo de nuevo.",
    );
const opcional = (maximo: number) => texto(maximo).transform((v) => v || null);
// "HH:MM" de un <input type="time">. Vacío -> null (es opcional).
const horaOpcional = z
  .string()
  .refine((v) => v === "" || /^([01]\d|2[0-3]):[0-5]\d$/.test(v), {
    message: "Ingrese una hora válida (HH:MM).",
  })
  .transform((v) => (v === "" ? null : v));

/** Sin espacios ni guiones, en mayúsculas. La forma única definitiva (el cero
 * del TSE, etc.) la pone la base con `normalizar_cedula`, igual que el núcleo. */
export function normalizarDocumento(valor: string) {
  return valor.trim().replace(/[\s.-]/g, "").toUpperCase();
}

const visitanteEntrada = z.object({
  nombre: texto(150).min(2, "Ingrese el nombre completo."),
  cedula: texto(60)
    .transform(normalizarDocumento)
    .pipe(
      z
        .string()
        .min(3, "Ingrese un documento válido.")
        .max(30, "El documento admite hasta 30 caracteres.")
        .regex(/^[A-Z0-9]+$/, "Use letras y números (los espacios y guiones se quitan solos)."),
    ),
  empresa: opcional(150),
  placa_vehiculo: opcional(20).transform((v) => v?.toUpperCase() ?? null),
});

/** Reglas de fecha de la cita: no en el pasado, y el fin no antes del inicio. */
export function validarRangoFechas(
  desde: string,
  hasta: string,
  hoy = hoyCostaRica(),
): { fecha_desde?: string; fecha_hasta?: string } {
  const errores: { fecha_desde?: string; fecha_hasta?: string } = {};
  if (desde < hoy) errores.fecha_desde = "La fecha no puede estar en el pasado.";
  if (hasta < desde) errores.fecha_hasta = "La fecha final debe ser igual o posterior al inicio.";
  return errores;
}

export function esquemaCita(hoy = hoyCostaRica()) {
  return z
    .object({
      fecha_desde: z.iso.date("Elija una fecha válida."),
      fecha_hasta: z.iso.date("Elija una fecha válida."),
      hora_estimada: horaOpcional,
      motivo: opcional(1000),
      sitios: z.array(z.uuid()).min(1, "Elija al menos un lugar.").max(MAX_SITIOS),
      visitantes: z.array(visitanteEntrada).min(1, "Agregue al menos una persona.").max(MAX_VISITANTES),
    })
    .superRefine((datos, contexto) => {
      const erroresFecha = validarRangoFechas(datos.fecha_desde, datos.fecha_hasta, hoy);
      if (erroresFecha.fecha_desde)
        contexto.addIssue({ code: "custom", path: ["fecha_desde"], message: erroresFecha.fecha_desde });
      if (erroresFecha.fecha_hasta)
        contexto.addIssue({ code: "custom", path: ["fecha_hasta"], message: erroresFecha.fecha_hasta });
      if (new Set(datos.sitios).size !== datos.sitios.length)
        contexto.addIssue({ code: "custom", path: ["sitios"], message: "Hay lugares repetidos." });
      const documentos = new Set<string>();
      datos.visitantes.forEach((visitante, i) => {
        if (documentos.has(visitante.cedula))
          contexto.addIssue({
            code: "custom",
            path: ["visitantes", i, "cedula"],
            message: "Esta persona ya está en la lista.",
          });
        documentos.add(visitante.cedula);
      });
    });
}

export type FormularioCita = z.input<ReturnType<typeof esquemaCita>>;
export type DatosCita = z.output<ReturnType<typeof esquemaCita>>;
export type VisitanteFormulario = FormularioCita["visitantes"][number];

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
