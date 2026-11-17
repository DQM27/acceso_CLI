import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { CircleHelp, HandCoins, Loader2, RotateCcw, UserSearch } from "lucide-react";
import Modal from "../componentes/Modal";
import { FilaListaFlotante, ListaFlotante, SinResultados } from "../componentes/ListaFlotante";
import { useListaFlotante, useNavegacionFlechas } from "../componentes/ListaFlotante.logica";
import { useBusquedaContratistas } from "../componentes/useBusquedaContratistas";
import { useNombreUsuario } from "../contexto/SesionContexto";
import {
  listarEncargadosRutaSeleccionables,
  nombrePortador,
  recuperarGafete,
  registrarPagoGafete,
  reportarGafetePerdido,
  verificarCheckInVisita,
} from "../api";
import type { ContratistaResumen, EncargadoRuta, GafeteResumen, ResponsableGafete } from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY } from "../tiempo";
import { LARGO_MAXIMO_DETALLE, detalleLimpio, textoEstado } from "./GestionGafeteModal.logica";

/** Qué se va a hacer con el gafete, según la tarjeta elegida. */
type Accion = "con_responsable" | "sin_responsable" | "pago" | "recuperar";

const NOMBRE_TIPO: Record<GafeteResumen["tipo"], string> = {
  Contratista: "Contratista",
  Visita: "Visita",
  ProvisionalKof: "Provisional KOF",
  Proveedor: "Proveedor",
};

/** El responsable elegido, con el nombre para mostrarlo antes de confirmar. */
interface ResponsableElegido {
  responsable: ResponsableGafete;
  nombre: string;
}

