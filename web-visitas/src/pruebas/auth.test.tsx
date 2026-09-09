import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AuthProvider, useAuth } from "../contexto/AuthContexto";

const dobles = vi.hoisted(() => ({
  getSession: vi.fn(),
  getUser: vi.fn(),
  signInWithOAuth: vi.fn(),
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
  return (
    <>
      <span data-testid="cuenta">
        {estado.anfitrion?.correo ?? "sin sesión"}
      </span>
      <span data-testid="verificado">{String(estado.verificado)}</span>
      <span data-testid="cargando">{String(estado.cargando)}</span>
      {estado.error && <div role="alert">{estado.error}</div>}
      <button onClick={estado.verificar}>Verificar</button>
      <button onClick={() => void estado.iniciarSesion()}>Entrar</button>
    </>
  );
}
beforeEach(() => {
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
  it("maneja errores de OAuth y usa un retorno de su propio origen", async () => {
    dobles.signInWithOAuth.mockResolvedValue({
      error: new Error("fallo"),
      data: null,
    });
    montar();
    await waitFor(() =>
      expect(screen.getByTestId("verificado").textContent).toBe("true"),
    );
    fireEvent.click(screen.getByText("Entrar"));
    await screen.findByRole("alert");
    expect(dobles.signInWithOAuth).toHaveBeenCalledWith(
      expect.objectContaining({
        provider: "google",
        options: expect.objectContaining({
          redirectTo: `${window.location.origin}/auth/callback`,
        }),
      }),
    );
  });
});
