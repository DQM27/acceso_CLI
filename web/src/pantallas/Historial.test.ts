import { describe, expect, it } from "vitest";
import { DEFINICIONES_EXPORT, generarHtmlHistorial } from "./Historial";
import type { MovimientoHistorial } from "../api/historial";

// Mismos tres casos que desktop/src-tauri/src/pdf/html.rs -- una sola
// definición de "cómo se ve el PDF de Historial" en dos lenguajes distintos
// necesita el mismo criterio de verificación en ambos lados.

function fila(sobrescribir: Partial<MovimientoHistorial> = {}): MovimientoHistorial {
  return {
    id: "1",
    sitio_id: "s1",
    sitio_nombre: "Brisas",
    contratista_cedula: "001010101",
    contratista_nombre: 'María <Pérez> & "Ruiz"',
    empresa_nombre: "Brisas",
    tipo_ingreso: "PRAIND",
    medio_ingreso: "CAMINANDO",
    gafete_numero: 7,
    hora_entrada: "2026-08-20T14:30:00Z",
    hora_salida: null,
    usuario_entrada_nombre: "Quintana",
    usuario_salida_nombre: null,
    dispositivo_entrada_tipo: "pc",
    ...sobrescribir,
  };
}

function columnas(...colIds: string[]) {
  return colIds.map((colId) => {
    const definicion = DEFINICIONES_EXPORT.find((d) => d.colId === colId);
    if (!definicion) throw new Error(`sin definición para ${colId}`);
    return definicion;
  });
}

describe("generarHtmlHistorial", () => {
  it("incluye encabezado, columnas y datos", () => {
    const html = generarHtmlHistorial(
      [fila({ contratista_nombre: "Alguien Normal" })],
      columnas("contratista_nombre", "gafete_numero"),
      { generadoPor: "Daniel Quintana", filtro: "Todo el historial" },
    );

    expect(html).toContain("<title>Historial de Movimientos</title>");
    expect(html).toContain("Daniel Quintana");
    expect(html).toContain("Todo el historial");
    expect(html).toContain("NOMBRE");
    expect(html).toContain("GAFETE");
    expect(html).toContain("7");
  });

  it("escapa HTML en datos de usuario para no inyectar marcado", () => {
    const html = generarHtmlHistorial([fila()], columnas("contratista_nombre"), {
      generadoPor: "Daniel",
      filtro: "Todo el historial",
    });

    expect(html).not.toContain("<Pérez>");
    expect(html).toContain("&lt;Pérez&gt;");
    expect(html).toContain("&quot;Ruiz&quot;");
  });

  it('columna activa sin salida muestra "Activo", no vacío', () => {
    const html = generarHtmlHistorial([fila()], columnas("hora_salida", "fecha_salida"), {
      generadoPor: "Daniel",
      filtro: "Todo el historial",
    });

    expect(html.match(/Activo/g)).toHaveLength(2);
  });

  it("respeta el orden y subconjunto de columnas visibles pasado", () => {
    const html = generarHtmlHistorial([fila()], columnas("gafete_numero", "contratista_nombre"), {
      generadoPor: "Daniel",
      filtro: "Todo el historial",
    });

    const posicionGafete = html.indexOf("GAFETE");
    const posicionNombre = html.indexOf("NOMBRE");
    expect(posicionGafete).toBeGreaterThan(0);
    expect(posicionGafete).toBeLessThan(posicionNombre);
  });
});
