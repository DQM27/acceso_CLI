import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import FormularioEditarUsuario from "./FormularioEditarUsuario";
import type { Usuario } from "../api/usuarios";

const mocks = vi.hoisted(() => ({ editarUsuario: vi.fn(), toastSuccess: vi.fn() }));
vi.mock("../api/usuarios", () => ({ editarUsuario: mocks.editarUsuario }));
vi.mock("sonner", () => ({ toast: { success: mocks.toastSuccess, error: vi.fn() } }));

function usuario(cambios: Partial<Usuario> = {}): Usuario {
  return { id: "u1", cedula: "123456789", nombre: "Ana", rol: "OPERADOR", activo: true, ...cambios };
}

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("FormularioEditarUsuario", () => {
  it("edita nombre y rol; la cédula se ve pero no se cambia", async () => {
    mocks.editarUsuario.mockResolvedValue(undefined);
    const onGuardado = vi.fn();
    render(<FormularioEditarUsuario usuario={usuario()} onGuardado={onGuardado} onCerrar={vi.fn()} />);

    const cedula = screen.getByLabelText("Cédula") as HTMLInputElement;
    expect(cedula.value).toBe("123456789");
    expect(cedula.disabled).toBe(true);

    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "Ana María 2" } });
    fireEvent.change(screen.getByLabelText("Rol"), { target: { value: "ADMINISTRADOR" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar" }));

    await waitFor(() => expect(mocks.editarUsuario).toHaveBeenCalledTimes(1));
    // El nombre sólo admite letras mientras se escribe.
    expect(mocks.editarUsuario).toHaveBeenCalledWith("u1", { nombre: "Ana María", rol: "ADMINISTRADOR" });
    await waitFor(() => expect(onGuardado).toHaveBeenCalled());
    expect(mocks.toastSuccess).toHaveBeenCalledWith("Ana María actualizado.");
  });

  it("a un ROOT no se le cambia el rol", async () => {
    mocks.editarUsuario.mockResolvedValue(undefined);
    render(<FormularioEditarUsuario usuario={usuario({ rol: "ROOT" })} onGuardado={vi.fn()} onCerrar={vi.fn()} />);

    expect((screen.getByLabelText("Rol") as HTMLInputElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Guardar" }));
    await waitFor(() => expect(mocks.editarUsuario).toHaveBeenCalledTimes(1));
    expect(mocks.editarUsuario).toHaveBeenCalledWith("u1", { nombre: "Ana", rol: undefined });
  });

  it("muestra el error y no cierra si falla", async () => {
    mocks.editarUsuario.mockRejectedValue(new Error("El nombre es obligatorio"));
    const onGuardado = vi.fn();
    render(<FormularioEditarUsuario usuario={usuario()} onGuardado={onGuardado} onCerrar={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "Guardar" }));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("El nombre es obligatorio"));
    expect(onGuardado).not.toHaveBeenCalled();
  });
});
