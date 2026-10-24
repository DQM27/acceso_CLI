import { describe, expect, it } from "vitest";
import { parametrosContratistas, parametrosIngresos } from "./consulta";
import { validarLienzo, type Lienzo } from "./esquema";

function lienzo(fuente: Lienzo["fuente"], filtros: unknown[]): Lienzo {
  const r = validarLienzo({ fuente, filtros, widgets: [{ tipo: "kpi", titulo: "t", medida: "contar" }] });
  if (!r.ok) throw new Error(r.errores.join("; "));
  return r.lienzo;
}

describe("parametrosContratistas", () => {
  it("traduce cada filtro del catálogo al parámetro de la función SQL", () => {
    const p = parametrosContratistas(
      lienzo("contratistas", [
        { campo: "empresa", op: "en", valor: ["A", "B"] },
        { campo: "estado_praind", op: "igual", valor: "VENCIDA" },
        { campo: "tipo_ingreso", op: "en", valor: ["PRAIND", "SWAT"] },
        { campo: "sitio", op: "igual", valor: "Brisas" },
        { campo: "acceso_activo", op: "igual", valor: true },
        { campo: "nombre", op: "contiene", valor: "pérez" },
        { campo: "vencimiento", op: "entre", valor: ["2026-10-01", "2026-10-31"] },
        { campo: "dias_para_vencer", op: "entre", valor: [0, 30] },
      ]),
    );
    expect(p).toEqual({
      p_empresas: ["A", "B"],
      p_estados_praind: ["VENCIDA"],
      p_tipos_ingreso: ["PRAIND", "SWAT"],
      p_sitio: "Brisas",
      p_acceso_activo: true,
      p_contiene_nombre: "pérez",
      p_vencimiento_desde: "2026-10-01",
      p_vencimiento_hasta: "2026-10-31",
      p_dias_desde: 0,
      p_dias_hasta: 30,
    });
  });

  it("empresa: 'no en' excluye y 'contiene' busca por texto", () => {
    expect(parametrosContratistas(lienzo("contratistas", [{ campo: "empresa", op: "no_en", valor: ["A"] }]))).toEqual({
      p_empresas_excluir: ["A"],
    });
    expect(parametrosContratistas(lienzo("contratistas", [{ campo: "empresa", op: "contiene", valor: "ACME" }]))).toEqual({
      p_contiene_empresa: "ACME",
    });
  });

  it("'no en' sobre una enumeración pide el resto de valores del catálogo", () => {
    const p = parametrosContratistas(
      lienzo("contratistas", [{ campo: "estado_praind", op: "no_en", valor: ["VIGENTE", "NO_REQUIERE"] }]),
    );
    expect(p.p_estados_praind).toEqual(["POR_VENCER", "VENCIDA", "SIN_REGISTRO"]);
  });

  it("'igual' en una fecha o un número fija los dos extremos", () => {
    expect(parametrosContratistas(lienzo("contratistas", [{ campo: "dias_para_vencer", op: "igual", valor: 7 }]))).toEqual({
      p_dias_desde: 7,
      p_dias_hasta: 7,
    });
  });

  it("rechaza dos filtros que se pisan en vez de combinarlos a escondidas", () => {
    expect(() =>
      parametrosContratistas(
        lienzo("contratistas", [
          { campo: "empresa", op: "en", valor: ["A"] },
          { campo: "empresa", op: "igual", valor: "B" },
        ]),
      ),
    ).toThrow(/se pisan/);
  });
});

describe("parametrosIngresos", () => {
  it("sin filtro de día usa las últimas 2 semanas hasta hoy", () => {
    expect(parametrosIngresos(lienzo("ingresos", []), "2026-10-22")).toEqual({
      p_desde: "2026-10-09",
      p_hasta: "2026-10-22",
    });
  });

  it("cruza el cambio de mes y de año al restar días", () => {
    expect(parametrosIngresos(lienzo("ingresos", []), "2027-01-05").p_desde).toBe("2026-12-23");
  });

  it("respeta el día pedido y traduce los demás filtros", () => {
    const p = parametrosIngresos(
      lienzo("ingresos", [
        { campo: "dia", op: "entre", valor: ["2026-09-01", "2026-09-30"] },
        { campo: "empresa", op: "no_en", valor: ["A"] },
        { campo: "tipo_ingreso", op: "igual", valor: "PRAIND" },
      ]),
      "2026-10-22",
    );
    expect(p).toEqual({
      p_desde: "2026-09-01",
      p_hasta: "2026-09-30",
      p_empresas_excluir: ["A"],
      p_tipos_ingreso: ["PRAIND"],
    });
  });

  it("un solo día pedido es el rango completo", () => {
    const p = parametrosIngresos(lienzo("ingresos", [{ campo: "dia", op: "igual", valor: "2026-10-20" }]), "2026-10-22");
    expect(p).toMatchObject({ p_desde: "2026-10-20", p_hasta: "2026-10-20" });
  });
});
