import { describe, expect, it } from "vitest";
import {
  DEFINICIONES_EXPORT,
  campoOrdenDeColumna,
  generarCsvHistorial,
  generarHtmlHistorial,
} from "./Historial";
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
    tipo_texto: "PRAIND",
    medio_texto: "CAMINANDO",
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

describe("campoOrdenDeColumna", () => {
  it("las columnas de fecha y hora ordenan por el instante correspondiente", () => {
    expect(campoOrdenDeColumna("fecha_ingreso")).toBe("hora_entrada");
    expect(campoOrdenDeColumna("hora_ingreso")).toBe("hora_entrada");
    expect(campoOrdenDeColumna("fecha_salida")).toBe("hora_salida");
    expect(campoOrdenDeColumna("hora_salida")).toBe("hora_salida");
  });

  it("tipo y medio ordenan por el texto que se ve", () => {
    expect(campoOrdenDeColumna("tipo_ingreso")).toBe("tipo_texto");
    expect(campoOrdenDeColumna("medio_ingreso")).toBe("medio_texto");
  });

  it("las columnas con campo propio lo usan y una desconocida no ordena en el servidor", () => {
    expect(campoOrdenDeColumna("empresa_nombre")).toBe("empresa_nombre");
    expect(campoOrdenDeColumna("gafete_numero")).toBe("gafete_numero");
    expect(campoOrdenDeColumna("inventada")).toBeNull();
  });
});

describe("generarCsvHistorial", () => {
  it("separa con ; , empieza con BOM y cierra cada línea con salto", () => {
    const csv = generarCsvHistorial(
      [fila({ contratista_nombre: "Ana", gafete_numero: 7 })],
      columnas("contratista_nombre", "gafete_numero"),
    );

    expect(csv.charCodeAt(0)).toBe(0xfeff);
    expect(csv.slice(1)).toBe("Nombre;Gafete\r\nAna;7\r\n");
  });

  it("pone comillas cuando el texto lleva ; comillas o salto de línea", () => {
    const csv = generarCsvHistorial(
      [fila({ contratista_nombre: 'Ana; "la" Pérez' })],
      columnas("contratista_nombre"),
    );

    expect(csv).toContain('"Ana; ""la"" Pérez"');
  });

  it("Tipo y Medio salen con el texto ya armado por la vista", () => {
    const csv = generarCsvHistorial(
      [fila({ tipo_texto: "IN HOUSE", medio_texto: "ABC123" })],
      columnas("tipo_ingreso", "medio_ingreso"),
    );

    expect(csv).toContain("IN HOUSE;ABC123");
  });
});
