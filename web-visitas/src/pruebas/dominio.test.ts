import { describe, expect, it } from "vitest";
import { agruparCitas, estadoVisitante, formularioDesdeCita, normalizarDocumento, tituloCita, validarCita } from "../dominio";
import type { Cita, FormularioCita } from "../dominio";
import { estadoCita, horaLegible, hoyCostaRica, rangoLegible, sumarDias } from "../fecha";

const datos = (): FormularioCita => ({
  fecha_desde: "2026-09-09",
  fecha_hasta: "2026-09-10",
  hora_estimada: "",
  motivo: "",
  sitios: ["00000000-0000-4000-8000-000000000001"],
  visitantes: [
    {
      nombre: "Persona de prueba",
      cedula: "1-2345-6789",
      empresa: "",
      placa_vehiculo: "",
    },
  ],
});
describe("validación de citas (reglas del núcleo)", () => {
  const valida = (entrada: FormularioCita, hoy = "2026-09-09") => {
    const resultado = validarCita(entrada, hoy);
    if (!resultado.ok) throw new Error(JSON.stringify(resultado.errores));
    return resultado.datos;
  };
  const errores = (entrada: FormularioCita, hoy = "2026-09-09") => {
    const resultado = validarCita(entrada, hoy);
    return resultado.ok ? {} : resultado.errores;
  };

  it("normaliza documentos y campos opcionales", () => {
    const resultado = valida(datos());
    expect(resultado.visitantes[0]).toMatchObject({
      cedula: "123456789",
      empresa: null,
      placa_vehiculo: null,
    });
    expect(resultado.motivo).toBeNull();
    expect(normalizarDocumento(" ab-123 ")).toBe("AB123");
  });
  it("la cédula sale igual que en el check-in de la portería (cero del TSE incluido)", () => {
    expect(normalizarDocumento("01-0847-0293")).toBe("108470293");
    expect(normalizarDocumento("1 0847 0293")).toBe("108470293");
  });
  it("rechaza documentos duplicados aunque se escriban con separadores distintos", () => {
    const entrada = datos();
    const [primero] = entrada.visitantes;
    if (!primero) throw new Error("fixture sin visitantes");
    entrada.visitantes.push({ ...primero, cedula: "123456789" });
    expect(errores(entrada)).toEqual({ "visitantes.1.cedula": "Esta persona ya está en la lista." });
  });
  it("detecta el duplicado aunque una cédula lleve el cero del TSE", () => {
    const entrada = datos();
    const [primero] = entrada.visitantes;
    if (!primero) throw new Error("fixture sin visitantes");
    entrada.visitantes = [
      { ...primero, cedula: "01-0847-0293" },
      { ...primero, cedula: "108470293" },
    ];
    expect(errores(entrada)).toHaveProperty(["visitantes.1.cedula"]);
  });
  it("un documento de más de 20 caracteres no se agenda (la portería no lo reconocería)", () => {
    const entrada = datos();
    const [primero] = entrada.visitantes;
    if (!primero) throw new Error("fixture sin visitantes");
    entrada.visitantes = [{ ...primero, cedula: "A".repeat(21) }];
    expect(errores(entrada)).toEqual({ "visitantes.0.cedula": "El documento admite hasta 20 caracteres." });
    entrada.visitantes = [{ ...primero, cedula: "A".repeat(20) }];
    expect(valida(entrada).visitantes[0]?.cedula).toBe("A".repeat(20));
  });
  it.each([
    { fecha_desde: "2026-09-08" },
    { fecha_hasta: "2026-09-08" },
    { fecha_desde: "2026-02-30" },
    { sitios: [] },
    { visitantes: [] },
    { motivo: "x".repeat(1001) },
    { motivo: "texto\u0000oculto" },
  ])("rechaza rangos, cantidades o textos inválidos: %j", (cambio) => {
    expect(validarCita({ ...datos(), ...cambio }, "2026-09-09").ok).toBe(false);
  });
  it("devuelve todos los errores, uno por campo, con su ruta", () => {
    const entrada = { ...datos(), fecha_desde: "2026-09-08", sitios: [] };
    const [primero] = entrada.visitantes;
    if (!primero) throw new Error("fixture sin visitantes");
    entrada.visitantes = [{ ...primero, nombre: "" }];
    expect(Object.keys(errores(entrada)).sort()).toEqual(["fecha_desde", "sitios", "visitantes.0.nombre"]);
  });
  it("hora_estimada: vacía se guarda como null, un formato válido se conserva", () => {
    expect(valida(datos()).hora_estimada).toBeNull();
    expect(valida({ ...datos(), hora_estimada: "10:00" }).hora_estimada).toBe("10:00");
  });
  it("rechaza una hora_estimada con formato inválido", () => {
    expect(errores({ ...datos(), hora_estimada: "25:99" })).toHaveProperty("hora_estimada");
  });
  it("acepta una visita de un solo día y rechaza un grupo sin límite", () => {
    const entrada = datos();
    entrada.fecha_hasta = entrada.fecha_desde;
    expect(validarCita(entrada, "2026-09-09").ok).toBe(true);
    const [primero] = entrada.visitantes;
    if (!primero) throw new Error("fixture sin visitantes");
    entrada.visitantes = Array.from({ length: 51 }, (_, i) => ({
      ...primero,
      cedula: `DOC${i}`,
    }));
    expect(errores(entrada)).toHaveProperty("visitantes");
  });
});
describe("fechas de Costa Rica", () => {
  it("no vence una cita al cambiar el día en UTC", () => {
    expect(hoyCostaRica(new Date("2026-09-10T05:59:59Z"))).toBe("2026-09-09");
    expect(hoyCostaRica(new Date("2026-09-10T06:00:00Z"))).toBe("2026-09-10");
    expect(
      estadoCita(
        { estado: "VIGENTE", fecha_hasta: "2026-09-09" },
        "2026-09-09",
      ),
    ).toBe("VIGENTE");
    expect(
      estadoCita(
        { estado: "VIGENTE", fecha_hasta: "2026-09-09" },
        "2026-09-10",
      ),
    ).toBe("VENCIDA");
  });
  it("una cita cancelada conserva ese estado después de vencer", () => {
    expect(
      estadoCita(
        { estado: "CANCELADA", fecha_hasta: "2026-09-09" },
        "2026-09-10",
      ),
    ).toBe("CANCELADA");
  });
});

