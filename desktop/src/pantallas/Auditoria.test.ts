import { describe, expect, it } from "vitest";
import { fechaLocalYMD, nombresActuales, textoFechaDDMMYYYY, textoHora } from "./Auditoria";
import type { CambioAuditado } from "../api";

// Agnósticas al huso horario del runner — ver el mismo criterio en
// Activos.test.ts.
describe("fechaLocalYMD / textoHora", () => {
  const fecha = new Date(2027, 2, 8, 14, 5);
  const iso = fecha.toISOString();

  it("fechaLocalYMD arma año-mes-día con padding, en hora local", () => {
    const esperado = `${fecha.getFullYear()}-${String(fecha.getMonth() + 1).padStart(2, "0")}-${String(
      fecha.getDate(),
    ).padStart(2, "0")}`;
    expect(fechaLocalYMD(iso)).toBe(esperado);
  });

  it("textoHora es de 24 horas, con padding", () => {
    const esperado = fecha.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
    expect(textoHora(iso)).toBe(esperado);
  });
});

describe("textoFechaDDMMYYYY", () => {
  it("reordena YYYY-MM-DD a DD/MM/YYYY", () => {
    expect(textoFechaDDMMYYYY("2027-03-08")).toBe("08/03/2027");
  });
});

describe("nombresActuales", () => {
  function cambio(overrides: Partial<CambioAuditado>): CambioAuditado {
    return {
      id: 1,
      fecha_hora: "2027-03-08T12:00:00Z",
      usuario_id: 1,
      usuario_nombre: "root",
      entidad: "Empresa",
      entidad_id: 5,
      entidad_nombre: "BAC",
      campo: "nombre",
      valor_anterior: null,
      valor_nuevo: null,
      ...overrides,
    };
  }

  it("con un solo cambio, ese es el nombre actual", () => {
    const items = [cambio({ entidad_nombre: "BAC" })];
    expect(nombresActuales(items)).toEqual(new Map([["Empresa:5", "BAC"]]));
  });

  it("items viene ordenado DESC por fecha del núcleo — se queda con el primero visto", () => {
    // "BAC" es el nombre más reciente (fila 0, la que ve primero el loop);
    // "BACA" es el nombre viejo que tenía antes del renombre.
    const items = [
      cambio({ id: 2, entidad_nombre: "BAC" }),
      cambio({ id: 1, entidad_nombre: "BACA" }),
    ];
    expect(nombresActuales(items).get("Empresa:5")).toBe("BAC");
  });

  it("distingue entidades distintas con el mismo id (Empresa:5 vs Usuario:5)", () => {
    const items = [
      cambio({ entidad: "Empresa", entidad_id: 5, entidad_nombre: "BAC" }),
      cambio({ entidad: "Usuario", entidad_id: 5, entidad_nombre: "Juan Pérez" }),
    ];
    const nombres = nombresActuales(items);
    expect(nombres.get("Empresa:5")).toBe("BAC");
    expect(nombres.get("Usuario:5")).toBe("Juan Pérez");
  });
});
