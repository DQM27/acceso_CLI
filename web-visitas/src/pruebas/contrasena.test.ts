import { describe, expect, it } from "vitest";
import { LONGITUD_MINIMA, normalizarCorreo, problemaDeContrasenaNueva } from "../lib/contrasena";

const correo = "ana.mora@empresa.example";

describe("política de contraseñas de anfitriones", () => {
  it("acepta una frase larga sin exigir símbolos ni mayúsculas", () => {
    expect(problemaDeContrasenaNueva("el cafe de la tarde en cartago", correo)).toBeNull();
  });
  it("exige la longitud mínima contando caracteres, no bytes", () => {
    expect(problemaDeContrasenaNueva("a".repeat(LONGITUD_MINIMA - 1), correo)).toContain(`${LONGITUD_MINIMA}`);
    expect(problemaDeContrasenaNueva("ñandú ñandú ñandú", correo)).toBeNull();
  });
  it("rechaza lo que bcrypt truncaría (más de 72 bytes)", () => {
    expect(problemaDeContrasenaNueva("ñ".repeat(37), correo)).toContain("larga");
  });
  it("rechaza caracteres repetidos", () => {
    expect(problemaDeContrasenaNueva("aaaaaaaaaaaaaaaaaaaa", correo)).toContain("repetidos");
  });
  it("rechaza el usuario del correo dentro de la contraseña", () => {
    expect(problemaDeContrasenaNueva("ana.mora 2026 visitas!", correo)).toContain("correo");
  });
  it("rechaza la palabra obvia con relleno", () => {
    expect(problemaDeContrasenaNueva("Contraseña2026!!!!", correo)).toContain("fácil");
    expect(problemaDeContrasenaNueva("megabrisas123456", correo)).toContain("fácil");
  });
  it("normaliza el correo como Supabase Auth", () => {
    expect(normalizarCorreo("  Ana.Mora@Empresa.Example ")).toBe("ana.mora@empresa.example");
  });
});