const cita = (cambios: Partial<Cita> = {}): Cita => ({
  id: "00000000-0000-4000-8000-0000000000aa",
  anfitrion_correo: "prueba@example.invalid",
  motivo: null,
  fecha_desde: "2026-10-05",
  fecha_hasta: "2026-10-05",
  hora_estimada: null,
  estado: "VIGENTE",
  created_at: "2026-10-01T15:00:00Z",
  cita_visitantes: [
    { id: "00000000-0000-4000-8000-0000000000b1", nombre: "Ana Mora", cedula: "123456789", empresa: null, placa_vehiculo: null },
  ],
  cita_sitios: [{ sitio_id: "00000000-0000-4000-8000-000000000001", sitios: null }],
  ...cambios,
});

describe("agrupación de la pantalla principal", () => {
  it("hoy incluye las de varios días en curso; próximas las futuras; fuera canceladas y vencidas", () => {
    const enCurso = cita({ id: "00000000-0000-4000-8000-000000000011", fecha_desde: "2026-10-03", fecha_hasta: "2026-10-07" });
    const tarde = cita({ id: "00000000-0000-4000-8000-000000000012", hora_estimada: "15:00:00" });
    const temprano = cita({ id: "00000000-0000-4000-8000-000000000013", hora_estimada: "08:30:00" });
    const futura = cita({ id: "00000000-0000-4000-8000-000000000014", fecha_desde: "2026-10-09", fecha_hasta: "2026-10-09" });
    const cancelada = cita({ id: "00000000-0000-4000-8000-000000000015", estado: "CANCELADA" });
    const vencida = cita({ id: "00000000-0000-4000-8000-000000000016", fecha_desde: "2026-10-01", fecha_hasta: "2026-10-04" });
    const grupos = agruparCitas([futura, tarde, cancelada, temprano, vencida, enCurso], "2026-10-05");
    expect(grupos.hoy.map((c) => c.id)).toEqual([enCurso.id, temprano.id, tarde.id]);
    expect(grupos.proximas.map((c) => c.id)).toEqual([futura.id]);
  });
});

