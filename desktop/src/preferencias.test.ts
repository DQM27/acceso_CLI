import { afterEach, describe, expect, it } from "vitest";
import { clavePorUsuario, guardarPreferencia, leerPreferencia } from "./preferencias";

afterEach(() => localStorage.clear());

describe("preferencias por usuario", () => {
  it("antepone el id del usuario a la clave", () => {
    expect(clavePorUsuario("sidebar:orden", 7)).toBe("u7:sidebar:orden");
    expect(clavePorUsuario("sidebar:orden", null)).toBe("sidebar:orden");
  });

  it("cada usuario lee sólo lo suyo", () => {
    guardarPreferencia("escritorio:tema", 1, "tokyo-night");
    guardarPreferencia("escritorio:tema", 2, "light");
    expect(leerPreferencia("escritorio:tema", 1)).toBe("tokyo-night");
    expect(leerPreferencia("escritorio:tema", 2)).toBe("light");
  });

  it("sin guardado propio, parte de la clave global vieja", () => {
    localStorage.setItem("sidebar:colapsado", "1");
    expect(leerPreferencia("sidebar:colapsado", 3)).toBe("1");
  });

  it("lo propio gana sobre la clave global vieja", () => {
    localStorage.setItem("sidebar:colapsado", "1");
    guardarPreferencia("sidebar:colapsado", 3, "0");
    expect(leerPreferencia("sidebar:colapsado", 3)).toBe("0");
  });

  it("sin nada guardado devuelve null", () => {
    expect(leerPreferencia("sidebar:orden", 4)).toBeNull();
  });
});
