import { useEffect, useState } from "react";
import Modal from "./Modal";
import { useVerificacionPorCorreo } from "./useVerificacionPorCorreo";
import { mensajeError } from "../mensajeError";

/**
 * Confirmación por código de correo para acciones sensibles (alta/baja de
 * administradores, Revocar/Eliminar dispositivos, y cualquier otra mutación
 * que en el futuro necesite el mismo "sos vos ahora mismo" -- ver
 * `useVerificacionPorCorreo`). Absorbe el modal + form + manejo de error
 * que antes vivía duplicado en cada pantalla.
 *
 * Dos pasos, a propósito -- "pregunta" primero (sin pedir código todavía) y
 * recién si se confirma ahí se pide el código ("codigo"). Antes el código
 * se mandaba apenas se abría el modal: un click de más (o probar dos veces
 * seguidas) quemaba un envío real contra el límite de reenvío de Supabase
 * (~1 cada 60s por correo, ver `useVerificacionPorCorreo`). Es un paso más,
 * pero evita gastar códigos en aperturas que no van a terminar en nada.
 */
export default function ConfirmacionSensible({
  abierto,
  correo,
  titulo,
  pregunta,
  descripcion,
  onConfirmar,
  onCerrar,
}: {
  abierto: boolean;
  /** Correo de quien está haciendo la acción -- ahí llega el código. */
  correo: string;
  titulo: string;
  /** Pregunta del primer paso, antes de mandar ningún código (ej. "¿Revocar
   * 'Brisas - PC'? Va a dejar de poder sincronizar."). */
  pregunta: string;
  /** Texto del segundo paso, ya con el código pedido (ej. "agregás a
   * fulano@..."). */
  descripcion: string;
  /** Mutación real, ejecutada recién después de validar el código. Si tira,
   * el modal queda abierto y el error se muestra vía toast en quien llama. */
  onConfirmar: () => Promise<void>;
  onCerrar: () => void;
}) {
  const [paso, setPaso] = useState<"pregunta" | "codigo">("pregunta");
  const [codigo, setCodigo] = useState("");
  const [confirmando, setConfirmando] = useState(false);
  // Error de `onConfirmar` en sí (la mutación real), separado de
  // `confirmacion.error` (error de validar el código) -- ver el catch de
  // `alConfirmarCodigo` más abajo.
  const [errorAccion, setErrorAccion] = useState<string | null>(null);
  const confirmacion = useVerificacionPorCorreo(correo);

  // Reinicia todo cada vez que el modal se cierra -- sea porque alguien
  // canceló, o porque `onConfirmar` terminó bien y quien llama bajó
  // `abierto` (cierra el modal desde afuera, ver el doc-comment de esa
  // prop). Deliberadamente NO se reinicia nada dentro de
  // `alConfirmarCodigo`: si `onConfirmar` falla y quien llama no baja
  // `abierto` (el patrón real hoy: atrapa su propio error y sólo hace
  // toast, sin cerrar el modal), este efecto no corre y el paso "código"
  // se queda tal cual estaba -- con botones para reintentar o cancelar --
  // en vez de caer en un estado sin ninguno.
  useEffect(() => {
    // Nota: NO se difiere con `Promise.resolve().then(...)` (el patrón que
    // sí usan los demás efectos de este repo para esta misma regla) porque
    // acá el efecto no sólo actualiza estado propio -- también llama
    // `confirmacion.reiniciar()`, una función externa cuyo llamador espera
    // que corra en el mismo tick que el cambio de `abierto` (ver
    // ConfirmacionSensible.test.tsx, "cierra y limpia el estado...").
    // Diferirla rompe ese contrato observable sin arreglar nada real.
    if (abierto) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setPaso("pregunta");
    } else {
      setCodigo("");
      setErrorAccion(null);
      confirmacion.reiniciar();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [abierto]);

  function cerrar() {
    onCerrar();
  }

  function alConfirmarPregunta() {
    setPaso("codigo");
    confirmacion.pedirConfirmacion();
  }

  async function alConfirmarCodigo(evento: React.FormEvent) {
    evento.preventDefault();
    setConfirmando(true);
    setErrorAccion(null);
    try {
      await confirmacion.confirmarCodigo(codigo);
    } catch {
      setConfirmando(false);
      return; // el error ya quedó en confirmacion.error, mostrado inline
    }
    try {
      await onConfirmar();
      // Éxito: la responsabilidad de cerrar (bajar `abierto`) es de quien
      // llama -- el efecto de arriba limpia el resto cuando eso pasa.
    } catch (error) {
      // Hoy ningún llamador relanza (atrapan su propio error y sólo
      // hacen toast) -- este catch cubre a un futuro llamador que sí lo
      // haga, para no dejar una promesa rechazada sin manejar ni un modal
      // sin ninguna pista de qué pasó.
      setErrorAccion(mensajeError(error));
    } finally {
      setConfirmando(false);
    }
  }

  if (!abierto) return null;

  return (
    <Modal titulo={titulo} onCerrar={cerrar}>
      {paso === "pregunta" ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          <p style={{ margin: 0 }}>{pregunta}</p>
          <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
            <button type="button" className="boton" onClick={cerrar}>
              Cancelar
            </button>
            <button type="button" className="boton boton-primario" onClick={alConfirmarPregunta}>
              Sí, enviar código
            </button>
          </div>
        </div>
      ) : confirmacion.enviado ? (
        <form onSubmit={alConfirmarCodigo} style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
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

          {(confirmacion.error || errorAccion) && (
            <p className="login-error" role="alert">
              {confirmacion.error ?? errorAccion}
            </p>
          )}

          <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
            <button type="button" className="boton" disabled={confirmando} onClick={cerrar}>
              Cancelar
            </button>
            <button type="submit" className="boton boton-primario" disabled={confirmando}>
              {confirmando ? "Confirmando…" : errorAccion ? "Reintentar" : "Confirmar"}
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
