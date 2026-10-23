import { afterEach, describe, expect, it } from "vitest";
import { leerTema } from "./useTema";

afterEach(() => localStorage.removeItem("web:tema"));

describe("leerTema", () => {
  it("devuelve el tema Tokyo guardado", () => {
    localStorage.setItem("web:tema", "tokyo-night");
    expect(leerTema()).toBe("tokyo-night");
    localStorage.setItem("web:tema", "tokyo-day");
    expect(leerTema()).toBe("tokyo-day");
  });

  it("el claro y el oscuro de antes pasan a su equivalente Tokyo", () => {
    localStorage.setItem("web:tema", "dark");
    expect(leerTema()).toBe("tokyo-night");
    localStorage.setItem("web:tema", "light");
    expect(leerTema()).toBe("tokyo-day");
  });

  it("sin nada guardado (o algo inválido) toma un tema Tokyo según el sistema", () => {
    localStorage.setItem("web:tema", "cualquiera");
    expect(["tokyo-day", "tokyo-night"]).toContain(leerTema());
  });
});
