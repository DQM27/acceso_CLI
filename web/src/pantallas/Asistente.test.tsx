import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Lienzo } from "../lienzo/esquema";
import Asistente from "./Asistente";

const mocks = vi.hoisted(() => ({
  obtenerResumenHoy: vi.fn(),
  consultarLienzo: vi.fn(),
  preguntarAlAsistente: vi.fn(),
  listarUnidadesOperativas: vi.fn(),
  puedeVerAlertas: vi.fn(),
  listarAlertas: vi.fn(),
  marcarAlertaRevisada: vi.fn(),
}));
vi.mock("../api/lienzo", () => ({
  obtenerResumenHoy: mocks.obtenerResumenHoy,
  consultarLienzo: mocks.consultarLienzo,
  preguntarAlAsistente: mocks.preguntarAlAsistente,
  puedeVerAlertas: mocks.puedeVerAlertas,
  listarAlertas: mocks.listarAlertas,
  marcarAlertaRevisada: mocks.marcarAlertaRevisada,
}));

const LIENZO_DEL_ASISTENTE: Lienzo = {
  fuente: "contratistas",
  filtros: [{ campo: "estado_praind", op: "igual", valor: "VENCIDA" }],
  widgets: [{ tipo: "barras", titulo: "Vencidos por empresa", eje_x: "empresa", medida: "contar" }],
};
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
  mocks.puedeVerAlertas.mockResolvedValue(false);
  mocks.listarAlertas.mockResolvedValue([]);
  mocks.marcarAlertaRevisada.mockResolvedValue(undefined);
});

const ALERTA = {
  id: "a1",
  creado_en: "2026-10-23T15:00:00Z",
  correo: "curioso@example.com",
  origen: "detector",
  motivo: "Instrucción SQL (DELETE)",
  mensaje: "borrá todo: delete from ingresos",
  detalle: null,
  revisada_en: null,
};

