import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AuthProvider, useAuth } from "../contexto/AuthContexto";
import type { Resultado } from "../contexto/AuthContexto";

const dobles = vi.hoisted(() => ({
  getSession: vi.fn(),
  getUser: vi.fn(),
  signInWithPassword: vi.fn(),
  signInWithOtp: vi.fn(),
  verifyOtp: vi.fn(),
  updateUser: vi.fn(),
  signOut: vi.fn(),
  onAuthStateChange: vi.fn(),
  from: vi.fn(),
}));
vi.mock("../lib/supabase", () => ({
  CLAVE_SESION: "prueba",
  supabase: { auth: dobles, from: dobles.from },
}));
const sesion = (id = "A") => ({
  access_token: `token-${id}`,
  user: { id, email: `${id}@example.invalid` },
});
let evento: (evento: string, sesion: unknown) => void;
let resolverConsulta: ReturnType<typeof vi.fn>;
function Vista() {
  const estado = useAuth();
  const [resultado, setResultado] = useState("");
  const mostrar = (r: Resultado) => setResultado(r.ok ? "ok" : r.mensaje);
  return (
    <>
      <span data-testid="cuenta">
        {estado.anfitrion?.correo ?? "sin sesión"}
      </span>
      <span data-testid="verificado">{String(estado.verificado)}</span>
      <span data-testid="cargando">{String(estado.cargando)}</span>
      {estado.error && <div role="alert">{estado.error}</div>}
      <button onClick={estado.verificar}>Verificar</button>
      <span data-testid="resultado">{resultado}</span>
      <span data-testid="definir">{String(estado.debeDefinirContrasena)}</span>
      <button
        onClick={() =>
          void estado.iniciarSesion("  Ana@Example.Invalid ", "frase secreta larga").then(mostrar)
        }
      >
        Entrar
      </button>
      <button onClick={() => void estado.solicitarCodigo("Ana@Example.Invalid").then(mostrar)}>
        Pedir código
      </button>
      <button onClick={() => void estado.verificarCodigo("Ana@Example.Invalid", " 123 456 ").then(mostrar)}>
        Verificar código
      </button>
      <button onClick={() => void estado.definirContrasena("frase secreta larga").then(mostrar)}>
        Definir
      </button>
    </>
  );
}
beforeEach(() => {
  sessionStorage.clear();
  vi.resetAllMocks();
  dobles.getSession.mockResolvedValue({
    data: { session: sesion() },
    error: null,
  });
  dobles.getUser.mockImplementation(async (token: string) => ({
    data: { user: sesion(token.slice(-1)).user },
    error: null,
  }));
  dobles.onAuthStateChange.mockImplementation((callback: typeof evento) => {
    evento = callback;
    return { data: { subscription: { unsubscribe: vi.fn() } } };
  });
  resolverConsulta = vi
    .fn()
    .mockResolvedValue({
      data: { correo: "A@example.invalid", nombre: "Anfitrión A" },
      error: null,
    });
  const consulta = {
    select: vi.fn(),
    eq: vi.fn(),
    maybeSingle: resolverConsulta,
  };
  consulta.select.mockReturnValue(consulta);
  consulta.eq.mockReturnValue(consulta);
  dobles.from.mockReturnValue(consulta);
  dobles.signOut.mockImplementation(async () => {
    evento("SIGNED_OUT", null);
    return { error: null };
  });
});
function montar() {
  return render(
    <AuthProvider>
      <Vista />
    </AuthProvider>,
  );
}
describe("autorización de anfitriones", () => {
  it("autoriza exclusivamente con una fila validada del servidor", async () => {
    montar();
    await waitFor(() =>
      expect(screen.getByTestId("cuenta").textContent).toBe(
        "A@example.invalid",
      ),
    );
    expect(screen.getByTestId("verificado").textContent).toBe("true");
    expect(dobles.getUser).toHaveBeenCalledWith("token-A");
  });
  it("deniega y cierra sesión si no existe el anfitrión", async () => {
    resolverConsulta.mockResolvedValue({ data: null, error: null });
    montar();
    await screen.findByRole("alert");
    expect(screen.getByTestId("cuenta").textContent).toBe("sin sesión");
    expect(dobles.signOut).toHaveBeenCalledWith({ scope: "local" });
    expect(screen.getByRole("alert").textContent).toContain(
      "no está autorizada",
    );
  });
  it("preserva la misma pantalla ante un error de red y bloquea escrituras", async () => {
    montar();
    await waitFor(() =>
      expect(screen.getByTestId("verificado").textContent).toBe("true"),
    );
    resolverConsulta.mockResolvedValue({
      data: null,
      error: { code: "timeout" },
    });
    fireEvent.click(screen.getByText("Verificar"));
    await screen.findByRole("alert");
    expect(screen.getByTestId("cuenta").textContent).toBe("A@example.invalid");
    expect(screen.getByTestId("verificado").textContent).toBe("false");
    expect(dobles.signOut).not.toHaveBeenCalled();
  });
  it("no restaura una cuenta si la consulta responde después del cierre", async () => {
    let resolver!: (valor: unknown) => void;
    resolverConsulta.mockReturnValue(
      new Promise((fin) => {
        resolver = fin;
      }),
    );
    montar();
    await waitFor(() => expect(resolverConsulta).toHaveBeenCalled());
    act(() => evento("SIGNED_OUT", null));
    await act(async () =>
      resolver({
        data: { correo: "A@example.invalid", nombre: "Cuenta anterior" },
        error: null,
      }),
    );
    expect(screen.getByTestId("cuenta").textContent).toBe("sin sesión");
    expect(screen.getByTestId("verificado").textContent).toBe("false");
  });
  it("borra la identidad anterior aunque la comprobación de la nueva falle", async () => {
    montar();
    await waitFor(() =>
      expect(screen.getByTestId("verificado").textContent).toBe("true"),
    );
    resolverConsulta.mockResolvedValue({
      data: null,
      error: { code: "timeout" },
    });
    act(() => evento("SIGNED_IN", sesion("B")));
    await screen.findByRole("alert");
    expect(screen.getByTestId("cuenta").textContent).toBe("sin sesión");
  });
  it("no queda cargando si falla recuperar la sesión", async () => {
    dobles.getSession.mockRejectedValue(new Error("almacenamiento bloqueado"));
    montar();
    await screen.findByRole("alert");
    expect(screen.getByTestId("cargando").textContent).toBe("false");
  });
});

