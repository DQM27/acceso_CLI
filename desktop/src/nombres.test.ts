import { describe, expect, it } from "vitest";
import { escribirNombreEnCampo, nombreMientrasSeEscribe } from "./nombres";

describe("nombres en mayúscula (regla del núcleo)", () => {
  it("pasa a mayúscula sin tocar los espacios, con tildes y ñ", () => {
    expect(nombreMientrasSeEscribe("josé peña ")).toBe("JOSÉ PEÑA ");
  });

  it("en un campo conserva el cursor, y retrocede si se limpió algo", () => {
    const campo = document.createElement("input");
    campo.value = "ana2 mora";
    campo.setSelectionRange(4, 4);
    escribirNombreEnCampo(campo, (texto) => texto.replace(/\d/g, ""));
    expect(campo.value).toBe("ANA MORA");
    expect(campo.selectionStart).toBe(3);
  });
});
