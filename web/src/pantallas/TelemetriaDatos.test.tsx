import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import type { EquipoFlota, RendimientoOperacion } from "../api/telemetriaDatos";
import { VistaFlota, VistaRendimiento } from "./TelemetriaDatos";

// Las vistas de datos de telemetría: qué pide cada una a la nube y qué texto
// arma para cada celda. La grilla real (AG Grid) no dibuja en jsdom: el doble
// dibuja el texto de cada columna con su `valueGetter`/`valueFormatter`.

const mocks = vi.hoisted(() => ({
  claves: [] as unknown[][],
  datos: null as unknown,
}));
vi.mock("../componentes/useLista", () => ({
  useLista: (clave: unknown[]) => {
    mocks.claves.push(clave);
    return { datos: mocks.datos, cargando: false, recargar: vi.fn() };
  },
}));
vi.mock("../componentes/Tabla", () => ({
  default: ({
    filas,
    columnas,
    controles,
  }: {
    controles?: ReactNode;
    filas: unknown[];
    columnas: Array<{
      field?: string;
      colId?: string;
      valueGetter?: (p: { data: unknown }) => unknown;
      valueFormatter?: (p: { value: unknown; data: unknown }) => string;
    }>;
  }) => (
    <>
      <div>{controles}</div>
      <table>
      <tbody>
        {filas.map((fila, i) => (
          <tr key={i}>
            {columnas.map((columna) => {
              const valor = columna.valueGetter
                ? columna.valueGetter({ data: fila })
                : (fila as Record<string, unknown>)[columna.field ?? ""];
              const texto = columna.valueFormatter
                ? columna.valueFormatter({ value: valor, data: fila })
                : String(valor);
              return <td key={columna.colId ?? columna.field}>{texto}</td>;
            })}
          </tr>
        ))}
      </tbody>
      </table>
    </>
  ),
}));

describe("VistaFlota", () => {
  it("muestra el estado de envío, la versión y el sistema de cada equipo", () => {
    const hace5Minutos = new Date(Date.now() - 5 * 60_000).toISOString();
    const flota: EquipoFlota[] = [
      {
        dispositivo_id: "a",
        etiqueta: "Celular Brisas",
        plataforma: "movil",
        version_app: "1.8.14",
        version_codigo: 59,
        sistema: "Android 14",
        modelo: "SM-A155M",
        sesion_iniciada_en: hace5Minutos,
        ultimo_lote_en: hace5Minutos,
        filas_24h: 1234,
        descartadas_24h: 0,
      },
      {
        dispositivo_id: "b",
        etiqueta: null,
        plataforma: "escritorio",
        version_app: "1.10.8",
        version_codigo: null,
        sistema: "windows",
        modelo: null,
        sesion_iniciada_en: hace5Minutos,
        ultimo_lote_en: null,
        filas_24h: 0,
        descartadas_24h: 0,
      },
    ];
    mocks.datos = flota;
    render(<VistaFlota controles={null} busqueda="" />);

    expect(screen.getByText("Celular Brisas")).toBeTruthy();
    expect(screen.getByText("Al día")).toBeTruthy();
    expect(screen.getByText("Móvil 1.8.14 (59)")).toBeTruthy();
    expect(screen.getByText("Android 14 · SM-A155M")).toBeTruthy();
    expect(screen.getByText(/1.234/)).toBeTruthy();
    expect(screen.getByText("PC sin etiqueta")).toBeTruthy();
    expect(screen.getByText("Sin señal")).toBeTruthy();
    expect(screen.getByText("Escritorio 1.10.8")).toBeTruthy();
    expect(screen.getByText("2 equipos")).toBeTruthy();
  });
});

describe("VistaRendimiento", () => {
  it("pide el último día por defecto y arma porcentaje de errores y tiempos", () => {
    const rendimiento: RendimientoOperacion[] = [
      {
        dispositivo_id: "a",
        etiqueta: "Celular Brisas",
        plataforma: "movil",
        version_app: "1.8.14",
        operacion: "registrar_ingreso",
        llamadas: 12,
        errores: 1,
        p50_ms: 38.5,
        p90_ms: 276.4,
        max_ms: null,
        ultima_en: new Date().toISOString(),
      },
    ];
    mocks.claves = [];
    mocks.datos = rendimiento;
    render(<VistaRendimiento controles={null} busqueda="" />);

    expect(mocks.claves).toContainEqual(["telemetria-rendimiento", 1]);
    expect(screen.getByText("registrar_ingreso")).toBeTruthy();
    expect(screen.getByText("8,3 %")).toBeTruthy();
    expect(screen.getByText("38,5 ms")).toBeTruthy();
    expect(screen.getByText("276 ms")).toBeTruthy();
    expect(screen.getByText("—")).toBeTruthy();
  });
});
