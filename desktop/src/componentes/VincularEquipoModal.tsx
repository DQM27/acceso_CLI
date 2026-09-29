import { useEffect, useState } from "react";
import { toast } from "sonner";
import Modal from "./Modal";
import { estadoVinculacion, revincularDispositivo } from "../api";
import type { EstadoVinculacion } from "../api";
import { codigoCompleto, formatearCodigo } from "./CodigoVinculacion.logica";

const DESCRIPCION_CREDENCIAL: Record<EstadoVinculacion["credencial"], string> = {
  clave: "Vinculado con su propia clave.",
  secreto_legado: "Usa el secreto de antes; se pasa solo a clave en la próxima sincronización.",
  sin_vincular: "Todavía no está vinculado a la nube.",
};

/**
 * Re-vincular este equipo con un código nuevo del panel ("Re-vincular" en
 * la pantalla de Dispositivos), sin vaciar la base local: lo pendiente de
 * enviar se conserva. Para un equipo reinstalado, con la clave perdida, o
 * que el panel dejó fuera al vincular otro equipo. Sólo ROOT (lo vuelve a
 * verificar `comandos::nube::revincular_dispositivo`).
 */
export default function VincularEquipoModal({ onCerrar }: { onCerrar: () => void }) {
  const [estado, setEstado] = useState<EstadoVinculacion | null>(null);
  const [codigo, setCodigo] = useState("");
  const [enviando, setEnviando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    estadoVinculacion()
      .then(setEstado)
      .catch((errorEstado) => setError(String(errorEstado)));
  }, []);

  async function alEnviar(evento: React.FormEvent) {
    evento.preventDefault();
    if (!codigoCompleto(codigo)) return;
    setEnviando(true);
    setError(null);
    try {
      await revincularDispositivo(codigo);
      toast.success("Equipo vinculado.");
      onCerrar();
    } catch (errorVincular) {
      setError(String(errorVincular));
    } finally {
      setEnviando(false);
    }
  }

  return (
    <Modal titulo="Vincular este equipo" onCerrar={onCerrar}>
      <form onSubmit={alEnviar} style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}>
        {estado && (
          <p style={{ margin: 0, color: "var(--muted)", fontSize: "0.9rem" }}>
            {DESCRIPCION_CREDENCIAL[estado.credencial]}
          </p>
        )}
        <label className="campo">
          Código de vinculación
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
          {enviando ? "Vinculando…" : "Vincular"}
        </button>
      </form>
    </Modal>
  );
}
