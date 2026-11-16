import { describe, expect, it } from "vitest";
import { nombresCoinciden, problemaArchivo, tipoDeClave } from "./DetalleContratista.logica";

describe("nombresCoinciden", () => {
  it("ignora tildes, mayúsculas y espacios", () => {
    expect(nombresCoinciden("Mindy  Murillo Guzmán", "MINDY MURILLO GUZMAN")).toBe(true);
  });

  it("acepta un nombre o apellido de más en cualquiera de los dos", () => {
    expect(nombresCoinciden("Ana María Pérez Soto", "ANA PEREZ SOTO")).toBe(true);
    expect(nombresCoinciden("Ana Pérez", "ANA MARIA PEREZ SOTO")).toBe(true);
  });

  it("no coincide con otra persona ni con un solo nombre suelto", () => {
    expect(nombresCoinciden("Ana Pérez Soto", "LUIS PEREZ SOTO")).toBe(false);
    expect(nombresCoinciden("Ana", "ANA PEREZ SOTO")).toBe(false);
    expect(nombresCoinciden(null, "ANA PEREZ")).toBe(false);
    expect(nombresCoinciden("", "ANA PEREZ")).toBe(false);
  });
});

describe("archivo del carné", () => {
  it("toma el tipo de la extensión de la ruta", () => {
    expect(tipoDeClave("contratistas/x/abc.jpg")).toBe("image/jpeg");
    expect(tipoDeClave("contratistas/x/abc.png")).toBe("image/png");
  });

  it("rechaza lo que no es PNG/JPEG o pasa de 10 MB", () => {
    expect(problemaArchivo({ size: 1000, type: "image/png" })).toBeNull();
    expect(problemaArchivo({ size: 1000, type: "application/pdf" })).toMatch(/PNG o JPEG/);
    expect(problemaArchivo({ size: 11 * 1024 * 1024, type: "image/jpeg" })).toMatch(/10 MB/);
  });
});
