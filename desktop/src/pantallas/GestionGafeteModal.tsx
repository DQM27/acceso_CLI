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
  marcarGafetePerdidoContratista,
  marcarGafetePerdidoVisita,
  nombrePortador,
  resolverGafete,
  verificarCheckInVisita,
} from "../api";
import type { ContratistaResumen, GafeteResumen, MotivoResolucionGafete } from "../api";

const DEBOUNCE_MS = 120;
const MAX_RESULTADOS = 4;

/**
 * Acciones de un gafete puntual, según su estado actual — mismo criterio
 * que la TUI (B/P/R en `src/tui/gafetes/`): Disponible ofrece dar de baja o
 * marcar perdido (búsqueda del portador, ramificada por `gafete.tipo`:
 * contratista busca por texto libre, mismo mecanismo de `NuevoIngresoModal`;
 * visita busca por cédula exacta, mismo mecanismo del check-in); Perdido
 * ofrece resolver (pagado/apareció, sólo tiene sentido para contratista,
 * pero el backend no lo restringe por tipo). De baja no ofrece ninguna
 * acción — es un estado final.
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
  const [buscandoPortador, setBuscandoPortador] = useState(false);
  const [filtro, setFiltro] = useState("");
  const [resultados, setResultados] = useState<ContratistaResumen[]>([]);
  const [cedulaVisita, setCedulaVisita] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const esVisita = gafete.tipo === "Visita";
  // Un gafete provisional KOF no se marca perdido/resuelto con este flujo
  // -- su ciclo es entrega/devolución (`prestamos_gafete_provisional`), no
  // "asignar un portador que debe responder por él". Mostrar el botón acá
  // lo haría caer por accidente en la búsqueda de contratista (rama
  // `!esVisita`), que no tiene sentido para este tipo -- se oculta hasta
  // que exista un flujo de entrega/devolución propio en desktop.
  const esProvisionalKof = gafete.tipo === "ProvisionalKof";
  // Mismo criterio que ProvisionalKof: el ciclo de un gafete de proveedor
  // es ingreso/salida (`registro_ingresos_proveedor`), no un portador que
  // se le asigna para marcarlo perdido -- se oculta hasta que exista un
  // flujo de ingreso/salida propio en desktop.
  const esProveedor = gafete.tipo === "Proveedor";
  const listaVisible = !esVisita && buscandoPortador && filtro.trim().length > 0;
  const { campoRef, posicion: posicionLista } = useListaFlotante(listaVisible);
  const { resaltado, setResaltado, manejarTecla } = useNavegacionFlechas(
    resultados,
    listaVisible,
    elegirPortadorContratista,
  );

  useEffect(() => {
    if (esVisita || !buscandoPortador || !filtro.trim()) {
      // `Promise.resolve().then(...)` en vez de llamar `setResultados([])`
      // directo -- ver el mismo comentario en Activos.tsx.
      Promise.resolve().then(() => setResultados([]));
      return;
    }
    const id = setTimeout(() => {
      buscarContratistas({ texto: filtro })
        .then((pagina) => setResultados(pagina.items.slice(0, MAX_RESULTADOS)))
        .catch((error) => setError(String(error)));
    }, DEBOUNCE_MS);
    return () => clearTimeout(id);
  }, [filtro, buscandoPortador, esVisita]);

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

  async function elegirPortadorContratista(contratista: ContratistaResumen) {
    setError(null);
    setEnviando(true);
    try {
      await marcarGafetePerdidoContratista(gafete.id, contratista.id);
      onCambiado();
    } catch (error) {
      setError(String(error));
      setEnviando(false);
    }
  }

  /** Sin búsqueda por texto libre para visitantes (a diferencia de
   * contratista) -- `cita_visitantes` sólo se resuelve por cédula exacta,
   * mismo mecanismo que ya usa el check-in (`verificarCheckInVisita`); no
   * hace falta un endpoint de búsqueda nuevo para este flujo puntual. */
  async function confirmarPortadorVisita() {
    setError(null);
    setEnviando(true);
    try {
      const { visitante } = await verificarCheckInVisita(cedulaVisita.trim());
      await marcarGafetePerdidoVisita(gafete.id, visitante.id);
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
          Tipo: <strong style={{ color: "var(--texto)" }}>{gafete.tipo}</strong>
          {" · "}
          Estado: <strong style={{ color: "var(--texto)" }}>{textoEstado(gafete.estado)}</strong>
        </p>
        {nombrePortador(gafete) && (
          <p style={{ margin: 0, color: "var(--muted)" }}>
            Asignado a: <strong style={{ color: "var(--texto)" }}>{nombrePortador(gafete)}</strong>
          </p>
        )}

        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        {gafete.estado === "Disponible" && !buscandoPortador && (
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button
              type="button"
              className="boton"
              disabled={enviando}
              onClick={confirmarBaja}
            >
              Dar de baja
            </button>
            {!esProvisionalKof && !esProveedor && (
              <button
                type="button"
                className="boton"
                disabled={enviando}
                onClick={() => setBuscandoPortador(true)}
              >
                Marcar perdido…
              </button>
            )}
          </div>
        )}

        {gafete.estado === "Disponible" && buscandoPortador && esVisita && (
          <form
            style={{ display: "flex", gap: "0.5rem", alignItems: "flex-end" }}
            onSubmit={(evento) => {
              evento.preventDefault();
              confirmarPortadorVisita();
            }}
          >
            <label className="campo" style={{ flex: 1 }}>
              Asignado a · cédula del visitante
              <input
                value={cedulaVisita}
                onChange={(evento) => setCedulaVisita(evento.target.value)}
                autoFocus
                placeholder="Cédula…"
              />
            </label>
            <button type="submit" className="boton" disabled={enviando || !cedulaVisita.trim()}>
              Confirmar
            </button>
          </form>
        )}

        {gafete.estado === "Disponible" && buscandoPortador && !esVisita && (
          <div ref={campoRef}>
            <label className="campo">
              Asignado a · cédula o nombre
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
                    onClick={() => elegirPortadorContratista(contratista)}
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

export function textoEstado(estado: GafeteResumen["estado"]): string {
  switch (estado) {
    case "Disponible":
      return "Disponible";
    case "Perdido":
      return "Perdido";
    case "DeBaja":
      return "De baja";
  }
}