describe("estado de cada persona en la portería", () => {
  const llegada = {
    cita_visitante_id: "00000000-0000-4000-8000-0000000000b1",
    sitio_nombre: "Brisas",
    hora_entrada: "2026-10-05T15:12:00Z",
    hora_salida: null,
    gafete_numero: 7,
  };
  it("sin movimiento espera; con entrada está adentro; con salida salió", () => {
    expect(estadoVisitante(undefined)).toEqual({ tipo: "espera" });
    expect(estadoVisitante(llegada)).toEqual({ tipo: "adentro", desde: llegada.hora_entrada, gafete: 7, sitio: "Brisas" });
    expect(estadoVisitante({ ...llegada, hora_salida: "2026-10-05T17:00:00Z" })).toEqual({
      tipo: "salio",
      hora: "2026-10-05T17:00:00Z",
      sitio: "Brisas",
    });
  });
});

describe("editar y duplicar", () => {
  it("una cita pasada se duplica desde hoy, conservando personas, lugares y hora", () => {
    const vieja = cita({
      fecha_desde: "2026-09-01",
      fecha_hasta: "2026-09-02",
      hora_estimada: "09:30:00",
      motivo: "Mantenimiento",
      cita_visitantes: [
        { id: "00000000-0000-4000-8000-0000000000b1", nombre: "Ana Mora", cedula: "123456789", empresa: "ACME", placa_vehiculo: null },
      ],
    });
    const formulario = formularioDesdeCita(vieja, "2026-10-05");
    expect(formulario).toEqual({
      fecha_desde: "2026-10-05",
      fecha_hasta: "2026-10-05",
      hora_estimada: "09:30",
      motivo: "Mantenimiento",
      sitios: ["00000000-0000-4000-8000-000000000001"],
      visitantes: [{ nombre: "Ana Mora", cedula: "123456789", empresa: "ACME", placa_vehiculo: "" }],
    });
    expect(validarCita(formulario, "2026-10-05").ok).toBe(true);
  });
  it("una cita en curso conserva su fecha final", () => {
    const formulario = formularioDesdeCita(cita({ fecha_desde: "2026-10-03", fecha_hasta: "2026-10-08" }), "2026-10-05");
    expect([formulario.fecha_desde, formulario.fecha_hasta]).toEqual(["2026-10-05", "2026-10-08"]);
  });
  it("el título es el motivo o quién viene", () => {
    expect(tituloCita(cita({ motivo: "Auditoría" }))).toBe("Auditoría");
    expect(tituloCita(cita())).toBe("Visita de Ana Mora");
    const dos = cita().cita_visitantes.concat({ ...cita().cita_visitantes[0], id: "00000000-0000-4000-8000-0000000000b2" });
    expect(tituloCita(cita({ cita_visitantes: dos }))).toBe("Visita de 2 personas");
  });
});

describe("formatos de fecha y hora", () => {
  it("horas en 24 h y sin hora cuando no se cargó", () => {
    expect(horaLegible("09:00:00")).toBe("9:00");
    expect(horaLegible("14:30")).toBe("14:30");
    expect(horaLegible(null)).toBeNull();
  });
  it("suma días cruzando meses y arma rangos", () => {
    expect(sumarDias("2026-10-31", 1)).toBe("2026-11-01");
    expect(sumarDias("2026-03-01", -1)).toBe("2026-02-28");
    expect(rangoLegible("2026-10-05", "2026-10-05")).not.toContain(" al ");
    expect(rangoLegible("2026-10-05", "2026-10-07")).toContain(" al ");
  });
});

describe("hora de llegada", () => {
  it("se muestra en hora de Costa Rica, 24 h y sin cero adelante", async () => {
    const { horaDeInstante } = await import("../fecha");
    expect(horaDeInstante("2026-10-05T15:12:00Z")).toBe("9:12");
    expect(horaDeInstante("2026-10-05T20:05:00Z")).toBe("14:05");
  });
});
