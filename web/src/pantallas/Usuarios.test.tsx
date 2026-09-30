import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Usuarios from "./Usuarios";

// useAutoRefresh mockeado entero -- ya tiene su propio test
// (useAutoRefresh.test.ts); acá sólo importa que no explote al montarse
// (usa el canal Realtime de Supabase por dentro, no relevante para esto).
vi.mock("../componentes/useAutoRefresh", () => ({ useAutoRefresh: vi.fn() }));

const mocks = vi.hoisted(() => ({
  listarUsuarios: vi.fn(),
  actualizarActivoUsuario: vi.fn(),
  crearUsuario: vi.fn(),
}));
vi.mock("../api/usuarios", () => mocks);

beforeEach(() => {
  vi.clearAllMocks();
  mocks.listarUsuarios.mockResolvedValue({ filas: [], truncado: false });
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("Usuarios -- alta", () => {
  it("crea el usuario sin unidad operativa: es global", async () => {
    mocks.crearUsuario.mockResolvedValue({ usuario_id: "u1", cedula: "123456789", password_temporal: "ABC" });

    // La pantalla carga con React Query: necesita su proveedor (sin reintentos,
    // para que un fallo de la consulta de presencia no alargue la prueba).
    const cliente = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={cliente}>
        <Usuarios />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(mocks.listarUsuarios).toHaveBeenCalled());

    fireEvent.click(screen.getByText("+ Nuevo"));
    fireEvent.change(screen.getByLabelText("Cédula"), { target: { value: "123456789" } });
    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "Alguien" } });
    fireEvent.click(screen.getByRole("button", { name: "Crear usuario" }));

    await waitFor(() => expect(mocks.crearUsuario).toHaveBeenCalledTimes(1));
    expect(mocks.crearUsuario.mock.calls[0][0]).toEqual({
      cedula: "123456789",
      nombre: "Alguien",
      rol: "OPERADOR",
    });
  });
});
