import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import FormularioContratista from "./FormularioContratista";
import { TIPOS_INGRESO, errorAntesDeEnviar, pidePraind } from "./FormularioContratista.logica";

const mocks = vi.hoisted(() => ({
  listarEmpresas: vi.fn(),
  crearEmpresa: vi.fn(),
  crearContratista: vi.fn(),
  toastSuccess: vi.fn(),
}));
vi.mock("../api/contratistas", () => ({
  listarEmpresas: mocks.listarEmpresas,
  crearEmpresa: mocks.crearEmpresa,
  crearContratista: mocks.crearContratista,
}));
vi.mock("sonner", () => ({ toast: { success: mocks.toastSuccess, error: vi.fn() } }));

function montar(onGuardado = vi.fn(), onCerrar = vi.fn()) {
  const cliente = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={cliente}>
      <FormularioContratista onGuardado={onGuardado} onCerrar={onCerrar} />
    </QueryClientProvider>,
  );
  return { onGuardado, onCerrar };
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.listarEmpresas.mockResolvedValue([
    { id: "e1", nombre: "BAC" },
    { id: "e2", nombre: "SODEXO" },
  ]);
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("lógica del formulario", () => {
  it("el PRAIND se pide solo para PRAIND e IN HOUSE y solo si la persona tendrá acceso", () => {
    expect(pidePraind("PRAIND", true)).toBe(true);
    expect(pidePraind("IN_HOUSE", true)).toBe(true);
    expect(pidePraind("POR_CORREO", true)).toBe(false);
    expect(pidePraind("SWAT", true)).toBe(false);
    expect(pidePraind("PRAIND", false)).toBe(false);
  });

  it("solo exige elegir la empresa antes de enviar", () => {
    expect(errorAntesDeEnviar({ empresaId: "" })).toBe("Elija la empresa");
    expect(errorAntesDeEnviar({ empresaId: "e1" })).toBeNull();
  });

  it("los tipos salen con el texto que se lee, no el valor interno", () => {
    expect(TIPOS_INGRESO.map((t) => t.etiqueta)).toEqual(["PRAIND", "IN HOUSE", "SWAT"]);
  });
});

describe("FormularioContratista", () => {
  it("crea un contratista bloqueado sin pedir el PRAIND", async () => {
    mocks.crearContratista.mockResolvedValue({ nombre: "ANA PÉREZ" });
    const { onGuardado } = montar();
    await waitFor(() => expect(screen.getByRole("option", { name: "BAC" })).toBeTruthy());

    expect(screen.getByText("Fecha de vencimiento PRAIND")).toBeTruthy();
    fireEvent.click(screen.getByLabelText("Crear con el acceso denegado"));
    expect(screen.queryByText("Fecha de vencimiento PRAIND")).toBeNull();

    fireEvent.change(screen.getByLabelText("Cédula"), { target: { value: "1-1234-0567abc" } });
    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "Ana Pérez 9" } });
    fireEvent.change(screen.getByLabelText(/^Empresa/), { target: { value: "e1" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar" }));

    await waitFor(() => expect(mocks.crearContratista).toHaveBeenCalledTimes(1));
    // La cédula y el nombre se limpian mientras se escribe (solo dígitos / solo letras).
    expect(mocks.crearContratista).toHaveBeenCalledWith({
      cedula: "112340567",
      nombre: "Ana Pérez",
      empresa_id: "e1",
      tipo_ingreso: "PRAIND",
      fecha_vencimiento_praind: null,
      con_acceso: false,
    });
    await waitFor(() => expect(onGuardado).toHaveBeenCalled());
    expect(mocks.toastSuccess).toHaveBeenCalledWith("ANA PÉREZ registrado con el acceso denegado.");
  });

  it("manda el PRAIND cuando la persona tendrá acceso", async () => {
    mocks.crearContratista.mockResolvedValue({ nombre: "LUIS" });
    montar();
    await waitFor(() => expect(screen.getByRole("option", { name: "SODEXO" })).toBeTruthy());

    fireEvent.change(screen.getByLabelText("Cédula"), { target: { value: "223450678" } });
    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "Luis" } });
    fireEvent.change(screen.getByLabelText(/^Empresa/), { target: { value: "e2" } });
    fireEvent.change(screen.getByLabelText("Fecha de vencimiento PRAIND"), { target: { value: "2030-01-31" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar" }));

    await waitFor(() => expect(mocks.crearContratista).toHaveBeenCalledTimes(1));
    expect(mocks.crearContratista.mock.calls[0][0]).toMatchObject({
      tipo_ingreso: "PRAIND",
      fecha_vencimiento_praind: "2030-01-31",
      con_acceso: true,
    });
  });

  it("muestra el mensaje de la base y no cierra si falla", async () => {
    mocks.crearContratista.mockRejectedValue(new Error("La cédula del contratista ya existe"));
    const { onGuardado } = montar();
    await waitFor(() => expect(screen.getByRole("option", { name: "BAC" })).toBeTruthy());

    fireEvent.click(screen.getByLabelText("Crear con el acceso denegado"));
    fireEvent.change(screen.getByLabelText("Cédula"), { target: { value: "112340567" } });
    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "Ana" } });
    fireEvent.change(screen.getByLabelText(/^Empresa/), { target: { value: "e1" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar" }));

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("La cédula del contratista ya existe"));
    expect(onGuardado).not.toHaveBeenCalled();
  });

  it("pide elegir la empresa antes de enviar", async () => {
    montar();
    await waitFor(() => expect(screen.getByRole("option", { name: "BAC" })).toBeTruthy());

    fireEvent.change(screen.getByLabelText("Cédula"), { target: { value: "112340567" } });
    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "Ana" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar" }));

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Elija la empresa"));
    expect(mocks.crearContratista).not.toHaveBeenCalled();
  });

  it("crea una empresa nueva desde el formulario y la deja elegida", async () => {
    mocks.crearEmpresa.mockResolvedValue({ id: "e3", nombre: "NUEVA SA" });
    montar();
    await waitFor(() => expect(screen.getByRole("option", { name: "BAC" })).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Crear una empresa nueva" }));
    fireEvent.change(screen.getByPlaceholderText("Nombre de la empresa"), { target: { value: "nueva sa" } });
    fireEvent.click(screen.getByRole("button", { name: "Crear" }));

    await waitFor(() => expect(mocks.crearEmpresa).toHaveBeenCalledWith("nueva sa"));
    await waitFor(() => expect((screen.getByLabelText(/^Empresa/) as HTMLSelectElement).value).toBe("e3"));
  });
});
