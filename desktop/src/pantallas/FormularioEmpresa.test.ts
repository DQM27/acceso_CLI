import { describe, expect, it } from "vitest";
import { esquema } from "./FormularioEmpresa";

describe("esquema de FormularioEmpresa", () => {
  it("nombre vacío no pasa", () => {
    expect(esquema.safeParse({ nombre: "" }).success).toBe(false);
  });

  it("a diferencia de Contratista, números y símbolos sí pasan (S.A., 3M)", () => {
    expect(esquema.safeParse({ nombre: "Constructora del Valle S.A." }).success).toBe(true);
    expect(esquema.safeParse({ nombre: "3M Costa Rica" }).success).toBe(true);
  });
});
