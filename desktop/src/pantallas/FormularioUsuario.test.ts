import { describe, expect, it } from "vitest";
import { construirEsquema } from "./FormularioUsuario";

function valores(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    cedula: "108470293",
    nombre: "Marlon Quesada",
    password: "12345678",
    rol: "Operador",
    activo: true,
    ...overrides,
  };
}

describe("construirEsquema — cédula/nombre/rol (iguales en alta y edición)", () => {
  it.each([true, false])("cédula vacía o con letras no pasa (esCreacion=%s)", (esCreacion) => {
    const esquema = construirEsquema(esCreacion);
    expect(esquema.safeParse(valores({ cedula: "" })).success).toBe(false);
    expect(esquema.safeParse(valores({ cedula: "1-084" })).success).toBe(false);
  });

  it.each([true, false])("nombre vacío o con números no pasa (esCreacion=%s)", (esCreacion) => {
    const esquema = construirEsquema(esCreacion);
    expect(esquema.safeParse(valores({ nombre: "" })).success).toBe(false);
    expect(esquema.safeParse(valores({ nombre: "Root2" })).success).toBe(false);
  });

  it.each([true, false])("rol fuera del enum no pasa (esCreacion=%s)", (esCreacion) => {
    const esquema = construirEsquema(esCreacion);
    expect(esquema.safeParse(valores({ rol: "SuperAdmin" })).success).toBe(false);
  });
});

describe("construirEsquema(true) — alta: la contraseña es obligatoria", () => {
  const esquema = construirEsquema(true);

  it("vacía no pasa", () => {
    expect(esquema.safeParse(valores({ password: "" })).success).toBe(false);
  });

  it("menos de 8 caracteres no pasa", () => {
    expect(esquema.safeParse(valores({ password: "1234567" })).success).toBe(false);
  });

  it("8 caracteres o más sí pasa", () => {
    expect(esquema.safeParse(valores({ password: "12345678" })).success).toBe(true);
  });
});

describe("construirEsquema(false) — edición: vacío = no cambiarla", () => {
  const esquema = construirEsquema(false);

  it("vacía sí pasa (no se toca la contraseña)", () => {
    expect(esquema.safeParse(valores({ password: "" })).success).toBe(true);
  });

  it("no vacía pero corta no pasa — si se escribe algo, cumple el mínimo igual", () => {
    expect(esquema.safeParse(valores({ password: "1234567" })).success).toBe(false);
  });

  it("8 caracteres o más sí pasa", () => {
    expect(esquema.safeParse(valores({ password: "12345678" })).success).toBe(true);
  });
});
