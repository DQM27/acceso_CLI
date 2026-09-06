import { useEffect, useState } from "react";
import Modal from "./Modal";
import { useVerificacionPorCorreo } from "./useVerificacionPorCorreo";

/**
 * Confirmación por código de correo para acciones sensibles (alta/baja de
 * administradores, y cualquier otra mutación que en el futuro necesite el
 * mismo "sos vos ahora mismo" -- ver `useVerificacionPorCorreo`). Absorbe
 * el modal + form + manejo de error que antes vivía duplicado (alta y baja)
 * en `Administradores.tsx`.
 *
 * El código se pide solo al montarse (abrir el modal), no en cada render --
 * quien lo usa controla el ciclo de vida con la prop `abierto`.
 */
export default function ConfirmacionSensible({
  abierto,
  correo,
  titulo,
  descripcion,
  onConfirmar,
  onCerrar,
}: {
  abierto: boolean;
  /** Correo de quien está haciendo la acción -- ahí llega el código. */
  correo: string;
  titulo: string;
  /** Texto que explica qué se va a confirmar (ej. "agregás a fulano@..."). */
  descripcion: string;
  /** Mutación real, ejecutada recién después de validar el código. Si tira,
   * el modal queda abierto y el error se muestra vía toast en quien llama. */
  onConfirmar: () => Promise<void>;
  onCerrar: () => void;
}) {
  const [codigo, setCodigo] = useState("");
  const [confirmando, setConfirmando] = useState(false);
  const confirmacion = useVerificacionPorCorreo(correo);

  useEffect(() => {
    if (abierto) confirmacion.pedirConfirmacion();
    // Sólo al abrir -- pedirConfirmacion/confirmacion cambian de identidad en
    // cada render y no deben re-disparar el envío del código.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [abierto]);

  function cerrar() {
    setCodigo("");
    confirmacion.reiniciar();
    onCerrar();
  }

  async function alConfirmar(evento: React.FormEvent) {
    evento.preventDefault();
    setConfirmando(true);
    try {
      await confirmacion.confirmarCodigo(codigo);
    } catch {
      setConfirmando(false);
      return; // el error ya quedó en confirmacion.error, mostrado inline
    }
    try {
      await onConfirmar();
      setCodigo("");
      confirmacion.reiniciar();
    } finally {
      setConfirmando(false);
    }
  }

  if (!abierto) return null;

  return (
    <Modal titulo={titulo} onCerrar={cerrar}>
      {confirmacion.enviado ? (
        <form onSubmit={alConfirmar} style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          <p style={{ margin: 0 }}>
            Te mandamos un código a tu correo. Escribilo acá para {descripcion}.
          </p>
          <label className="campo">
            Código de confirmación
            <input
              inputMode="numeric"
              autoComplete="one-time-code"
              required
              autoFocus
              value={codigo}
              disabled={confirmando}
              placeholder="123456"
              onChange={(evento) => setCodigo(evento.target.value)}
            />
          </label>

          {confirmacion.error && (
            <p className="login-error" role="alert">
              {confirmacion.error}
            </p>
          )}

          <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
            <button type="button" className="boton" disabled={confirmando} onClick={cerrar}>
              Cancelar
            </button>
            <button type="submit" className="boton boton-primario" disabled={confirmando}>
              {confirmando ? "Confirmando…" : "Confirmar"}
            </button>
          </div>
        </form>
      ) : confirmacion.error ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          <p className="login-error" role="alert">
            {confirmacion.error}
          </p>
          <div style={{ display: "flex", justifyContent: "flex-end" }}>
            <button type="button" className="boton" onClick={cerrar}>
              Cerrar
            </button>
          </div>
        </div>
      ) : (
        <p style={{ margin: 0, color: "var(--muted)" }}>Enviando código…</p>
      )}
    </Modal>
  );
}
