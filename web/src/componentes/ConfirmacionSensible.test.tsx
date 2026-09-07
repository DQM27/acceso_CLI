import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ConfirmacionSensible from "./ConfirmacionSensible";

// Se mockea el hook entero en vez de `../lib/supabase` -- lo que se prueba
// acá es el manejo de estado de `ConfirmacionSensible` (el bug real:
// quedaba atascado en "Enviando código…" cuando `onConfirmar` fallaba),
// no la lógica de `useVerificacionPorCorreo`, que ya vive aparte.
const confirmacion = {
  enviado: true,
  enviando: false,
  error: null as string | null,
  pedirConfirmacion: vi.fn(),
  confirmarCodigo: vi.fn(),
  reiniciar: vi.fn(),
};
vi.mock("./useVerificacionPorCorreo", () => ({
  useVerificacionPorCorreo: () => confirmacion,
}));

beforeEach(() => {
  vi.clearAllMocks();
  confirmacion.enviado = true;
  confirmacion.error = null;
  confirmacion.confirmarCodigo.mockResolvedValue(undefined);
});
afterEach(() => {
  vi.restoreAllMocks();
});

function renderizar(props: Partial<Parameters<typeof ConfirmacionSensible>[0]> = {}) {
  const onConfirmar = props.onConfirmar ?? vi.fn().mockResolvedValue(undefined);
  const onCerrar = props.onCerrar ?? vi.fn();
  const resultado = render(
    <ConfirmacionSensible
      abierto
      correo="admin@example.com"
      titulo="Confirmar"
      pregunta="¿Seguro?"
      descripcion="confirmar la acción"
      onConfirmar={onConfirmar}
      onCerrar={onCerrar}
      {...props}
    />,
  );
  // Salta directo al paso "código" -- el paso "pregunta" ya está cubierto
  // por el resto de los tests (todos pasan por acá con un click).
  fireEvent.click(screen.getByText("Sí, enviar código"));
  return { ...resultado, onConfirmar, onCerrar };
}

async function enviarCodigo() {
  fireEvent.change(screen.getByLabelText("Código de confirmación"), { target: { value: "123456" } });
  fireEvent.click(screen.getByRole("button", { name: /Confirmar|Reintentar/ }));
  // Deja correr los `await` internos de `alConfirmarCodigo`.
  await screen.findByRole("button", { name: /Confirmar|Reintentar|Cancelar/ });
}

describe("ConfirmacionSensible", () => {
  it("no queda atascado en 'Enviando código…' cuando onConfirmar falla sin relanzar -- el patrón real de hoy (toast + no cierra)", async () => {
    // Reproduce exactamente lo que hacen alConfirmarAlta/alConfirmarBaja/etc.:
    // atrapan su propio error, hacen toast, y NO relanzan ni cierran el
    // modal (no bajan `abierto`) -- desde la perspectiva de
    // ConfirmacionSensible, onConfirmar "resuelve bien".
    const onConfirmar = vi.fn().mockResolvedValue(undefined);
    renderizar({ onConfirmar });
    await enviarCodigo();

    expect(onConfirmar).toHaveBeenCalledTimes(1);
    // El bug real: sin el fix, acá sólo quedaba el texto "Enviando
    // código…" sin ningún botón. Con el fix, el formulario del código
    // sigue ahí, usable.
    expect(screen.getByLabelText("Código de confirmación")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Confirmar" })).not.toBeNull();
    expect(screen.queryByText("Enviando código…")).toBeNull();
  });

  it("muestra el error y ofrece 'Reintentar' cuando onConfirmar relanza", async () => {
    const onConfirmar = vi.fn().mockRejectedValue(new Error("el servidor no respondió"));
    renderizar({ onConfirmar });
    await enviarCodigo();

    expect(screen.getByRole("alert").textContent).toBe("el servidor no respondió");
    expect(screen.getByRole("button", { name: "Reintentar" })).not.toBeNull();
  });

  it("cierra y limpia el estado cuando onConfirmar termina bien y quien llama baja `abierto`", async () => {
    const onConfirmar = vi.fn().mockResolvedValue(undefined);
    const onCerrar = vi.fn();
    const { rerender } = renderizar({ onConfirmar, onCerrar });
    await enviarCodigo();

    // Quien llama es responsable de bajar `abierto` tras el éxito (ver
    // Administradores.tsx: cerrarModal()/setBajaEnCurso(null)).
    rerender(
      <ConfirmacionSensible
        abierto={false}
        correo="admin@example.com"
        titulo="Confirmar"
        pregunta="¿Seguro?"
        descripcion="confirmar la acción"
        onConfirmar={onConfirmar}
        onCerrar={onCerrar}
      />,
    );
    expect(confirmacion.reiniciar).toHaveBeenCalled();
  });

  it("Cancelar en el paso de código llama a onCerrar", () => {
    const { onCerrar } = renderizar();
    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(onCerrar).toHaveBeenCalledTimes(1);
  });
});
