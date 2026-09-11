import { describe, expect, it } from "vitest";
import { esquemaCambioObligatorio, esquemaLogin } from "./Login";

describe("esquema de Login", () => {
  it("acepta cédula y contraseña no vacías", () => {
    expect(esquemaLogin.safeParse({ cedula: "108470293", password: "algo" }).success).toBe(true);
  });

  it("rechaza cédula vacía", () => {
    expect(esquemaLogin.safeParse({ cedula: "", password: "algo" }).success).toBe(false);
  });

  it("rechaza contraseña vacía", () => {
    expect(esquemaLogin.safeParse({ cedula: "108470293", password: "" }).success).toBe(false);
  });
});

describe("esquema de cambio obligatorio", () => {
  it("acepta una contraseña de al menos 8 caracteres que coincide con la confirmación", () => {
    expect(
      esquemaCambioObligatorio.safeParse({ passwordNueva: "12345678", confirmar: "12345678" })
        .success,
    ).toBe(true);
  });

  it("rechaza una contraseña de menos de 8 caracteres", () => {
    expect(
      esquemaCambioObligatorio.safeParse({ passwordNueva: "1234567", confirmar: "1234567" })
        .success,
    ).toBe(false);
  });

  it("rechaza si la confirmación no coincide", () => {
    const resultado = esquemaCambioObligatorio.safeParse({
      passwordNueva: "12345678",
      confirmar: "distinta",
    });
    expect(resultado.success).toBe(false);
    if (!resultado.success) {
      expect(resultado.error.issues[0]?.path).toEqual(["confirmar"]);
    }
  });
});
