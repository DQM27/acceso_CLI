import { describe, expect, it } from "vitest";
import { mensajeError } from "./mensajeError";

describe("mensajeError", () => {
  it("devuelve el mensaje limpio de un Error, sin el prefijo 'Error: ' de String()", () => {
    expect(mensajeError(new Error("Ese correo ya está en la lista."))).toBe(
      "Ese correo ya está en la lista.",
    );
    // Documenta justo el bug que esto reemplaza -- String() sí agrega el prefijo.
    expect(String(new Error("Ese correo ya está en la lista."))).toBe(
      "Error: Ese correo ya está en la lista.",
    );
  });

  it("cae a String() para algo que no es un Error", () => {
    expect(mensajeError("mensaje plano")).toBe("mensaje plano");
    expect(mensajeError({ mensaje: "objeto raro" })).toBe("[object Object]");
    expect(mensajeError(undefined)).toBe("undefined");
  });
});
