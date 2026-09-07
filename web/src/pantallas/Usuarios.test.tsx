import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Usuarios from "./Usuarios";

// useAutoRefresh mockeado entero -- ya tiene su propio test
// (useAutoRefresh.test.ts); acá sólo importa que no explote al montarse
// (usa el canal Realtime de Supabase por dentro, no relevante para esto).
vi.mock("../componentes/useAutoRefresh", () => ({ useAutoRefresh: vi.fn() }));

const mocks = vi.hoisted(() => ({
  listarUsuarios: vi.fn(),
  listarSitios: vi.fn(),
  actualizarActivoUsuario: vi.fn(),
  crearUsuario: vi.fn(),
}));
vi.mock("../api/usuarios", () => mocks);

function deferido<T>() {
  let resolver!: (valor: T) => void;
  const promesa = new Promise<T>((resolve) => {
    resolver = resolve;
  });
  return { promesa, resolver };
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.listarUsuarios.mockResolvedValue({ filas: [], truncado: false });
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("Usuarios -- abrirModal", () => {
  it("una apertura vieja del modal no pisa el sitio que ya eligió una apertura más nueva", async () => {
    // Reproduce la condición de carrera: se abre, se cierra y se vuelve a
    // abrir el modal antes de que resuelva la PRIMERA llamada a
    // listarSitios -- y esa primera llamada resuelve DESPUÉS que la
    // segunda (red desordenada, no es un orden garantizado).
    const primeraApertura = deferido<{ id: string; nombre: string }[]>();
    const segundaApertura = deferido<{ id: string; nombre: string }[]>();
    mocks.listarSitios
      .mockReturnValueOnce(primeraApertura.promesa)
      .mockReturnValueOnce(segundaApertura.promesa);
    mocks.crearUsuario.mockResolvedValue(undefined);

    render(<Usuarios />);
    await waitFor(() => expect(mocks.listarUsuarios).toHaveBeenCalled());

    fireEvent.click(screen.getByText("+ Nuevo"));
    fireEvent.click(screen.getByText("Cancelar"));
    fireEvent.click(screen.getByText("+ Nuevo"));

    // La apertura NUEVA (segunda) resuelve primero...
    segundaApertura.resolver([{ id: "sitio-nuevo", nombre: "Sitio nuevo" }]);
    await Promise.resolve();
    // ...y la VIEJA (primera) resuelve después -- sin el fix, esto pisaba
    // `sitioId` con el sitio de la apertura ya descartada.
    primeraApertura.resolver([{ id: "sitio-viejo", nombre: "Sitio viejo" }]);
    await Promise.resolve();

    fireEvent.change(screen.getByLabelText("Cédula"), { target: { value: "123456789" } });
    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "Alguien" } });
    fireEvent.click(screen.getByRole("button", { name: "Crear usuario" }));

    await waitFor(() => expect(mocks.crearUsuario).toHaveBeenCalledTimes(1));
    expect(mocks.crearUsuario.mock.calls[0][0].sitio_id).toBe("sitio-nuevo");
  });
});
