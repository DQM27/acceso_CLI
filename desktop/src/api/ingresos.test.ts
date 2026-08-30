import { describe, expect, it } from "vitest";
import {
  gafetesDe,
  mensajeBloqueo,
  mensajeMotivoDenegacion,
  puedeContinuar,
  sanearGafetes,
  textoMedio,
} from "./ingresos";
import type { PreparacionIngreso } from "./ingresos";

describe("textoMedio", () => {
  it("Vehiculo -> Vehículo, cualquier otro -> Caminando", () => {
    expect(textoMedio("Vehiculo")).toBe("Vehículo");
    expect(textoMedio("Caminando")).toBe("Caminando");
  });
});

function preparacion(overrides: Partial<PreparacionIngreso> = {}): PreparacionIngreso {
  return {
    contratista_id: 1,
    cedula: "1-0847-0293",
    nombre: "Marlon Quesada",
    empresa_nombre: "Constructora del Valle",
    tipo_ingreso: "Praind",
    resultado_acceso: "Permitido",
    requiere_gafete: false,
    tiene_ingreso_activo: false,
    gafetes_deuda: [],
    ...overrides,
  };
}

describe("puedeContinuar", () => {
  it("permite continuar con acceso permitido y sin ingreso activo", () => {
    expect(puedeContinuar(preparacion())).toBe(true);
  });

  it("permite continuar con advertencia (no es un bloqueo)", () => {
    expect(puedeContinuar(preparacion({ resultado_acceso: "PermitidoConAdvertencia" }))).toBe(
      true,
    );
  });

  it("no permite continuar si ya tiene un ingreso activo", () => {
    expect(puedeContinuar(preparacion({ tiene_ingreso_activo: true }))).toBe(false);
  });

  it("no permite continuar si el acceso está denegado", () => {
    expect(
      puedeContinuar(preparacion({ resultado_acceso: { Denegado: "PraindVencido" } })),
    ).toBe(false);
  });
});

describe("mensajeBloqueo", () => {
  it("prioriza el ingreso activo sobre el motivo de denegación", () => {
    expect(
      mensajeBloqueo(
        preparacion({
          tiene_ingreso_activo: true,
          resultado_acceso: { Denegado: "SinAcceso" },
        }),
      ),
    ).toBe("El contratista ya tiene un ingreso activo.");
  });

  it("usa el mensaje del motivo de denegación cuando no hay ingreso activo", () => {
    expect(
      mensajeBloqueo(preparacion({ resultado_acceso: { Denegado: "PraindVencido" } })),
    ).toBe("Acceso denegado · PRAIND vencido");
  });
});

describe("mensajeMotivoDenegacion", () => {
  it.each([
    ["SinAcceso", "Acceso denegado · no tiene acceso autorizado"],
    ["PraindVencido", "Acceso denegado · PRAIND vencido"],
    ["PraindNoRegistrado", "Acceso denegado · PRAIND sin fecha registrada"],
    ["EmpresaInactiva", "Acceso denegado · la empresa está inactiva"],
  ] as const)("%s -> %s", (motivo, esperado) => {
    expect(mensajeMotivoDenegacion(motivo)).toBe(esperado);
  });
});

describe("sanearGafetes", () => {
  it("conserva dígitos, comas y espacios", () => {
    expect(sanearGafetes("2, 25, 85")).toBe("2, 25, 85");
  });

  it("descarta cualquier otro carácter", () => {
    expect(sanearGafetes("2a, 25!, 85#")).toBe("2, 25, 85");
  });

  it("trunca a 60 caracteres", () => {
    const entrada = "1".repeat(100);
    expect(sanearGafetes(entrada)).toHaveLength(60);
  });
});

describe("gafetesDe", () => {
  it("parsea una lista separada por comas", () => {
    expect(gafetesDe("2, 25, 85")).toEqual([2, 25, 85]);
  });

  it("ignora tokens vacíos (comas de más, espacios)", () => {
    expect(gafetesDe("2,, 25,  ,85,")).toEqual([2, 25, 85]);
  });

  it("descarta tokens no enteros", () => {
    expect(gafetesDe("2, abc, 2.5, 85")).toEqual([2, 85]);
  });

  it("texto vacío da lista vacía", () => {
    expect(gafetesDe("")).toEqual([]);
  });
});
