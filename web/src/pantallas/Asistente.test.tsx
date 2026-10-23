import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Asistente from "./Asistente";

const mocks = vi.hoisted(() => ({
  obtenerResumenHoy: vi.fn(),
  consultarLienzo: vi.fn(),
  listarUnidadesOperativas: vi.fn(),
}));
vi.mock("../api/lienzo", () => ({
  obtenerResumenHoy: mocks.obtenerResumenHoy,
  consultarLienzo: mocks.consultarLienzo,
}));
vi.mock("../api/historial", () => ({ listarUnidadesOperativas: mocks.listarUnidadesOperativas }));

function montar() {
  const cliente = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={cliente}>
      <Asistente />
    </QueryClientProvider>,
  );
}

const vencidos = [
  { nombre: "Ana", empresa: "ACME", vencimiento: "2026-10-01", dias_para_vencer: -3, tipo_ingreso: "PRAIND" },
  { nombre: "Beto", empresa: "ACME", vencimiento: "2026-10-02", dias_para_vencer: -2, tipo_ingreso: "PRAIND" },
  { nombre: "Cris", empresa: "OTRA", vencimiento: "2026-09-01", dias_para_vencer: -40, tipo_ingreso: "SWAT" },
];

beforeEach(() => {
  mocks.obtenerResumenHoy.mockResolvedValue({
    hoy: "2026-10-22",
    adentro_contratistas: 19,
    adentro_proveedores: 0,
    ingresos_hoy: 7,
    praind_vencidos: 13,
    praind_por_vencer: 2,
    ingresos_por_dia: [{ dia: "2026-10-22", cantidad: 7 }],
  });
  mocks.consultarLienzo.mockResolvedValue({ filas: vencidos });
  mocks.listarUnidadesOperativas.mockResolvedValue([{ id: "s1", nombre: "Brisas" }]);
});
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

async function abrirVencidos() {
  montar();
  await screen.findByText("Contratistas adentro ahora");
  fireEvent.change(screen.getByLabelText("Vista del lienzo"), { target: { value: "praind-vencidos" } });
  await screen.findByText("Empresas afectadas");
}

describe("Asistente", () => {
  it("muestra el resumen de hoy por omisión, sin pedir un sitio", async () => {
    montar();
    expect(await screen.findByText("Contratistas adentro ahora")).toBeTruthy();
    expect(mocks.obtenerResumenHoy).toHaveBeenCalledWith(undefined);
  });

  it("el chat está maquetado pero apagado: 'próximamente'", () => {
    montar();
    expect(screen.getByText("Próximamente")).toBeTruthy();
    expect((screen.getByLabelText("Mensaje para el asistente") as HTMLInputElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Enviar" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Ver los PRAIND vencidos por empresa" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("al elegir un sitio vuelve a pedir el resumen de ese sitio", async () => {
    montar();
    await screen.findByText("Contratistas adentro ahora");
    await screen.findByRole("option", { name: "Brisas" });
    fireEvent.change(screen.getByLabelText("Sitio"), { target: { value: "s1" } });
    await waitFor(() => expect(mocks.obtenerResumenHoy).toHaveBeenCalledWith("s1"));
  });

  it("una vista del catálogo se valida, se consulta y se dibuja con sus filtros como chips", async () => {
    await abrirVencidos();
    expect(mocks.consultarLienzo).toHaveBeenCalledTimes(1);
    expect(mocks.consultarLienzo.mock.calls[0][0]).toMatchObject({ fuente: "contratistas" });
    expect(screen.getByText("Estado PRAIND: VENCIDA")).toBeTruthy();
    expect((screen.getByRole("button", { name: /Deshacer/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("tocar una barra filtra el lienzo por ella, y Deshacer lo devuelve", async () => {
    await abrirVencidos();
    const zona = document.querySelector("rect.lienzo-zona-seleccionable");
    if (!zona) throw new Error("Ninguna barra acepta clic");
    fireEvent.click(zona);

    await waitFor(() => expect(mocks.consultarLienzo).toHaveBeenCalledTimes(2));
    expect(mocks.consultarLienzo.mock.calls[1][0].filtros).toContainEqual({ campo: "empresa", op: "igual", valor: "ACME" });
    expect(await screen.findByText("Empresa: ACME")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /Deshacer/ }));
    await waitFor(() => expect(screen.queryByText("Empresa: ACME")).toBeNull());
  });

  it("quitar un filtro desde su chip vuelve a consultar sin él", async () => {
    await abrirVencidos();
    fireEvent.click(screen.getByRole("button", { name: "Quitar filtro Estado PRAIND: VENCIDA" }));
    await waitFor(() => expect(mocks.consultarLienzo).toHaveBeenCalledTimes(2));
    expect(mocks.consultarLienzo.mock.calls[1][0].filtros).toEqual([]);
  });

  it("si la consulta falla, avisa con el mensaje del error", async () => {
    mocks.obtenerResumenHoy.mockRejectedValue(new Error("Sólo los administradores del panel pueden usar el lienzo"));
    montar();
    expect((await screen.findByRole("alert")).textContent).toContain("administradores del panel");
  });
});
