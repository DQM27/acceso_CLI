import { describe, expect, it } from "vitest";
import type { EmpresaProveedorPanel } from "../api/empresasProveedor";
import {
  destinosPosibles,
  estadoRevision,
  filtrarPorRevision,
  similitudNombres,
  UMBRAL_PARECIDA,
} from "./EmpresasProveedor.logica";

function empresa(
  nombre: string,
  sobrescribir: Partial<EmpresaProveedorPanel> = {},
): EmpresaProveedorPanel {
  return {
    id: nombre,
    sitio_id: "brisas",
    sitio_nombre: "Brisas",
    nombre,
    activa: true,
    created_at: "2026-10-01T00:00:00Z",
    revisada_en: null,
    revisada_por: null,
    fusionada_en: null,
    fusionada_en_nombre: null,
    creada_por_equipo: "Celular Brisas",
    ingresos: 0,
    ultimo_ingreso: null,
    ...sobrescribir,
  };
}

describe("similitudNombres", () => {
  // Los duplicados reales del catálogo de Brisas (2026-10-30), mismos casos
  // que las pruebas del núcleo (src/texto.rs).
  it.each([
    ["FUDDY FACTORY", "FOODIE FACTORY"],
    ["PROCESOS FRESCOS", "PROCESADOS FRESCOS"],
    ["TEHE CHIKEN", "THE CHIKEN"],
    ["MARISCO JERUSALEN", "MARISCOS JERUSALÉN"],
    ["LA YEMA DORADA", "YEMA DORADA"],
  ])("%s se parece a %s", (a, b) => {
    expect(similitudNombres(a, b)).toBeGreaterThanOrEqual(UMBRAL_PARECIDA);
  });

  it.each([
    ["BIMBO", "BELCA"],
    ["LA GRAN BODEGA", "LA GRANJITA DEL ABUELO"],
    ["CARNES DORADAS", "YEMA DORADA"],
  ])("%s no se parece a %s", (a, b) => {
    expect(similitudNombres(a, b)).toBeLessThan(UMBRAL_PARECIDA);
  });
});

describe("filtrarPorRevision", () => {
  const nueva = empresa("NUEVA");
  const revisada = empresa("REVISADA", { revisada_en: "2026-10-31T00:00:00Z" });
  const fusionada = empresa("FUSIONADA", {
    fusionada_en: "REVISADA",
    revisada_en: "2026-10-31T00:00:00Z",
  });
  const todas = [nueva, revisada, fusionada];

  it("separa por revisar, vigentes y fusionadas", () => {
    expect(filtrarPorRevision(todas, "por_revisar")).toEqual([nueva]);
    expect(filtrarPorRevision(todas, "vigentes")).toEqual([nueva, revisada]);
    expect(filtrarPorRevision(todas, "fusionadas")).toEqual([fusionada]);
    expect(estadoRevision(fusionada)).toBe("fusionada");
  });
});

describe("destinosPosibles", () => {
  it("ofrece primero la parecida, sólo de la misma unidad y sin fusionadas", () => {
    const fuddy = empresa("FUDDY FACTORY");
    const destinos = destinosPosibles(fuddy, [
      fuddy,
      empresa("BIMBO"),
      empresa("FOODIE FACTORY"),
      empresa("FOODIE FACTORY OTRA UNIDAD", { sitio_id: "otra" }),
      empresa("FOODY", { fusionada_en: "FOODIE FACTORY" }),
      empresa("ALIMENTOS MARILU"),
    ]);

    expect(
      destinos.map(({ empresa: e, parecida }) => [e.nombre, parecida]),
    ).toEqual([
      ["FOODIE FACTORY", true],
      ["ALIMENTOS MARILU", false],
      ["BIMBO", false],
    ]);
  });
});
