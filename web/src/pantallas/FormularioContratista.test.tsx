import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import FormularioContratista from "./FormularioContratista";
import type { ContratistaConEstado } from "../api/contratistas";
import { errorAntesDeEnviar, pidePraind, tiposIngreso } from "./FormularioContratista.logica";

const mocks = vi.hoisted(() => ({
  listarEmpresas: vi.fn(),
  crearEmpresa: vi.fn(),
  crearContratista: vi.fn(),
  editarContratista: vi.fn(),
  toastSuccess: vi.fn(),
}));
vi.mock("../api/contratistas", () => ({
  listarEmpresas: mocks.listarEmpresas,
  crearEmpresa: mocks.crearEmpresa,
  crearContratista: mocks.crearContratista,
  editarContratista: mocks.editarContratista,
}));
vi.mock("sonner", () => ({ toast: { success: mocks.toastSuccess, error: vi.fn() } }));

function montar(onGuardado = vi.fn(), onCerrar = vi.fn(), contratista?: ContratistaConEstado) {
  const cliente = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={cliente}>
      <FormularioContratista contratista={contratista} onGuardado={onGuardado} onCerrar={onCerrar} />
    </QueryClientProvider>,
  );
  return { onGuardado, onCerrar };
}

function guardado(cambios: Partial<ContratistaConEstado> = {}): ContratistaConEstado {
  return {
    id: "c1",
    identificacion: "112340567",
    nombre: "ANA PÉREZ",
    empresa_id: "e2",
    empresa_nombre: "SODEXO",
    tipo_ingreso: "SWAT",
    fecha_vencimiento_praind: null,
    es_personal_ruta: false,
    activo: true,
    empresa_activa: true,
    requiere_praind: false,
    dias_para_vencer: null,
    estado_praind: "NO_REQUIERE",
    estado_acceso: "PERMITIDO",
    adentro_sitio_nombre: null,
    adentro_desde: null,
    ...cambios,
  };
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

  it("avisa antes de enviar con las reglas y los mensajes del núcleo", () => {
    const base = {
      empresaId: "e1",
      cedula: "112340567",
      nombre: "Ana",
      tipo: "SWAT" as const,
      praind: null,
      conAcceso: true,
    };
    const hoy = "2026-10-04";
    expect(errorAntesDeEnviar(base, hoy)).toBeNull();
    expect(errorAntesDeEnviar({ ...base, empresaId: "" }, hoy)).toBe("Elija la empresa");
    expect(errorAntesDeEnviar({ ...base, cedula: "12" }, hoy)).toBe(
      "La cédula debe tener sólo números, entre 9 y 13 dígitos",
    );
    expect(errorAntesDeEnviar({ ...base, tipo: "PRAIND" }, hoy)).toBe("Fecha PRAIND requerida");
    expect(errorAntesDeEnviar({ ...base, tipo: "PRAIND", praind: "2026-10-03" }, hoy)).toBe(
      "El PRAIND está vencido — ingrese una fecha vigente",
    );
    expect(errorAntesDeEnviar({ ...base, tipo: "POR_CORREO" }, hoy)).toContain("ya no es un tipo de contratista");
    // Los datos van primero: con la cédula mal y sin empresa, avisa la cédula.
    expect(errorAntesDeEnviar({ ...base, cedula: "", empresaId: "" }, hoy)).toBe("La cédula es obligatoria");
  });

  it("los tipos los decide el núcleo y salen con el texto que se lee", () => {
    expect(tiposIngreso().map((t) => t.etiqueta)).toEqual(["PRAIND", "IN HOUSE", "SWAT"]);
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
    // La cédula y el nombre se limpian mientras se escribe (solo dígitos / solo letras, en mayúscula).
    expect(mocks.crearContratista).toHaveBeenCalledWith({
      cedula: "112340567",
      nombre: "ANA PÉREZ",
      empresa_id: "e1",
      tipo_ingreso: "PRAIND",
      fecha_vencimiento_praind: null,
      con_acceso: false,
      es_personal_ruta: false,
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
    // Con los datos válidos (PRAIND incluido), lo único que falta es la empresa.
    fireEvent.change(screen.getByLabelText("Fecha de vencimiento PRAIND"), { target: { value: "2030-01-31" } });
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

  it("edita: arranca con lo guardado y manda los cambios con el id", async () => {
    mocks.editarContratista.mockResolvedValue({ nombre: "ANA MARÍA PÉREZ" });
    const { onGuardado } = montar(vi.fn(), vi.fn(), guardado());
    await waitFor(() => expect(screen.getByRole("option", { name: "SODEXO" })).toBeTruthy());

    expect(screen.getByText("Editar contratista")).toBeTruthy();
    expect((screen.getByLabelText("Cédula") as HTMLInputElement).value).toBe("112340567");
    expect((screen.getByLabelText(/^Empresa/) as HTMLSelectElement).value).toBe("e2");
    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "Ana María Pérez" } });
    fireEvent.change(screen.getByLabelText("Cédula"), { target: { value: "200000002" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar" }));

    await waitFor(() => expect(mocks.editarContratista).toHaveBeenCalledTimes(1));
    expect(mocks.editarContratista).toHaveBeenCalledWith("c1", {
      cedula: "200000002",
      nombre: "ANA MARÍA PÉREZ",
      empresa_id: "e2",
      tipo_ingreso: "SWAT",
      fecha_vencimiento_praind: null,
      con_acceso: true,
      es_personal_ruta: false,
    });
    expect(mocks.crearContratista).not.toHaveBeenCalled();
    await waitFor(() => expect(onGuardado).toHaveBeenCalled());
    expect(mocks.toastSuccess).toHaveBeenCalledWith("ANA MARÍA PÉREZ actualizado.");
  });

  it("edita uno viejo POR CORREO sin obligar a cambiarle el tipo", async () => {
    mocks.editarContratista.mockResolvedValue({ nombre: "LUIS" });
    montar(vi.fn(), vi.fn(), guardado({ tipo_ingreso: "POR_CORREO", nombre: "LUIS" }));
    await waitFor(() => expect(screen.getByRole("option", { name: "SODEXO" })).toBeTruthy());

    expect(screen.getByRole("option", { name: "POR CORREO (retirado)" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Guardar" }));
    await waitFor(() => expect(mocks.editarContratista).toHaveBeenCalledTimes(1));
    expect(mocks.editarContratista.mock.calls[0][1]).toMatchObject({ tipo_ingreso: "POR_CORREO" });
  });

  it("personal de ruta: la casilla aparece sólo para los tipos que la admiten y pide PRAIND", async () => {
    montar(vi.fn(), vi.fn(), guardado());
    await waitFor(() => expect(screen.getByRole("option", { name: "SODEXO" })).toBeTruthy());

    // SWAT no la admite.
    expect(screen.queryByLabelText("Personal de ruta")).toBeNull();
    fireEvent.change(screen.getByLabelText("Tipo de ingreso"), { target: { value: "IN_HOUSE" } });
    expect(screen.getByLabelText("Personal de ruta")).toBeTruthy();
  });
});

