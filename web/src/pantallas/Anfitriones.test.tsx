import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Anfitriones from "./Anfitriones";

vi.mock("../componentes/useAutoRefresh", () => ({ useAutoRefresh: vi.fn() }));

const mocks = vi.hoisted(() => ({
  listarAnfitriones: vi.fn(),
  crearAnfitrion: vi.fn(),
  restablecerAnfitrion: vi.fn(),
  cambiarEstadoAnfitrion: vi.fn(),
}));
vi.mock("../api/anfitriones", () => mocks);

function montar() {
  const cliente = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={cliente}>
      <Anfitriones />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.listarAnfitriones.mockResolvedValue([]);
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("Anfitriones -- alta", () => {
  it("crea con el correo normalizado y el nombre en mayúscula, y muestra el código una vez", async () => {
    mocks.crearAnfitrion.mockResolvedValue({
      correo: "ana@empresa.example",
      nombre: "ANA MORA",
      codigo: "ABCDE23456",
      vence: "2026-10-09T18:00:00Z",
    });
    montar();
    await waitFor(() => expect(mocks.listarAnfitriones).toHaveBeenCalled());

    fireEvent.click(screen.getByText("+ Nuevo"));
    fireEvent.change(screen.getByLabelText("Correo de la empresa"), { target: { value: " Ana@Empresa.Example " } });
    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "ana mora" } });
    fireEvent.click(screen.getByRole("button", { name: "Crear anfitrión" }));

    await waitFor(() => expect(mocks.crearAnfitrion).toHaveBeenCalledTimes(1));
    expect(mocks.crearAnfitrion.mock.calls[0][0]).toEqual({ correo: "ana@empresa.example", nombre: "ANA MORA" });
    expect(await screen.findByText("ABCDE-23456")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Ya lo copié" }));
    await waitFor(() => expect(screen.queryByText("ABCDE-23456")).toBeNull());
  });

  it("muestra el motivo si el servidor rechaza el alta", async () => {
    mocks.crearAnfitrion.mockRejectedValue(new Error("Ya hay un anfitrión con ese correo."));
    montar();
    await waitFor(() => expect(mocks.listarAnfitriones).toHaveBeenCalled());

    fireEvent.click(screen.getByText("+ Nuevo"));
    fireEvent.change(screen.getByLabelText("Correo de la empresa"), { target: { value: "ana@empresa.example" } });
    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "Ana" } });
    fireEvent.click(screen.getByRole("button", { name: "Crear anfitrión" }));

    expect((await screen.findByRole("alert")).textContent).toContain("Ya hay un anfitrión");
  });
});
