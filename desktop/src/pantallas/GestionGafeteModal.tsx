import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import Modal from "../componentes/Modal";
import { FilaListaFlotante, ListaFlotante, SinResultados } from "../componentes/ListaFlotante";
import { useListaFlotante, useNavegacionFlechas } from "../componentes/ListaFlotante.logica";
import { useBusquedaContratistas } from "../componentes/useBusquedaContratistas";
import {
  listarEncargadosRutaSeleccionables,
  nombrePortador,
  recuperarGafete,
  registrarPagoGafete,
  reportarGafetePerdido,
  verificarCheckInVisita,
} from "../api";
import type { ContratistaResumen, EncargadoRuta, GafeteResumen, ResponsableGafete } from "../api";
import { LARGO_MAXIMO_DETALLE, detalleLimpio, textoEstado } from "./GestionGafeteModal.logica";

/** Paso en que está la ventana. */
type Paso =
  | { tipo: "acciones" }
  /** Reportar perdido: primero la pregunta, después quién o el detalle. */
  | { tipo: "pregunta" }
  | { tipo: "con_responsable" }
  | { tipo: "sin_responsable" }
  | { tipo: "pago" }
  | { tipo: "recuperar" };

/** El responsable elegido, con el nombre para mostrarlo antes de confirmar. */
interface ResponsableElegido {
  responsable: ResponsableGafete;
  nombre: string;
}

