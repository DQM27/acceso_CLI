import { describe, expect, it } from "vitest";
import { columnasParaExportar } from "./exportar";
import { nombreDeArchivo, pngDesdeDataUrl, svgIndependiente } from "./imagen";

describe("columnasParaExportar", () => {
  it("usa las columnas del catálogo con sus etiquetas y formatea fechas y sí/no", () => {
    const columnas = columnasParaExportar("contratistas", "dia_mes_anio");
    expect(columnas.map((c) => c.etiqueta)).toContain("Estado PRAIND");
    const fila = { nombre: "Ana", vencimiento: "2026-11-08", acceso_activo: true, empresa: null };
    const valor = (id: string) => columnas.find((c) => c.colId === id)?.valor(fila);
    expect(valor("vencimiento")).toBe("08/11/2026");
    expect(valor("acceso_activo")).toBe("Sí");
    // Los días para vencer salen en palabras también en Excel, CSV y PDF.
    expect(columnas.find((c) => c.colId === "dias_para_vencer")?.valor({ dias_para_vencer: -3 })).toBe("Vencido hace 3 días");
    expect(valor("empresa")).toBe("");
    expect(columnas.find((c) => c.colId === "nombre")?.izquierda).toBe(true);
  });

  it("con una consulta, exporta exactamente las columnas que declara", () => {
    const columnas = columnasParaExportar({
      columnas: [
        { nombre: "proveedor", tipo: "texto", etiqueta: "Proveedor" },
        { nombre: "visitas", tipo: "numero" },
        { nombre: "ultima", tipo: "fecha", etiqueta: "Última visita" },
      ],
    });
    expect(columnas.map((c) => c.etiqueta)).toEqual(["Proveedor", "visitas", "Última visita"]);
    expect(columnas[2].valor({ ultima: "2026-10-20T00:00:00" })).toMatch(/2026/);
    expect(columnas[1].izquierda).toBe(false);
  });
});

describe("imagen de un gráfico", () => {
  it("el SVG exportado lleva los estilos en línea, su tamaño y nada de clases ni animaciones", () => {
    document.body.innerHTML =
      '<svg class="lienzo-svg" viewBox="0 0 10 10"><rect class="lienzo-crecer-v" width="5" height="5" style="fill: rgb(42, 120, 214)"/></svg>';
    const svg = document.querySelector("svg") as SVGSVGElement;
    const texto = svgIndependiente(svg, 320, 200);
    expect(texto).toContain('width="320"');
    expect(texto).toContain('height="200"');
    expect(texto).toContain("fill:rgb(42, 120, 214)");
    expect(texto).toContain("animation:none");
    expect(texto).not.toContain("class=");
    // El original no se toca.
    expect(svg.getAttribute("class")).toBe("lienzo-svg");
  });

  it("del sandbox sólo se acepta un PNG de verdad", () => {
    const pngMinimo = btoa(String.fromCharCode(0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0));
    expect(pngDesdeDataUrl(`data:image/png;base64,${pngMinimo}`)?.type).toBe("image/png");
    expect(pngDesdeDataUrl(`data:image/png;base64,${btoa("<html>no soy png</html>")}`)).toBeNull();
    expect(pngDesdeDataUrl(`data:text/html;base64,${pngMinimo}`)).toBeNull();
    expect(pngDesdeDataUrl("data:image/png;base64,%%%")).toBeNull();
    expect(pngDesdeDataUrl(42)).toBeNull();
  });

  it("nombres de archivo sin tildes ni símbolos", () => {
    expect(nombreDeArchivo("Ingresos por día · ALDAMA/K-9", "png")).toBe("ingresos-por-dia-aldama-k-9.png");
    expect(nombreDeArchivo("¿?", "csv")).toBe("lienzo.csv");
  });
});
