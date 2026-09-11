import { describe, expect, it } from "vitest";
import { esquemaLogin } from "./Login";

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
