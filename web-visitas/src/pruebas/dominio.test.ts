import { describe, expect, it } from "vitest";
import { esquemaNuevaCita, normalizarDocumento } from "../dominio";
import { estadoCita, hoyCostaRica } from "../fecha";

const datos = () => ({
  fecha_desde: "2026-09-09",
  fecha_hasta: "2026-09-10",
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
describe("validación de citas", () => {
  it("normaliza documentos y campos opcionales", () => {
    const resultado = esquemaNuevaCita("2026-09-09").parse(datos());
    expect(resultado.visitantes[0]).toMatchObject({
      cedula: "123456789",
      empresa: null,
      placa_vehiculo: null,
    });
    expect(resultado.motivo).toBeNull();
    expect(normalizarDocumento(" ab-123 ")).toBe("AB123");
  });
  it("rechaza documentos duplicados aunque se escriban con separadores distintos", () => {
    const entrada = datos();
    const [primero] = entrada.visitantes;
    if (!primero) throw new Error("fixture sin visitantes");
    entrada.visitantes.push({ ...primero, cedula: "123456789" });
    const resultado = esquemaNuevaCita("2026-09-09").safeParse(entrada);
    expect(resultado.success).toBe(false);
    if (!resultado.success)
      expect(resultado.error.issues[0]?.path).toEqual([
        "visitantes",
        1,
        "cedula",
      ]);
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
    expect(
      esquemaNuevaCita("2026-09-09").safeParse({ ...datos(), ...cambio })
        .success,
    ).toBe(false);
  });
  it("acepta una visita de un solo día y rechaza un grupo sin límite", () => {
    const entrada = datos();
    entrada.fecha_hasta = entrada.fecha_desde;
    expect(esquemaNuevaCita("2026-09-09").safeParse(entrada).success).toBe(
      true,
    );
    const [primero] = entrada.visitantes;
    if (!primero) throw new Error("fixture sin visitantes");
    entrada.visitantes = Array.from({ length: 51 }, (_, i) => ({
      ...primero,
      cedula: `DOC${i}`,
    }));
    expect(esquemaNuevaCita("2026-09-09").safeParse(entrada).success).toBe(
      false,
    );
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