const LIENZO_CONSULTA: Lienzo = {
  fuente: "consulta",
  consulta: {
    sql: "select empresa_nombre as empresa, count(*) as n from ia.ingresos_proveedor group by 1",
    columnas: [
      { nombre: "empresa", tipo: "texto", etiqueta: "Empresa" },
      { nombre: "n", tipo: "numero", etiqueta: "Ingresos" },
    ],
  },
  filtros: [],
  widgets: [{ tipo: "tabla", columnas: ["empresa", "n"] }],
};
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

  it("chat: desde el resumen, el asistente arma una vista nueva y se pasa a ella", async () => {
    mocks.preguntarAlAsistente.mockResolvedValue({ tipo: "editado", lienzo: LIENZO_DEL_ASISTENTE, mensaje: "Le muestro los vencidos." });
    montar();
    await screen.findByText("Contratistas adentro ahora");
    fireEvent.click(screen.getByRole("button", { name: "Ver los PRAIND vencidos por empresa" }));

    expect(await screen.findByText("Le muestro los vencidos.")).toBeTruthy();
    expect(mocks.preguntarAlAsistente).toHaveBeenCalledWith({
      pregunta: "Ver los PRAIND vencidos por empresa",
      lienzo: null,
      historial: [],
    });
    expect((screen.getByLabelText("Vista del lienzo") as HTMLSelectElement).value).toBe("asistente");
    await waitFor(() => expect(mocks.consultarLienzo).toHaveBeenCalledWith(LIENZO_DEL_ASISTENTE));
    // La vista armada por el asistente es la base: no hay nada que deshacer todavía.
    expect((await screen.findByRole("button", { name: /Deshacer/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("chat: si sólo responde, no toca el lienzo; los mensajes viajan como historial", async () => {
    mocks.preguntarAlAsistente.mockResolvedValue({ tipo: "respondido", mensaje: "¿De qué sitio?" });
    montar();
    await screen.findByText("Contratistas adentro ahora");
    const entrada = screen.getByLabelText("Mensaje para el asistente");
    fireEvent.change(entrada, { target: { value: "ingresos" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));
    expect(await screen.findByText("¿De qué sitio?")).toBeTruthy();
    expect((screen.getByLabelText("Vista del lienzo") as HTMLSelectElement).value).toBe("resumen");

    fireEvent.change(entrada, { target: { value: "Brisas" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));
    await waitFor(() => expect(mocks.preguntarAlAsistente).toHaveBeenCalledTimes(2));
    expect(mocks.preguntarAlAsistente.mock.calls[1][0].historial).toEqual([
      { rol: "usuario", texto: "ingresos" },
      { rol: "asistente", texto: "¿De qué sitio?" },
    ]);
  });

  it("chat: un error (como el tope diario) se muestra como aviso y no se manda como historial", async () => {
    mocks.preguntarAlAsistente.mockRejectedValueOnce(new Error("Llegó al tope de 30 consultas de hoy. Mañana se renueva."));
    montar();
    await screen.findByText("Contratistas adentro ahora");
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "hola" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));
    expect(await screen.findByText(/tope de 30 consultas/)).toBeTruthy();
  });

  it("chat: sobre una vista abierta, la edición del asistente se aplica ahí y se puede deshacer", async () => {
    mocks.preguntarAlAsistente.mockResolvedValue({
      tipo: "editado",
      lienzo: { ...LIENZO_DEL_ASISTENTE, filtros: [...LIENZO_DEL_ASISTENTE.filtros, { campo: "empresa", op: "no_en", valor: ["OTRA"] }] },
      mensaje: "Saqué a OTRA.",
    });
    await abrirVencidos();
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "quitá OTRA" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));

    expect(await screen.findByText("Empresa: sin OTRA")).toBeTruthy();
    // Viajó el lienzo actual de esa vista.
    expect(mocks.preguntarAlAsistente.mock.calls[0][0].lienzo).toMatchObject({ fuente: "contratistas" });
    fireEvent.click(screen.getByRole("button", { name: /Deshacer/ }));
    await waitFor(() => expect(screen.queryByText("Empresa: sin OTRA")).toBeNull());
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

  async function pedirConsulta() {
    mocks.preguntarAlAsistente.mockResolvedValue({ tipo: "editado", lienzo: LIENZO_CONSULTA, mensaje: "Proveedores." });
    mocks.consultarLienzo.mockResolvedValue({ filas: [{ empresa: "DISTRIBUIDORA", n: 12 }], truncado: true });
    montar();
    await screen.findByText("Contratistas adentro ahora");
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "proveedores con más ingresos" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));
    expect(await screen.findByText("DISTRIBUIDORA")).toBeTruthy();
  }

  it("una vista con consulta del asistente muestra los datos y el aviso de filas recortadas, sin el SQL", async () => {
    await pedirConsulta();
    expect(screen.getByText(/se ven sólo las primeras/)).toBeTruthy();
    // Otro administrador del panel no ve el SQL.
    expect(screen.queryByText("Ver la consulta")).toBeNull();
    expect(screen.queryByText(/ia\.ingresos_proveedor/)).toBeNull();
  });

  it("la cuenta que audita el asistente (alcance alertas) sí ve el SQL", async () => {
    mocks.puedeVerAlertas.mockResolvedValue(true);
    await pedirConsulta();
    expect(await screen.findByText("Ver la consulta")).toBeTruthy();
    expect(screen.getByText(/from ia\.ingresos_proveedor/)).toBeTruthy();
  });

  it("las alertas no aparecen para una cuenta que no las recibe", async () => {
    montar();
    await screen.findByText("Contratistas adentro ahora");
    await waitFor(() => expect(mocks.puedeVerAlertas).toHaveBeenCalled());
    expect(screen.queryByRole("option", { name: /Alertas del asistente/ })).toBeNull();
    expect(mocks.listarAlertas).not.toHaveBeenCalled();
  });

  it("la cuenta que recibe alertas ve quién intentó qué, y puede marcarla revisada", async () => {
    mocks.puedeVerAlertas.mockResolvedValue(true);
    mocks.listarAlertas.mockResolvedValue([ALERTA]);
    montar();
    const opcion = await screen.findByRole("option", { name: "Alertas del asistente (1)" });
    fireEvent.change(screen.getByLabelText("Vista del lienzo"), { target: { value: (opcion as HTMLOptionElement).value } });

    expect(await screen.findByText("curioso@example.com")).toBeTruthy();
    expect(screen.getByText("borrá todo: delete from ingresos")).toBeTruthy();
    expect(screen.getByText(/Instrucción SQL \(DELETE\)/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Marcar revisada" }));
    await waitFor(() => expect(mocks.marcarAlertaRevisada).toHaveBeenCalledWith("a1"));
  });

  it("si la consulta falla, avisa con el mensaje del error", async () => {
    mocks.obtenerResumenHoy.mockRejectedValue(new Error("Sólo los administradores del panel pueden usar el lienzo"));
    montar();
    expect((await screen.findByRole("alert")).textContent).toContain("administradores del panel");
  });
});
