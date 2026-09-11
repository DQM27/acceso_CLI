import { describe, expect, it } from "vitest";
import { esquema } from "./FijarPasswordInicial";

describe("esquema de FijarPasswordInicial", () => {
  it("acepta contraseñas iguales de al menos 8 caracteres", () => {
    expect(esquema.safeParse({ password: "12345678", confirmar: "12345678" }).success).toBe(true);
  });

  it("rechaza contraseña de menos de 8 caracteres", () => {
    const resultado = esquema.safeParse({ password: "1234567", confirmar: "1234567" });
    expect(resultado.success).toBe(false);
  });

  it("rechaza confirmación vacía", () => {
    const resultado = esquema.safeParse({ password: "12345678", confirmar: "" });
    expect(resultado.success).toBe(false);
  });

  it("rechaza cuando password y confirmar no coinciden, con el error en confirmar", () => {
    const resultado = esquema.safeParse({ password: "12345678", confirmar: "87654321" });
    expect(resultado.success).toBe(false);
    if (!resultado.success) {
      expect(resultado.error.issues[0].path).toEqual(["confirmar"]);
      expect(resultado.error.issues[0].message).toBe("Las contraseñas no coinciden");
    }
  });
});