/**
 * Acciones de un gafete puntual. Cada una es un reporte que queda a nombre
 * de quien lo hace, con su detalle en texto libre:
 *
 * - Disponible → **Reportar perdido**: "¿Se sabe a quién se le asignó?".
 *   Sí: se busca a la persona (contratista por cédula o nombre, visita por
 *   cédula, KOF del catálogo de encargados) y el detalle es opcional. No: el
 *   detalle es obligatorio (qué pasó, quién entregó el turno). Proveedor no
 *   tiene catálogo de personas: va directo a "no se sabe".
 * - Perdido con responsable → **Registrar pago**: se le quita la marca, el
 *   gafete sigue perdido hasta que se reimprima.
 * - Perdido → **Recuperar**: apareció o llegó reimpreso; vuelve a estar
 *   disponible. Detalle obligatorio.
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
  const [paso, setPaso] = useState<Paso>({ tipo: "acciones" });
  const [elegido, setElegido] = useState<ResponsableElegido | null>(null);
  const [detalle, setDetalle] = useState("");
  const [filtro, setFiltro] = useState("");
  const [cedulaVisita, setCedulaVisita] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const esVisita = gafete.tipo === "Visita";
  const esProvisionalKof = gafete.tipo === "ProvisionalKof";
  const esProveedor = gafete.tipo === "Proveedor";
  const esContratista = gafete.tipo === "Contratista";
  const responsableActual = nombrePortador(gafete);
  const buscandoContratista = paso.tipo === "con_responsable" && esContratista && !elegido;

  // `cargandoResultados`: señal chica mientras pasa la espera antes de
  // buscar (docs/pendientes.md, "Spinner durante debounce de búsqueda").
  const { resultados, buscando: cargandoResultados } = useBusquedaContratistas(
    filtro,
    buscandoContratista,
    setError,
  );
  const listaVisible = buscandoContratista && filtro.trim().length > 0;
  const { campoRef, posicion: posicionLista } = useListaFlotante(listaVisible);
  const { resaltado, setResaltado, manejarTecla } = useNavegacionFlechas(
    resultados,
    listaVisible,
    elegirContratista,
  );

  // Catálogo de encargados de ruta para KOF -- mismo combobox y catálogo
  // (`listarEncargadosRutaSeleccionables`, sólo activos) que ya usa
  // `EntregarGafeteProvisionalModal`.
  const [encargados, setEncargados] = useState<EncargadoRuta[]>([]);
  useEffect(() => {
    if (!esProvisionalKof) return;
    listarEncargadosRutaSeleccionables()
      .then(setEncargados)
      .catch(() => {});
  }, [esProvisionalKof]);
  const [encargadoTexto, setEncargadoTexto] = useState("");
  const etiquetaEncargado = (encargado: EncargadoRuta) =>
    `${encargado.nombre} · ${encargado.codigo_empleado}`;
  const encargadoElegido = encargados.find(
    (encargado) => etiquetaEncargado(encargado).toLowerCase() === encargadoTexto.trim().toLowerCase(),
  );

  function irA(nuevo: Paso) {
    setError(null);
    setDetalle("");
    setElegido(null);
    setFiltro("");
    setCedulaVisita("");
    setEncargadoTexto("");
    setPaso(nuevo);
  }

  function elegirContratista(contratista: ContratistaResumen) {
    setError(null);
    setElegido({
      responsable: { tipo: "contratista", id: contratista.id },
      nombre: `${contratista.nombre} · ${contratista.cedula}`,
    });
  }

  /** Visitantes sólo se resuelven por cédula exacta, mismo mecanismo que el
   * check-in (`verificarCheckInVisita`). */
  async function buscarVisitante() {
    setError(null);
    setEnviando(true);
    try {
      const { visitante } = await verificarCheckInVisita(cedulaVisita.trim());
      setElegido({
        responsable: { tipo: "visita", id: visitante.id },
        nombre: `${visitante.nombre} · ${cedulaVisita.trim()}`,
      });
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  function elegirEncargado() {
    if (!encargadoElegido) return;
    setElegido({
      responsable: { tipo: "provisional_kof", id: encargadoElegido.id },
      nombre: etiquetaEncargado(encargadoElegido),
    });
  }

  async function enviar(accion: () => Promise<void>) {
    setError(null);
    setEnviando(true);
    try {
      await accion();
      onCambiado();
    } catch (error) {
      setError(String(error));
      setEnviando(false);
    }
  }

  const detalleEnviado = detalleLimpio(detalle);

  const campoDetalle = (etiqueta: string, obligatorio: boolean) => (
    <label className="campo">
      {etiqueta}
      {obligatorio ? "" : " (opcional)"}
      <textarea
        className="sin-mayuscula"
        value={detalle}
        maxLength={LARGO_MAXIMO_DETALLE}
        onChange={(evento) => setDetalle(evento.target.value)}
        autoFocus={!(paso.tipo === "con_responsable" && !elegido)}
        placeholder={
          paso.tipo === "sin_responsable"
            ? "Ej.: Recibo el turno sin el gafete; me entrega Fulano de Tal."
            : paso.tipo === "recuperar"
              ? "Ej.: Lo encontró Fulano en el parqueo / llegó reimpreso."
              : "Lo que haga falta dejar escrito."
        }
      />
    </label>
  );

  const botonesConfirmar = (texto: string, deshabilitado: boolean, accion: () => Promise<void>) => (
    <div style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end" }}>
      <button type="button" className="boton" disabled={enviando} onClick={() => irA({ tipo: "acciones" })}>
        Volver
      </button>
      <button
        type="button"
        className="boton boton-primario"
        disabled={enviando || deshabilitado}
        onClick={() => enviar(accion)}
      >
        {texto}
      </button>
    </div>
  );

  return (
    <Modal titulo={`Gafete ${String(gafete.numero).padStart(2, "0")}`} onCerrar={onCerrar}>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}>
        <p style={{ margin: 0, color: "var(--muted)" }}>
          Tipo: <strong style={{ color: "var(--texto)" }}>{gafete.tipo}</strong>
          {" · "}
          Estado: <strong style={{ color: "var(--texto)" }}>{textoEstado(gafete.estado)}</strong>
        </p>
        {gafete.estado === "Perdido" && (
          <p style={{ margin: 0, color: "var(--muted)" }}>
            Responsable:{" "}
            <strong style={{ color: "var(--texto)" }}>{responsableActual ?? "sin responsable"}</strong>
          </p>
        )}

        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        {paso.tipo === "acciones" && gafete.estado === "Disponible" && (
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button
              type="button"
              className="boton"
              onClick={() => irA(esProveedor ? { tipo: "sin_responsable" } : { tipo: "pregunta" })}
            >
              Reportar perdido…
            </button>
          </div>
        )}

        {paso.tipo === "acciones" && gafete.estado === "Perdido" && (
          <div style={{ display: "flex", gap: "0.5rem" }}>
            {responsableActual && (
              <button type="button" className="boton" onClick={() => irA({ tipo: "pago" })}>
                Registrar pago…
              </button>
            )}
            <button type="button" className="boton" onClick={() => irA({ tipo: "recuperar" })}>
              Recuperar…
            </button>
          </div>
        )}

        {paso.tipo === "pregunta" && (
          <>
            <p style={{ margin: 0 }}>¿Se sabe a quién se le asignó?</p>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button type="button" className="boton" onClick={() => irA({ tipo: "con_responsable" })}>
                Sí, buscar a la persona
              </button>
              <button type="button" className="boton" onClick={() => irA({ tipo: "sin_responsable" })}>
                No se sabe
              </button>
              <button type="button" className="boton" onClick={() => irA({ tipo: "acciones" })}>
                Volver
              </button>
            </div>
          </>
        )}

        {paso.tipo === "con_responsable" && elegido && (
          <p style={{ margin: 0, color: "var(--muted)" }}>
            Se le asignó a: <strong style={{ color: "var(--texto)" }}>{elegido.nombre}</strong>{" "}
            <button
              type="button"
              className="boton"
              style={{ padding: "0.1rem 0.5rem", fontSize: "var(--tam-xs)" }}
              onClick={() => setElegido(null)}
            >
              Cambiar
            </button>
          </p>
        )}

        {paso.tipo === "con_responsable" && !elegido && esVisita && (
          <form
            style={{ display: "flex", gap: "0.5rem", alignItems: "flex-end" }}
            onSubmit={(evento) => {
              evento.preventDefault();
              buscarVisitante();
            }}
          >
            <label className="campo" style={{ flex: 1 }}>
              Cédula del visitante
              <input
                value={cedulaVisita}
                onChange={(evento) => setCedulaVisita(evento.target.value)}
                autoFocus
                placeholder="Cédula…"
              />
            </label>
            <button type="submit" className="boton" disabled={enviando || !cedulaVisita.trim()}>
              Buscar
            </button>
          </form>
        )}

        {paso.tipo === "con_responsable" && !elegido && esProvisionalKof && (
          <form
            style={{ display: "flex", gap: "0.5rem", alignItems: "flex-end" }}
            onSubmit={(evento) => {
              evento.preventDefault();
              elegirEncargado();
            }}
          >
            <label className="campo" style={{ flex: 1 }}>
              Encargado de ruta
              <input
                list="encargados-perdido-datalist"
                value={encargadoTexto}
                onChange={(evento) => setEncargadoTexto(evento.target.value)}
                autoFocus
                autoComplete="off"
                placeholder="Escriba para buscar por nombre o código…"
              />
              <datalist id="encargados-perdido-datalist">
                {encargados.map((encargado) => (
                  <option key={encargado.id} value={etiquetaEncargado(encargado)} />
                ))}
              </datalist>
            </label>
            <button type="submit" className="boton" disabled={!encargadoElegido}>
              Elegir
            </button>
          </form>
        )}

        {paso.tipo === "con_responsable" && !elegido && esContratista && (
          <div ref={campoRef}>
            <label className="campo">
              Cédula o nombre del contratista
              <div style={{ position: "relative" }}>
                <input
                  value={filtro}
                  onChange={(evento) => setFiltro(evento.target.value)}
                  onKeyDown={manejarTecla}
                  autoFocus
                  placeholder="Cédula o nombre…"
                />
                {cargandoResultados && (
                  <Loader2
                    size={14}
                    strokeWidth={2}
                    aria-hidden="true"
                    className="girando"
                    style={{
                      position: "absolute",
                      right: "0.6rem",
                      top: "50%",
                      transform: "translateY(-50%)",
                      color: "var(--muted)",
                    }}
                  />
                )}
              </div>
            </label>
            {listaVisible && posicionLista && (
              <ListaFlotante posicion={posicionLista}>
                {resultados.length === 0 && <SinResultados />}
                {resultados.map((contratista, indice) => (
                  <FilaListaFlotante
                    key={contratista.id}
                    resaltada={indice === resaltado}
                    onClick={() => elegirContratista(contratista)}
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

        {paso.tipo === "con_responsable" && (
          <>
            {campoDetalle("Detalle", false)}
            {botonesConfirmar("Reportar perdido", !elegido, () =>
              reportarGafetePerdido(gafete.id, elegido?.responsable ?? null, detalleEnviado),
            )}
          </>
        )}

        {paso.tipo === "sin_responsable" && (
          <>
            <p style={{ margin: 0, color: "var(--muted)" }}>
              Nadie queda marcado. Cuente qué pasó: el reporte queda a su nombre.
            </p>
            {campoDetalle("Detalle", true)}
            {botonesConfirmar("Reportar perdido", !detalleEnviado, () =>
              reportarGafetePerdido(gafete.id, null, detalleEnviado),
            )}
          </>
        )}

        {paso.tipo === "pago" && (
          <>
            <p style={{ margin: 0, color: "var(--muted)" }}>
              Se le quita la marca a {responsableActual}. El gafete sigue perdido (no se puede
              asignar) hasta que se recupere o llegue reimpreso.
            </p>
            {campoDetalle("Detalle", false)}
            {botonesConfirmar("Registrar pago", false, () =>
              registrarPagoGafete(gafete.id, detalleEnviado),
            )}
          </>
        )}

        {paso.tipo === "recuperar" && (
          <>
            <p style={{ margin: 0, color: "var(--muted)" }}>
              El gafete vuelve a estar disponible
              {responsableActual ? ` y se le quita la marca a ${responsableActual}` : ""}.
            </p>
            {campoDetalle("Detalle", true)}
            {botonesConfirmar("Recuperar", !detalleEnviado, () =>
              recuperarGafete(gafete.id, detalleEnviado ?? ""),
            )}
          </>
        )}

        {paso.tipo === "acciones" && (
          <div style={{ display: "flex", justifyContent: "flex-end" }}>
            <button type="button" className="boton" onClick={onCerrar}>
              Cerrar
            </button>
          </div>
        )}
      </div>
    </Modal>
  );
}
