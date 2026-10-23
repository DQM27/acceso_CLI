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
  listarVistasGuardadas: vi.fn(),
  guardarVista: vi.fn(),
  borrarVista: vi.fn(),
  exportarFilas: vi.fn(),
  contarConsultasDeHoy: vi.fn(),
}));
vi.mock("../api/lienzo", () => ({
  obtenerResumenHoy: mocks.obtenerResumenHoy,
  consultarLienzo: mocks.consultarLienzo,
  preguntarAlAsistente: mocks.preguntarAlAsistente,
  puedeVerAlertas: mocks.puedeVerAlertas,
  listarAlertas: mocks.listarAlertas,
  marcarAlertaRevisada: mocks.marcarAlertaRevisada,
  listarVistasGuardadas: mocks.listarVistasGuardadas,
  guardarVista: mocks.guardarVista,
  borrarVista: mocks.borrarVista,
  contarConsultasDeHoy: mocks.contarConsultasDeHoy,
  TOPE_DIARIO_ASISTENTE: 30,
}));
vi.mock("../lib/supabase", () => ({
  supabase: { auth: { getUser: () => Promise.resolve({ data: { user: { email: "admin@example.com" } } }) } },
}));
vi.mock("../lienzo/exportar", () => ({ exportarFilas: mocks.exportarFilas }));

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
  mocks.listarVistasGuardadas.mockResolvedValue([]);
  mocks.guardarVista.mockResolvedValue("v1");
  mocks.borrarVista.mockResolvedValue(undefined);
  mocks.exportarFilas.mockResolvedValue(undefined);
  mocks.contarConsultasDeHoy.mockResolvedValue(3);
  sessionStorage.clear();
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

  it("chat: muestra las consultas de hoy contra el tope y se actualiza después de cada pedido", async () => {
    mocks.preguntarAlAsistente.mockResolvedValue({ tipo: "respondido", mensaje: "ok" });
    montar();
    expect(await screen.findByText("3 de 30 hoy")).toBeTruthy();
    mocks.contarConsultasDeHoy.mockResolvedValue(4);
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "hola" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));
    expect(await screen.findByText("4 de 30 hoy")).toBeTruthy();
    expect(screen.queryByText("Beta")).toBeNull();
  });

  it("chat: Nueva conversación borra el chat y el próximo pedido va sin historial", async () => {
    mocks.preguntarAlAsistente.mockResolvedValue({ tipo: "respondido", mensaje: "Respuesta vieja" });
    montar();
    await screen.findByText("Contratistas adentro ahora");
    const nueva = screen.getByRole("button", { name: "Nueva conversación" }) as HTMLButtonElement;
    expect(nueva.disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "primero" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));
    expect(await screen.findByText("Respuesta vieja")).toBeTruthy();

    fireEvent.click(nueva);
    expect(screen.queryByText("Respuesta vieja")).toBeNull();
    // Vuelven las sugerencias del principio.
    expect(screen.getByRole("button", { name: "Ver los PRAIND vencidos por empresa" })).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "segundo" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));
    await waitFor(() => expect(mocks.preguntarAlAsistente).toHaveBeenCalledTimes(2));
    expect(mocks.preguntarAlAsistente.mock.calls[1][0].historial).toEqual([]);
  });

  it("chat: la conversación sigue ahí al volver a la pantalla (misma pestaña)", async () => {
    mocks.preguntarAlAsistente.mockResolvedValue({ tipo: "respondido", mensaje: "Me acuerdo de esto" });
    montar();
    await screen.findByText("Contratistas adentro ahora");
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "algo" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));
    expect(await screen.findByText("Me acuerdo de esto")).toBeTruthy();

    cleanup();
    montar();
    expect(await screen.findByText("Me acuerdo de esto")).toBeTruthy();
    expect(screen.getByText("algo")).toBeTruthy();
  });

  it("chat: si falla la red o el modelo, Reintentar vuelve a pedir lo mismo sin reescribirlo", async () => {
    mocks.preguntarAlAsistente
      .mockRejectedValueOnce(new Error("El asistente no respondió. Intente de nuevo en un momento."))
      .mockResolvedValueOnce({ tipo: "respondido", mensaje: "Ahora sí" });
    montar();
    await screen.findByText("Contratistas adentro ahora");
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "ingresos de hoy" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));
    fireEvent.click(await screen.findByRole("button", { name: /Reintentar/ }));

    expect(await screen.findByText("Ahora sí")).toBeTruthy();
    expect(mocks.preguntarAlAsistente.mock.calls[1][0].pregunta).toBe("ingresos de hoy");
    expect(mocks.preguntarAlAsistente.mock.calls[1][0].historial).toEqual([]);
    // El aviso del error se fue y el pedido no se duplicó.
    expect(screen.queryByText(/no respondió/)).toBeNull();
    expect(screen.getAllByText("ingresos de hoy")).toHaveLength(1);
  });

  it("chat: Enter envía y Shift+Enter no (es salto de línea)", async () => {
    mocks.preguntarAlAsistente.mockResolvedValue({ tipo: "respondido", mensaje: "ok" });
    montar();
    await screen.findByText("Contratistas adentro ahora");
    const caja = screen.getByLabelText("Mensaje para el asistente");
    fireEvent.change(caja, { target: { value: "ingresos de hoy" } });
    fireEvent.keyDown(caja, { key: "Enter", shiftKey: true });
    expect(mocks.preguntarAlAsistente).not.toHaveBeenCalled();
    fireEvent.keyDown(caja, { key: "Enter" });
    await waitFor(() => expect(mocks.preguntarAlAsistente).toHaveBeenCalledTimes(1));
    expect(mocks.preguntarAlAsistente.mock.calls[0][0].pregunta).toBe("ingresos de hoy");
    // Se vacía al enviar.
    expect((caja as HTMLTextAreaElement).value).toBe("");
  });

  it("chat: un error (como el tope diario) se muestra como aviso y no se manda como historial", async () => {
    mocks.preguntarAlAsistente.mockRejectedValueOnce(new Error("Llegó al tope de 30 consultas de hoy. Mañana se renueva."));
    montar();
    await screen.findByText("Contratistas adentro ahora");
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "hola" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));
    expect(await screen.findByText(/tope de 30 consultas/)).toBeTruthy();
    // Con el tope no tiene sentido reintentar.
    expect(screen.queryByRole("button", { name: /Reintentar/ })).toBeNull();
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

  it("chat: «¿qué podés hacer?» abre la guía sin gastar una consulta, y un ejemplo de la guía se envía", async () => {
    mocks.preguntarAlAsistente.mockResolvedValue({ tipo: "respondido", mensaje: "ok" });
    montar();
    await screen.findByText("Contratistas adentro ahora");
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "¿Qué podés hacer?" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));

    expect(await screen.findByRole("heading", { name: "¿Qué puedo pedir?" })).toBeTruthy();
    expect(mocks.preguntarAlAsistente).not.toHaveBeenCalled();
    // Sin vista abierta (resumen) no hay ideas "Sobre esta vista".
    expect(screen.queryByText("Sobre esta vista")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Ingresos de esta semana por sitio" }));
    await waitFor(() => expect(mocks.preguntarAlAsistente).toHaveBeenCalledTimes(1));
    expect(mocks.preguntarAlAsistente.mock.calls[0][0].pregunta).toBe("Ingresos de esta semana por sitio");
    // Al pedir, la guía se cierra y vuelve el chat.
    expect(screen.queryByRole("heading", { name: "¿Qué puedo pedir?" })).toBeNull();
  });

  it("chat: el botón ? abre la guía; sobre una vista abierta trae ideas para esa vista", async () => {
    await abrirVencidos();
    fireEvent.click(screen.getByRole("button", { name: "¿Qué puedo pedir?" }));
    expect(await screen.findByText("Sobre esta vista")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Volver al chat" }));
    expect(screen.queryByText("Sobre esta vista")).toBeNull();
  });

  it("chat: las sugerencias del asistente salen sólo bajo su última respuesta y se envían al tocarlas", async () => {
    mocks.preguntarAlAsistente
      .mockResolvedValueOnce({
        tipo: "editado",
        lienzo: LIENZO_DEL_ASISTENTE,
        mensaje: "Le muestro los vencidos.",
        sugerencias: ["Agrupar por sitio"],
      })
      .mockResolvedValueOnce({ tipo: "respondido", mensaje: "Listo." });
    montar();
    await screen.findByText("Contratistas adentro ahora");
    fireEvent.change(screen.getByLabelText("Mensaje para el asistente"), { target: { value: "vencidos" } });
    fireEvent.click(screen.getByRole("button", { name: "Enviar" }));

    const sugerencia = await screen.findByRole("button", { name: "Agrupar por sitio" });
    fireEvent.click(sugerencia);
    await waitFor(() => expect(mocks.preguntarAlAsistente).toHaveBeenCalledTimes(2));
    expect(mocks.preguntarAlAsistente.mock.calls[1][0].pregunta).toBe("Agrupar por sitio");
    expect(await screen.findByText("Listo.")).toBeTruthy();
    // La respuesta nueva no trae sugerencias: las viejas ya no se muestran.
    expect(screen.queryByRole("button", { name: "Agrupar por sitio" })).toBeNull();
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

  it("Rehacer vuelve a aplicar lo deshecho, y una edición nueva lo descarta", async () => {
    await abrirVencidos();
    fireEvent.click(screen.getByRole("button", { name: "Quitar filtro Estado PRAIND: VENCIDA" }));
    await waitFor(() => expect(screen.queryByText("Estado PRAIND: VENCIDA")).toBeNull());
    const rehacer = () => screen.getByRole("button", { name: /Rehacer/ }) as HTMLButtonElement;
    expect(rehacer().disabled).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: /Deshacer/ }));
    expect(await screen.findByText("Estado PRAIND: VENCIDA")).toBeTruthy();
    expect(rehacer().disabled).toBe(false);
    fireEvent.click(rehacer());
    await waitFor(() => expect(screen.queryByText("Estado PRAIND: VENCIDA")).toBeNull());
    expect(rehacer().disabled).toBe(true);
  });

  it("Guardar pide un nombre, guarda la vista y queda abierta en Mis vistas guardadas", async () => {
    await abrirVencidos();
    mocks.listarVistasGuardadas.mockResolvedValue([
      { id: "v1", nombre: "Vencidos de octubre", lienzo: LIENZO_DEL_ASISTENTE, actualizado_en: "2026-10-23T10:00:00Z" },
    ]);
    fireEvent.click(screen.getByRole("button", { name: /Guardar/ }));
    const nombre = screen.getByLabelText("Nombre de la vista");
    fireEvent.change(nombre, { target: { value: "  Vencidos de octubre " } });
    fireEvent.submit(nombre.closest("form") as HTMLFormElement);

    await waitFor(() => expect(mocks.guardarVista).toHaveBeenCalledWith("Vencidos de octubre", expect.objectContaining({ fuente: "contratistas" })));
    await waitFor(() => expect((screen.getByLabelText("Vista del lienzo") as HTMLSelectElement).value).toBe("guardada:v1"));
    expect(screen.getByRole("group", { name: "Mis vistas guardadas" })).toBeTruthy();
  });

  it("una vista guardada se abre sin llamar al modelo, y se puede borrar confirmando", async () => {
    mocks.listarVistasGuardadas.mockResolvedValue([
      { id: "v1", nombre: "Vencidos de octubre", lienzo: LIENZO_DEL_ASISTENTE, actualizado_en: "2026-10-23T10:00:00Z" },
    ]);
    montar();
    await screen.findByRole("option", { name: "Vencidos de octubre" });
    fireEvent.change(screen.getByLabelText("Vista del lienzo"), { target: { value: "guardada:v1" } });
    await waitFor(() => expect(mocks.consultarLienzo).toHaveBeenCalledWith(LIENZO_DEL_ASISTENTE));
    expect(mocks.preguntarAlAsistente).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Borrar vista guardada" }));
    expect(mocks.borrarVista).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: /¿Borrar esta vista\?/ }));
    await waitFor(() => expect(mocks.borrarVista).toHaveBeenCalledWith("v1"));
  });

  it("una vista guardada que ya no pasa el validador avisa en vez de dibujarse", async () => {
    mocks.listarVistasGuardadas.mockResolvedValue([
      { id: "v2", nombre: "Vieja", lienzo: { fuente: "usuarios", filtros: [], widgets: [] }, actualizado_en: "2026-10-01T10:00:00Z" },
    ]);
    montar();
    await screen.findByRole("option", { name: "Vieja" });
    fireEvent.change(screen.getByLabelText("Vista del lienzo"), { target: { value: "guardada:v2" } });
    expect((await screen.findByRole("alert")).textContent).toContain("Vista inválida");
    expect(mocks.consultarLienzo).not.toHaveBeenCalled();
  });

  it("Exportar baja los datos de la vista con sus columnas y filtros", async () => {
    await abrirVencidos();
    fireEvent.click(screen.getByText("Exportar"));
    fireEvent.click(screen.getByRole("menuitem", { name: /Excel/ }));
    await waitFor(() => expect(mocks.exportarFilas).toHaveBeenCalledTimes(1));
    const [formato, opciones] = mocks.exportarFilas.mock.calls[0];
    expect(formato).toBe("excel");
    expect(opciones.filas).toEqual(vencidos);
    expect(opciones.origen).toBe("contratistas");
    expect(opciones.filtros).toBe("Estado PRAIND: VENCIDA");
    expect(opciones.generadoPor).toBe("admin@example.com");
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