/**
 * Ventana de un gafete. Cada acción es un reporte que queda a nombre de quien
 * lo hace, con su detalle en texto libre. Las opciones son tarjetas: se elige
 * una y lo que pide aparece debajo, sin cambiar de pantalla; el pie siempre
 * tiene "Cancelar" y la acción, que se habilita cuando el reporte está
 * completo.
 *
 * - Disponible → abre directo en **Reportar perdido**: "Sé quién lo tenía"
 *   (contratista por cédula o nombre, visita por cédula, KOF de la lista de
 *   encargados; detalle opcional) o "No se sabe" (detalle obligatorio).
 *   Proveedor no tiene catálogo de personas: sólo "No se sabe".
 * - Perdido → **Registrar pago** (si tiene responsable: se le quita la marca,
 *   el gafete sigue perdido) o **Recuperar** (apareció o llegó reimpreso;
 *   detalle obligatorio).
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
  const esVisita = gafete.tipo === "Visita";
  const esProvisionalKof = gafete.tipo === "ProvisionalKof";
  const esProveedor = gafete.tipo === "Proveedor";
  const esContratista = gafete.tipo === "Contratista";
  const perdido = gafete.estado === "Perdido";
  const responsableActual = nombrePortador(gafete);
  const nombreUsuario = useNombreUsuario();

  // Si sólo hay una opción, ya viene elegida.
  const [accion, setAccion] = useState<Accion | null>(() => {
    if (!perdido) return esProveedor ? "sin_responsable" : null;
    return responsableActual ? null : "recuperar";
  });
  const [elegido, setElegido] = useState<ResponsableElegido | null>(null);
  const [detalle, setDetalle] = useState("");
  const [filtro, setFiltro] = useState("");
  const [cedulaVisita, setCedulaVisita] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const buscandoContratista = accion === "con_responsable" && esContratista && !elegido;
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

  function elegirAccion(nueva: Accion) {
    if (nueva === accion) return;
    setError(null);
    setElegido(null);
    setFiltro("");
    setCedulaVisita("");
    setEncargadoTexto("");
    setAccion(nueva);
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

  const detalleEnviado = detalleLimpio(detalle);
  const detalleObligatorio = accion === "sin_responsable" || accion === "recuperar";

  const listo =
    accion !== null &&
    (accion === "con_responsable" ? elegido !== null : true) &&
    (!detalleObligatorio || detalleEnviado !== null);

  const textoAccion: Record<Accion, string> = {
    con_responsable: "Reportar perdido",
    sin_responsable: "Reportar perdido",
    pago: "Registrar pago",
    recuperar: "Recuperar",
  };

  async function confirmar() {
    if (!accion || !listo) return;
    setError(null);
    setEnviando(true);
    try {
      switch (accion) {
        case "con_responsable":
          await reportarGafetePerdido(gafete.id, elegido?.responsable ?? null, detalleEnviado);
          break;
        case "sin_responsable":
          await reportarGafetePerdido(gafete.id, null, detalleEnviado);
          break;
        case "pago":
          await registrarPagoGafete(gafete.id, detalleEnviado);
          break;
        case "recuperar":
          await recuperarGafete(gafete.id, detalleEnviado ?? "");
          break;
      }
      onCambiado();
    } catch (error) {
      setError(String(error));
      setEnviando(false);
    }
  }

  const perdidoDesde = gafete.fecha_marcado_perdido
    ? textoFechaDDMMYYYY(fechaLocalYMD(gafete.fecha_marcado_perdido))
    : null;

  return (
    <Modal
      titulo={`Gafete ${String(gafete.numero).padStart(2, "0")} · ${NOMBRE_TIPO[gafete.tipo]}`}
      onCerrar={onCerrar}
      ancho="36rem"
    >
      <div style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", flexWrap: "wrap" }}>
          <span
            className="chip"
            style={{ ["--chip-color" as string]: perdido ? "var(--error)" : "var(--exito)" }}
          >
            {textoEstado(gafete.estado)}
          </span>
          {perdido && (
            <span style={{ color: "var(--muted)", fontSize: "var(--tam-sm)" }}>
              {perdidoDesde ? `Desde el ${perdidoDesde} · ` : ""}
              {responsableActual ? `Responsable: ${responsableActual}` : "Sin responsable"}
            </span>
          )}
        </div>

        <div>
          <h3 style={{ margin: "0 0 0.2rem", fontSize: "var(--tam-base)" }}>
            {perdido ? "¿Qué pasó con el gafete?" : "Reportar perdido"}
          </h3>
          {!perdido && (
            <p style={{ margin: 0, color: "var(--muted)" }}>¿Se sabe a quién se le asignó?</p>
          )}
        </div>

        <div className="opciones-tarjeta" role="group" aria-label="Opciones">
          {!perdido && !esProveedor && (
            <Tarjeta
              activa={accion === "con_responsable"}
              onElegir={() => elegirAccion("con_responsable")}
              icono={<UserSearch size={16} aria-hidden="true" />}
              titulo="Sé quién lo tenía"
              texto="Búsquela; queda marcada hasta que pague."
            />
          )}
          {!perdido && (
            <Tarjeta
              activa={accion === "sin_responsable"}
              onElegir={() => elegirAccion("sin_responsable")}
              icono={<CircleHelp size={16} aria-hidden="true" />}
              titulo="No se sabe"
              texto="Cuente qué pasó en el detalle."
            />
          )}
          {perdido && responsableActual && (
            <Tarjeta
              activa={accion === "pago"}
              onElegir={() => elegirAccion("pago")}
              icono={<HandCoins size={16} aria-hidden="true" />}
              titulo="Registrar pago"
              texto="Se le quita la marca; sigue perdido hasta que llegue."
            />
          )}
          {perdido && (
            <Tarjeta
              activa={accion === "recuperar"}
              onElegir={() => elegirAccion("recuperar")}
              icono={<RotateCcw size={16} aria-hidden="true" />}
              titulo="Recuperar"
              texto="Apareció o llegó reimpreso; vuelve a estar disponible."
            />
          )}
        </div>

        {accion === "con_responsable" && elegido && (
          <p style={{ margin: 0, color: "var(--muted)" }}>
            Se le asignó a: <strong style={{ color: "var(--texto)" }}>{elegido.nombre}</strong>{" "}
            <button type="button" className="boton-discreto" onClick={() => setElegido(null)}>
              Cambiar
            </button>
          </p>
        )}

        {accion === "con_responsable" && !elegido && esVisita && (
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

        {accion === "con_responsable" && !elegido && esProvisionalKof && (
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

        {accion === "con_responsable" && !elegido && esContratista && (
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

        {accion !== null && (
          <label className="campo">
            <span style={{ display: "flex", justifyContent: "space-between" }}>
              <span>Detalle{detalleObligatorio ? "" : " (opcional)"}</span>
              <span style={{ fontVariantNumeric: "tabular-nums" }}>
                {detalle.length} / {LARGO_MAXIMO_DETALLE}
              </span>
            </span>
            <textarea
              className="sin-mayuscula"
              value={detalle}
              maxLength={LARGO_MAXIMO_DETALLE}
              onChange={(evento) => setDetalle(evento.target.value)}
              autoFocus={accion !== "con_responsable"}
              placeholder={
                accion === "sin_responsable"
                  ? "Ej.: Recibo el turno sin el gafete; me entrega Fulano de Tal."
                  : accion === "recuperar"
                    ? "Ej.: Lo encontró Fulano en el parqueo / llegó reimpreso."
                    : accion === "pago"
                      ? "Ej.: Número de recibo."
                      : "Lo que haga falta dejar escrito."
              }
            />
          </label>
        )}

        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        {accion !== null && nombreUsuario && (
          <p style={{ margin: 0, color: "var(--muted)", fontSize: "var(--tam-sm)" }}>
            Queda a nombre de: <strong style={{ color: "var(--texto)" }}>{nombreUsuario}</strong>
          </p>
        )}

        <div className="pie-acciones">
          <button type="button" className="boton-discreto" onClick={onCerrar}>
            Cancelar
          </button>
          <button
            type="button"
            className="boton boton-primario"
            disabled={enviando || !listo}
            onClick={confirmar}
          >
            {accion ? textoAccion[accion] : perdido ? "Elija una opción" : "Reportar perdido"}
          </button>
        </div>
      </div>
    </Modal>
  );
}

function Tarjeta({
  activa,
  onElegir,
  icono,
  titulo,
  texto,
}: {
  activa: boolean;
  onElegir: () => void;
  icono: ReactNode;
  titulo: string;
  texto: string;
}) {
  return (
    <button type="button" className="opcion-tarjeta" aria-pressed={activa} onClick={onElegir}>
      <span className="opcion-tarjeta-titulo">
        {icono}
        {titulo}
      </span>
      <span className="opcion-tarjeta-texto">{texto}</span>
    </button>
  );
}
