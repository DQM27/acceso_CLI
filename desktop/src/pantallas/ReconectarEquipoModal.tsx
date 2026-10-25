import { useState } from "react";
import Modal from "../componentes/Modal";
import { codigoCompleto, formatearCodigo } from "../componentes/CodigoVinculacion.logica";
import { reconectarEquipo, reiniciarApp } from "../api";
import type { ResultadoReconexion } from "../api";

type Paso =
  | { tipo: "codigo" }
  | { tipo: "confirmar"; pendientes: number; unidad: string }
  | { tipo: "otraUnidad"; unidad: string };

/**
 * Reconexión de un equipo retirado en el panel (ver
 * docs/features-futuras/plan-reconexion-equipo-retirado.md). Sólo se ofrece
 * cuando el núcleo dice que el equipo está retirado. Todo lo decide el
 * núcleo (`reconectar_equipo`): esta ventana pide el código, muestra el aviso
 * de pérdida si el núcleo lo pide y reinicia la app si el equipo cambió de
 * unidad.
 */
export default function ReconectarEquipoModal({
  onReconectado,
  onCerrar,
}: {
  /** Misma unidad: quien llama sincroniza para que suba lo pendiente. */
  onReconectado: (unidad: string) => void;
  onCerrar: () => void;
}) {
  const [codigo, setCodigo] = useState("");
  const [paso, setPaso] = useState<Paso>({ tipo: "codigo" });
  const [enviando, setEnviando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function reconectar(pendientesConfirmados?: number) {
    setEnviando(true);
    setError(null);
    try {
      const resultado: ResultadoReconexion = await reconectarEquipo(codigo, pendientesConfirmados);
      if (resultado.tipo === "Reconectado") {
        onReconectado(resultado.unidad);
      } else if (resultado.tipo === "NecesitaConfirmacion") {
        setPaso({ tipo: "confirmar", pendientes: resultado.pendientes, unidad: resultado.unidad });
      } else {
        setPaso({ tipo: "otraUnidad", unidad: resultado.unidad });
      }
    } catch (causa) {
      setError(String(causa));
    } finally {
      setEnviando(false);
    }
  }

  // Ya reconectado en otra unidad: la base de la anterior no sirve. No se
  // puede cerrar sin reiniciar.
  if (paso.tipo === "otraUnidad") {
    return (
      <Modal titulo="Equipo reconectado" onCerrar={() => void reiniciarApp()}>
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          <p style={{ margin: 0 }}>
            Este equipo quedó en <strong>{paso.unidad}</strong>. La app se va a reiniciar para borrar
            los datos de la unidad anterior y bajar los de la nueva.
          </p>
          <button type="button" className="boton boton-primario" onClick={() => void reiniciarApp()}>
            Reiniciar ahora
          </button>
        </div>
      </Modal>
    );
  }

  if (paso.tipo === "confirmar") {
    return (
      <Modal titulo="Cambiar de unidad" onCerrar={onCerrar}>
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          <p style={{ margin: 0 }}>
            El código pasa este equipo a <strong>{paso.unidad}</strong>. Hay{" "}
            <strong>
              {paso.pendientes} {paso.pendientes === 1 ? "registro" : "registros"} sin subir
            </strong>{" "}
            a la nube que se van a perder: los datos de la unidad anterior se borran de este equipo.
          </p>
          {error && (
            <p className="login-error" role="alert" style={{ margin: 0 }}>
              {error}
            </p>
          )}
          <div style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end" }}>
            <button type="button" className="boton" onClick={onCerrar} disabled={enviando}>
              Cancelar
            </button>
            <button
              type="button"
              className="boton boton-peligro-solido"
              onClick={() => void reconectar(paso.pendientes)}
              disabled={enviando}
            >
              {enviando ? "Reconectando…" : "Reconectar y borrar"}
            </button>
          </div>
        </div>
      </Modal>
    );
  }

  return (
    <Modal titulo="Reconectar este equipo" onCerrar={onCerrar}>
      <form
        onSubmit={(evento) => {
          evento.preventDefault();
          if (codigoCompleto(codigo)) void reconectar();
        }}
        style={{ display: "flex", flexDirection: "column", gap: "1rem" }}
      >
        <p style={{ margin: 0, color: "var(--muted)", fontSize: "0.9rem" }}>
          Escriba el código de reconexión que le dio un administrador desde el panel. Sirve una sola
          vez y vence en pocos minutos.
        </p>
        <label className="campo">
          Código de reconexión
          <input
            type="text"
            autoFocus
            value={codigo}
            onChange={(evento) => setCodigo(formatearCodigo(evento.target.value))}
            placeholder="XXXX-XXXX-XX"
            autoComplete="off"
            spellCheck={false}
            disabled={enviando}
            style={{ fontFamily: "var(--fuente-mono, monospace)", letterSpacing: "0.12em", textAlign: "center" }}
          />
        </label>
        {error && (
          <p className="login-error" role="alert" style={{ margin: 0 }}>
            {error}
          </p>
        )}
        <button type="submit" className="boton boton-primario" disabled={enviando || !codigoCompleto(codigo)}>
          {enviando ? "Reconectando…" : "Reconectar"}
        </button>
      </form>
    </Modal>
  );
}
