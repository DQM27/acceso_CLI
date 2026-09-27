import { describe, expect, it } from "vitest";
import {
  agruparVisitas,
  esquemaNuevaVisita,
  estadoVisita,
  normalizarDocumento,
} from "../dominio";
import type { MisVisitasFila } from "../dominio";
import { hoyCostaRica } from "../fecha";

const datos = () => ({
  fecha_desde: "2026-09-09",
  fecha_hasta: "2026-09-10",
  hora_desde: "08:00",
  hora_hasta: "17:00",
  tipo_visita: "",
  motivo: "",
  requiere_escolta: false,
  sitios: ["00000000-0000-4000-8000-000000000001"],
  invitados: [
    {
      tipo_documento: "CEDULA" as const,
      numero_documento: "1-2345-6789",
      nombre: "Persona de prueba",
      empresa: "",
      telefono: "",
      correo: "",
      placa_vehiculo: "",
    },
  ],
});

describe("validación de visitas", () => {
  it("normaliza documentos y campos opcionales", () => {
    const resultado = esquemaNuevaVisita("2026-09-09").parse(datos());
    expect(resultado.invitados[0]).toMatchObject({
      numero_documento: "123456789",
      empresa: null,
      placa_vehiculo: null,
      correo: null,
    });
    expect(resultado.motivo).toBeNull();
    expect(normalizarDocumento(" ab-123 ")).toBe("AB123");
  });

  it("rechaza documentos duplicados dentro del mismo grupo", () => {
    const entrada = datos();
    const [primero] = entrada.invitados;
    if (!primero) throw new Error("fixture sin invitados");
    entrada.invitados.push({ ...primero, numero_documento: "123456789" });
    const resultado = esquemaNuevaVisita("2026-09-09").safeParse(entrada);
    expect(resultado.success).toBe(false);
    if (!resultado.success)
      expect(resultado.error.issues[0]?.path).toEqual([
        "invitados",
        1,
        "numero_documento",
      ]);
  });

  it.each([
    { fecha_desde: "2026-09-08" },
    { fecha_hasta: "2026-09-08" },
    { hora_hasta: "08:00" },
    { sitios: [] },
    { invitados: [] },
    { motivo: "x".repeat(1001) },
    { motivo: "texto\u0000oculto" },
  ])("rechaza rangos, cantidades o textos inválidos: %j", (cambio) => {
    expect(
      esquemaNuevaVisita("2026-09-09").safeParse({ ...datos(), ...cambio })
        .success,
    ).toBe(false);
  });

  it("acepta una visita de un solo día y rechaza un grupo sin límite", () => {
    const entrada = datos();
    entrada.fecha_hasta = entrada.fecha_desde;
    expect(esquemaNuevaVisita("2026-09-09").safeParse(entrada).success).toBe(true);
    const [primero] = entrada.invitados;
    if (!primero) throw new Error("fixture sin invitados");
    entrada.invitados = Array.from({ length: 51 }, (_, i) => ({
      ...primero,
      numero_documento: `DOC${i}`,
    }));
    expect(esquemaNuevaVisita("2026-09-09").safeParse(entrada).success).toBe(false);
  });
});

function filaBase(sobrescribe: Partial<MisVisitasFila> = {}): MisVisitasFila {
  return {
    visita_id: "00000000-0000-4000-8000-000000000010",
    sitio_id: "00000000-0000-4000-8000-000000000001",
    sitio_nombre: "Brisas",
    anfitrion_id: "00000000-0000-4000-8000-000000000099",
    tipo_visita: null,
    motivo: null,
    fecha_desde: "2026-09-09",
    fecha_hasta: "2026-09-09",
    hora_desde: "08:00:00",
    hora_hasta: "17:00:00",
    requiere_escolta: false,
    grupo_id: null,
    origen: "PRE_REGISTRO",
    visita_estado: "VIGENTE",
    invitado_id: "00000000-0000-4000-8000-000000000020",
    visitante_id: "00000000-0000-4000-8000-000000000030",
    tipo_documento: "CEDULA",
    numero_documento: "123456789",
    visitante_nombre: "Persona de prueba",
    visitante_empresa: null,
    placa_vehiculo: null,
    invitado_estado: "PROGRAMADO",
    aprobado_por: null,
    aprobado_en: null,
    motivo_rechazo: null,
    ultima_entrada: null,
    ultima_salida: null,
    ultimo_gafete_numero: null,
    ...sobrescribe,
  };
}

describe("estado de una visita (derivado, no persistido)", () => {
  it("una visita cancelada lo es sin importar las fechas", () => {
    expect(estadoVisita(filaBase({ visita_estado: "CANCELADA" }), "2026-09-09")).toBe("CANCELADA");
  });
  it("programada antes de empezar, en curso durante, finalizada después", () => {
    const visita = filaBase({ fecha_desde: "2026-09-09", fecha_hasta: "2026-09-10" });
    expect(estadoVisita(visita, "2026-09-08")).toBe("PROGRAMADA");
    expect(estadoVisita(visita, "2026-09-09")).toBe("EN_CURSO");
    expect(estadoVisita(visita, "2026-09-11")).toBe("FINALIZADA");
  });
});

describe("agrupar filas de mis_visitas en visitas", () => {
  it("junta varias filas de invitados de la misma visita en una sola tarjeta", () => {
    const filas = [
      filaBase({ invitado_id: "1", visitante_nombre: "Ana" }) as unknown as MisVisitasFila,
      filaBase({ invitado_id: "2", visitante_nombre: "Beto" }) as unknown as MisVisitasFila,
    ];
    const visitas = agruparVisitas(filas);
    expect(visitas).toHaveLength(1);
    expect(visitas[0]?.invitados).toHaveLength(2);
  });
  it("separa visitas distintas", () => {
    const filas = [
      filaBase({ visita_id: "a" }),
      filaBase({ visita_id: "b" }),
    ];
    expect(agruparVisitas(filas)).toHaveLength(2);
  });
});

describe("fechas de Costa Rica", () => {
  it("no cambia el día al cruzar medianoche en UTC", () => {
    expect(hoyCostaRica(new Date("2026-09-10T05:59:59Z"))).toBe("2026-09-09");
    expect(hoyCostaRica(new Date("2026-09-10T06:00:00Z"))).toBe("2026-09-10");
  });
});
