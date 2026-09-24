import { describe, expect, it } from "vitest";
import { esquemaCambioPassword } from "./CambiarPasswordModal";

function primerError(valores: {
  passwordActual: string;
  passwordNueva: string;
  confirmar: string;
}) {
  const resultado = esquemaCambioPassword.safeParse(valores);
  return resultado.success ? null : resultado.error.issues[0]?.path;
}

describe("esquema de cambio de contraseña", () => {
  it("acepta actual, una nueva de 8+ caracteres distinta y su confirmación", () => {
    expect(
      primerError({ passwordActual: "vieja123", passwordNueva: "nueva1234", confirmar: "nueva1234" }),
    ).toBeNull();
  });

  it("exige la contraseña actual", () => {
    expect(
      primerError({ passwordActual: "", passwordNueva: "nueva1234", confirmar: "nueva1234" }),
    ).toEqual(["passwordActual"]);
  });

  it("rechaza una nueva de menos de 8 caracteres", () => {
    expect(
      primerError({ passwordActual: "vieja123", passwordNueva: "corta", confirmar: "corta" }),
    ).toEqual(["passwordNueva"]);
  });

  it("rechaza si la confirmación no coincide", () => {
    expect(
      primerError({ passwordActual: "vieja123", passwordNueva: "nueva1234", confirmar: "otra1234" }),
    ).toEqual(["confirmar"]);
  });

  it("rechaza una nueva igual a la actual", () => {
    expect(
      primerError({ passwordActual: "misma1234", passwordNueva: "misma1234", confirmar: "misma1234" }),
    ).toEqual(["passwordNueva"]);
  });
});