/** Error con la forma de AuthApiError de Supabase. */
const errorAuth = (status: number, code?: string) =>
  Object.assign(new Error(code ?? "fallo"), { status, code });

async function pulsar(boton: string) {
  fireEvent.click(screen.getByText(boton));
  await waitFor(() =>
    expect(screen.getByTestId("resultado").textContent).not.toBe(""),
  );
  return screen.getByTestId("resultado").textContent;
}

describe("ingreso con correo y contraseña", () => {
  it("normaliza el correo antes de enviarlo", async () => {
    dobles.signInWithPassword.mockResolvedValue({ data: {}, error: null });
    montar();
    expect(await pulsar("Entrar")).toBe("ok");
    expect(dobles.signInWithPassword).toHaveBeenCalledWith({
      email: "ana@example.invalid",
      password: "frase secreta larga",
    });
  });
  it.each([
    ["credenciales incorrectas", errorAuth(400, "invalid_credentials")],
    ["correo sin confirmar", errorAuth(400, "email_not_confirmed")],
    ["usuario bloqueado", errorAuth(403, "user_banned")],
  ])("no revela el estado de la cuenta: %s", async (_caso, fallo) => {
    dobles.signInWithPassword.mockResolvedValue({ data: {}, error: fallo });
    montar();
    expect(await pulsar("Entrar")).toBe("Correo o contraseña incorrectos.");
  });
  it("avisa el límite de intentos del servidor", async () => {
    dobles.signInWithPassword.mockResolvedValue({
      data: {},
      error: errorAuth(429, "over_request_rate_limit"),
    });
    montar();
    expect(await pulsar("Entrar")).toContain("Demasiados intentos");
  });
  it("distingue un problema de conexión", async () => {
    dobles.signInWithPassword.mockRejectedValue(new TypeError("Failed to fetch"));
    montar();
    expect(await pulsar("Entrar")).toContain("conexión");
  });
});

describe("código de correo (primer ingreso y recuperación)", () => {
  it("pide el código sin crear cuentas", async () => {
    dobles.signInWithOtp.mockResolvedValue({ data: {}, error: null });
    montar();
    expect(await pulsar("Pedir código")).toBe("ok");
    expect(dobles.signInWithOtp).toHaveBeenCalledWith({
      email: "ana@example.invalid",
      options: { shouldCreateUser: false },
    });
  });
  it("contesta igual si el correo no tiene cuenta", async () => {
    dobles.signInWithOtp.mockResolvedValue({
      data: {},
      error: errorAuth(422, "otp_disabled"),
    });
    montar();
    expect(await pulsar("Pedir código")).toBe("ok");
  });
  it("avisa el límite de envíos de correo", async () => {
    dobles.signInWithOtp.mockResolvedValue({
      data: {},
      error: errorAuth(429, "over_email_send_rate_limit"),
    });
    montar();
    expect(await pulsar("Pedir código")).toContain("Demasiados intentos");
  });
  it("con un código válido exige definir la contraseña", async () => {
    dobles.verifyOtp.mockResolvedValue({ data: {}, error: null });
    montar();
    expect(await pulsar("Verificar código")).toBe("ok");
    expect(dobles.verifyOtp).toHaveBeenCalledWith({
      email: "ana@example.invalid",
      token: "123456",
      type: "email",
    });
    expect(screen.getByTestId("definir").textContent).toBe("true");
  });
  it("rechaza un código vencido sin marcar nada", async () => {
    dobles.verifyOtp.mockResolvedValue({
      data: {},
      error: errorAuth(403, "otp_expired"),
    });
    montar();
    expect(await pulsar("Verificar código")).toContain("vencido");
    expect(screen.getByTestId("definir").textContent).toBe("false");
  });
  it("al guardar la contraseña ya no la pide", async () => {
    dobles.verifyOtp.mockResolvedValue({ data: {}, error: null });
    dobles.updateUser.mockResolvedValue({ data: {}, error: null });
    montar();
    await pulsar("Verificar código");
    fireEvent.click(screen.getByText("Definir"));
    await waitFor(() =>
      expect(screen.getByTestId("definir").textContent).toBe("false"),
    );
  });
  it("cerrar la sesión olvida el paso pendiente", async () => {
    dobles.verifyOtp.mockResolvedValue({ data: {}, error: null });
    montar();
    await pulsar("Verificar código");
    act(() => evento("SIGNED_OUT", null));
    expect(screen.getByTestId("definir").textContent).toBe("false");
  });
  it("explica cuando el servidor rechaza la contraseña por débil", async () => {
    dobles.updateUser.mockResolvedValue({
      data: {},
      error: errorAuth(422, "weak_password"),
    });
    montar();
    expect(await pulsar("Definir")).toContain("filtraciones");
    expect(dobles.updateUser).toHaveBeenCalledWith({
      password: "frase secreta larga",
    });
  });
});
