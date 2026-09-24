import { describe, expect, it } from "vitest";
import { avisosContratista, validarGafete, validarPlaca } from "./NuevoIngresoModal";

describe("avisosContratista (chips del buscador)", () => {
  const hoy = "2026-09-23";
  type Datos = Parameters<typeof avisosContratista>[0];
  const base: Datos = { tiene_ingreso_activo: false, tiene_acceso: true, fecha_vencimiento_praind: "2027-01-01" };
  const textos = (c: Partial<Datos>) =>
    avisosContratista({ ...base, ...c }, hoy).map((aviso) => aviso.texto);

  it("sin nada que avisar, no hay chips", () => {
    expect(textos({})).toEqual([]);
  });

  it("avisa adentro, sin acceso y PRAIND vencido, en ese orden", () => {
    expect(
      textos({ tiene_ingreso_activo: true, tiene_acceso: false, fecha_vencimiento_praind: "2026-09-22" }),
    ).toEqual(["Adentro", "Sin acceso", "PRAIND vencido"]);
  });

  it("el PRAIND que vence hoy todavía no está vencido; sin fecha no avisa", () => {
    expect(textos({ fecha_vencimiento_praind: "2026-09-23" })).toEqual([]);
    expect(textos({ fecha_vencimiento_praind: null })).toEqual([]);
  });
});

describe("validarGafete", () => {
  it("sin gafete requerido, siempre válido con numero null", () => {
    expect(validarGafete("", false)).toEqual({ valido: true, numero: null });
    expect(validarGafete("basura", false)).toEqual({ valido: true, numero: null });
  });

  it("gafete requerido y vacío", () => {
    expect(validarGafete("", true)).toEqual({ valido: false, mensaje: "El gafete es requerido" });
    expect(validarGafete("   ", true)).toEqual({ valido: false, mensaje: "El gafete es requerido" });
  });

  it("gafete requerido y no numérico", () => {
    expect(validarGafete("abc", true)).toEqual({
      valido: false,
      mensaje: "Ingrese un número de gafete válido",
    });
  });

  it("gafete requerido y válido", () => {
    expect(validarGafete("12", true)).toEqual({ valido: true, numero: 12 });
    expect(validarGafete("  7  ", true)).toEqual({ valido: true, numero: 7 });
  });

  it("acepta números con texto arrastrado (parseInt) -- documenta el comportamiento actual", () => {
    // parseInt("12abc") da 12, no NaN -- mismo comportamiento que ya tenía
    // el código original antes de extraer la función, no un cambio nuevo.
    expect(validarGafete("12abc", true)).toEqual({ valido: true, numero: 12 });
  });
});

describe("validarPlaca", () => {
  it("vacía o sólo espacios es inválida", () => {
    expect(validarPlaca("")).toEqual({ valido: false, mensaje: "La placa es requerida" });
    expect(validarPlaca("   ")).toEqual({ valido: false, mensaje: "La placa es requerida" });
  });

  it("recorta espacios y acepta cualquier texto no vacío", () => {
    expect(validarPlaca("ABC123")).toEqual({ valido: true, placa: "ABC123" });
    expect(validarPlaca("  ABC123  ")).toEqual({ valido: true, placa: "ABC123" });
  });
});
