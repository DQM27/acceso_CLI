import { useEffect, useState } from "react";
import Modal from "../componentes/Modal";
import {
  FilaListaFlotante,
  ListaFlotante,
  SinResultados,
  useListaFlotante,
  useNavegacionFlechas,
} from "../componentes/ListaFlotante";
import {
  buscarContratistas,
  darDeBajaGafete,
  marcarGafetePerdido,
  resolverGafete,
} from "../api";
import type { ContratistaResumen, GafeteResumen, MotivoResolucionGafete } from "../api";

const DEBOUNCE_MS = 120;
const MAX_RESULTADOS = 4;

/**
 * Acciones de un gafete puntual, según su estado actual — mismo criterio
 * que la TUI (B/P/R en `src/tui/gafetes/`): Disponible ofrece dar de baja o
 * marcar perdido (con búsqueda de contratista deudor, mismo mecanismo de
 * `NuevoIngresoModal`); Perdido ofrece resolver la deuda (pagado/apareció).
 * De baja no ofrece ninguna acción — es un estado final.
 */
export default function GestionGafeteModal({
  gafete,
  onCambiado,
  onCerrar,
}: {
  gafete: GafeteResumen;
  onCambiado: () => void;
  onCerrar: () => void;
}) {
  const [buscandoDeudor, setBuscandoDeudor] = useState(false);
  const [filtro, setFiltro] = useState("");
  const [resultados, setResultados] = useState<ContratistaResumen[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const listaVisible = buscandoDeudor && filtro.trim().length > 0;
  const { campoRef, posicion: posicionLista } = useListaFlotante(listaVisible);
  const { resaltado, setResaltado, manejarTecla } = useNavegacionFlechas(
    resultados,
    listaVisible,
    elegirDeudor,
  );

  useEffect(() => {
    if (!buscandoDeudor || !filtro.trim()) {
      setResultados([]);
      return;
    }
    const id = setTimeout(() => {
      buscarContratistas({ texto: filtro })
        .then((pagina) => setResultados(pagina.items.slice(0, MAX_RESULTADOS)))
        .catch((error) => setError(String(error)));
    }, DEBOUNCE_MS);
    return () => clearTimeout(id);
  }, [filtro, buscandoDeudor]);

  async function confirmarBaja() {
    setError(null);
    setEnviando(true);
    try {
      await darDeBajaGafete(gafete.id);
      onCambiado();
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  async function elegirDeudor(contratista: ContratistaResumen) {
    setError(null);
    setEnviando(true);
    try {
      await marcarGafetePerdido(gafete.id, contratista.id);
      onCambiado();
    } catch (error) {
      setError(String(error));
      setEnviando(false);
    }
  }

  async function confirmarResolver(motivo: MotivoResolucionGafete) {
    setError(null);
    setEnviando(true);
    try {
      await resolverGafete(gafete.id, motivo);
      onCambiado();
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  return (
    <Modal titulo={`Gafete ${String(gafete.numero).padStart(2, "0")}`} onCerrar={onCerrar}>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}>
        <p style={{ margin: 0, color: "var(--muted)" }}>
          Estado: <strong style={{ color: "var(--texto)" }}>{textoEstado(gafete.estado)}</strong>
        </p>
        {gafete.contratista_deudor_nombre && (
          <p style={{ margin: 0, color: "var(--muted)" }}>
            Deudor: <strong style={{ color: "var(--texto)" }}>{gafete.contratista_deudor_nombre}</strong>
          </p>
        )}

        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        {gafete.estado === "Disponible" && !buscandoDeudor && (
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button
              type="button"
              className="boton"
              disabled={enviando}
              onClick={confirmarBaja}
            >
              Dar de baja
            </button>
            <button
              type="button"
              className="boton"
              disabled={enviando}
              onClick={() => setBuscandoDeudor(true)}
            >
              Marcar perdido…
            </button>
          </div>
        )}

        {gafete.estado === "Disponible" && buscandoDeudor && (
          <div ref={campoRef}>
            <label className="campo">
              Deudor · cédula o nombre
              <input
                value={filtro}
                onChange={(evento) => setFiltro(evento.target.value)}
                onKeyDown={manejarTecla}
                autoFocus
                placeholder="Cédula o nombre…"
              />
            </label>
            {listaVisible && posicionLista && (
              <ListaFlotante posicion={posicionLista}>
                {resultados.length === 0 && <SinResultados />}
                {resultados.map((contratista, indice) => (
                  <FilaListaFlotante
                    key={contratista.id}
                    resaltada={indice === resaltado}
                    onClick={() => elegirDeudor(contratista)}
                    onMouseEnter={() => setResaltado(indice)}
                  >
                    <span>
                      {contratista.nombre}{" "}
                      <span style={{ color: "var(--muted)" }}>· {contratista.cedula}</span>
                    </span>
                  </FilaListaFlotante>
                ))}
              </ListaFlotante>
            )}
          </div>
        )}

        {gafete.estado === "Perdido" && (
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button
              type="button"
              className="boton"
              disabled={enviando}
              onClick={() => confirmarResolver("Pagado")}
            >
              Resolver · pagado
            </button>
            <button
              type="button"
              className="boton"
              disabled={enviando}
              onClick={() => confirmarResolver("Aparecido")}
            >
              Resolver · apareció
            </button>
          </div>
        )}

        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <button type="button" className="boton" onClick={onCerrar}>
            Cerrar
          </button>
        </div>
      </div>
    </Modal>
  );
}

function textoEstado(estado: GafeteResumen["estado"]): string {
  switch (estado) {
    case "Disponible":
      return "Disponible";
    case "Perdido":
      return "Perdido";
    case "DeBaja":
      return "De baja";
  }
}
