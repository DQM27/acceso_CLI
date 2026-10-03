import { describe, expect, it } from "vitest";
import { avisosContratista, gafeteParaRegistrar } from "./NuevoIngresoModal.logica";

describe("avisosContratista (chips del buscador)", () => {
  type Datos = Parameters<typeof avisosContratista>[0];
  const base: Datos = { tiene_ingreso_activo: false, aviso_acceso: null };
  const textos = (c: Partial<Datos>) => avisosContratista({ ...base, ...c }).map((aviso) => aviso.texto);

  it("sin nada que avisar, no hay chips", () => {
    expect(textos({})).toEqual([]);
  });

  it("muestra adentro y el aviso del núcleo tal cual, en ese orden", () => {
    expect(textos({ tiene_ingreso_activo: true, aviso_acceso: "ACCESO DENEGADO" })).toEqual([
      "Adentro",
      "ACCESO DENEGADO",
    ]);
  });
});

describe("gafeteParaRegistrar", () => {
  it("sin gafete requerido es S/G, aunque haya texto", () => {
    expect(gafeteParaRegistrar("basura", false, false)).toEqual({ gafete: null });
  });

  it("con «Sin gafete» marcado es S/G, aunque haya texto", () => {
    expect(gafeteParaRegistrar("7", true, true)).toEqual({ gafete: null });
  });

  it("requerido y vacío, sin marcar «Sin gafete», pide el número", () => {
    expect(gafeteParaRegistrar("   ", true, false)).toEqual({
      error: "Ingrese el número de gafete o marque «Sin gafete»",
    });
  });

  it("texto que no es número es un error", () => {
    expect(gafeteParaRegistrar("abc", true, false)).toEqual({ error: "Ingrese un número de gafete válido" });
  });

  it("número con espacios se convierte", () => {
    expect(gafeteParaRegistrar("  7  ", true, false)).toEqual({ gafete: 7 });
  });
});
