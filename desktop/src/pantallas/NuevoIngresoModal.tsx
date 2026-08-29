import { useEffect, useRef, useState } from "react";
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
  mensajeBloqueo,
  prepararIngreso,
  puedeContinuar,
  registrarIngreso,
} from "../api";
import type { ContratistaResumen, MedioIngreso, PreparacionIngreso } from "../api";

const DEBOUNCE_MS = 120;
const MAX_RESULTADOS = 4;

type Seleccion =
  | { tipo: "ninguna" }
  | { tipo: "cargando"; contratista: ContratistaResumen }
  | { tipo: "bloqueada"; contratista: ContratistaResumen; mensaje: string }
  | { tipo: "formulario"; contratista: ContratistaResumen; preparacion: PreparacionIngreso };

/**
 * El buscador queda siempre visible arriba; al elegir un contratista el
 * panel correspondiente (formulario o motivo de bloqueo) se expande debajo
 * en el mismo flujo del documento — nada de saltar a otra vista ni cambiar
 * el ancho del modal. Al confirmar un ingreso NO se cierra: colapsa de
 * vuelta al buscador (vacío) con un mensaje de confirmación, listo para la
 * siguiente persona (mismo criterio que
 * `src/tui/nuevo_ingreso/state.rs::completar_registro`). Sólo se cierra si
 * el operador lo cierra a propósito.
 */
export default function NuevoIngresoModal({
  onRegistrado,
  onCerrar,
}: {
  /** Se llama tras cada registro exitoso — la pantalla detrás refresca su
   * listado de activos sin que el modal se cierre. */
  onRegistrado: () => void;
  onCerrar: () => void;
}) {
  const [filtro, setFiltro] = useState("");
  const [resultados, setResultados] = useState<ContratistaResumen[]>([]);
  const [seleccion, setSeleccion] = useState<Seleccion>({ tipo: "ninguna" });
  const [medio, setMedio] = useState<MedioIngreso>("Caminando");
  const [gafeteTexto, setGafeteTexto] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);
  const buscadorRef = useRef<HTMLInputElement>(null);
  const confirmarRef = useRef<HTMLButtonElement>(null);

  const listaVisible = seleccion.tipo === "ninguna" && filtro.trim().length > 0;
  const { campoRef, posicion: posicionLista } = useListaFlotante(listaVisible);
  const { resaltado, setResaltado, manejarTecla: manejarTeclaBuscador } = useNavegacionFlechas(
    resultados,
    listaVisible,
    elegirContratista,
  );

  // Sin gafete no hay ningún campo que se lleve el `autoFocus` del
  // formulario — sin esto el foco se queda en el buscador y Enter no llega
  // a disparar el `<form>` de abajo. Enfocar el botón alcanza: Enter sobre
  // un botón dentro de un <form> lo envía igual que sobre un input de texto.
  useEffect(() => {
    if (seleccion.tipo === "formulario" && !seleccion.preparacion.requiere_gafete) {
      confirmarRef.current?.focus();
    }
  }, [seleccion]);

  useEffect(() => {
    if (!filtro.trim()) {
      setResultados([]);
      return;
    }
    const id = setTimeout(() => {
      buscarContratistas({ texto: filtro })
        .then((pagina) => setResultados(pagina.items.slice(0, MAX_RESULTADOS)))
        .catch((error) => setError(String(error)));
    }, DEBOUNCE_MS);
    return () => clearTimeout(id);
  }, [filtro]);

  function cambiarFiltro(texto: string) {
    setFiltro(texto);
    setMensaje(null);
    // Escribir de nuevo abandona lo que estaba seleccionado — vuelve a
    // buscar en vez de dejar un panel expandido con datos ya viejos.
    if (seleccion.tipo !== "ninguna") {
      setSeleccion({ tipo: "ninguna" });
    }
  }

  async function elegirContratista(contratista: ContratistaResumen) {
    setError(null);
    setSeleccion({ tipo: "cargando", contratista });
    try {
      const preparacion = await prepararIngreso(contratista.id);
      if (puedeContinuar(preparacion)) {
        setMedio("Caminando");
        setGafeteTexto("");
        setSeleccion({ tipo: "formulario", contratista, preparacion });
      } else {
        setSeleccion({ tipo: "bloqueada", contratista, mensaje: mensajeBloqueo(preparacion) });
      }
    } catch (error) {
      setError(String(error));
      setSeleccion({ tipo: "ninguna" });
    }
  }

  function cambiarSeleccion() {
    setError(null);
    setSeleccion({ tipo: "ninguna" });
  }

  async function confirmarIngreso() {
    if (seleccion.tipo !== "formulario") return;
    const { preparacion } = seleccion;
    let gafete: number | null = null;
    if (preparacion.requiere_gafete) {
      const numero = Number.parseInt(gafeteTexto.trim(), 10);
      if (!gafeteTexto.trim() || Number.isNaN(numero)) {
        setError(
          gafeteTexto.trim() ? "Ingrese un número de gafete válido" : "El gafete es requerido",
        );
        return;
      }
      gafete = numero;
    }
    setError(null);
    setEnviando(true);
    try {
      await registrarIngreso(preparacion.contratista_id, medio, gafete);
      setMensaje(`✓ Ingreso registrado — ${preparacion.nombre}`);
      setSeleccion({ tipo: "ninguna" });
      setFiltro("");
      buscadorRef.current?.focus();
      onRegistrado();
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  return (
    <Modal titulo="Nuevo ingreso" onCerrar={onCerrar}>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
        <div ref={campoRef}>
          <label className="campo">
            Buscar contratista
            <input
              ref={buscadorRef}
              value={filtro}
              onChange={(evento) => cambiarFiltro(evento.target.value)}
              onKeyDown={manejarTeclaBuscador}
              autoFocus
              placeholder="Cédula o nombre…"
            />
          </label>
        </div>

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
                <span style={{ color: "var(--muted)", fontSize: "0.85rem" }}>
                  {contratista.empresa_nombre}
                  {contratista.tiene_ingreso_activo && (
                    <span className="chip" style={{ ["--chip-color" as string]: "var(--acento)" }}>
                      Adentro
                    </span>
                  )}
                </span>
              </FilaListaFlotante>
            ))}
          </ListaFlotante>
        )}

        {mensaje && <p style={{ color: "var(--exito)", margin: 0 }}>{mensaje}</p>}
        {error && seleccion.tipo !== "formulario" && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        {seleccion.tipo !== "ninguna" && (
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "0.9rem",
              padding: "0.85rem",
              border: "1px solid var(--borde)",
              borderRadius: "var(--radio-chico)",
              background: "var(--campo-fondo)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
              <div>
                <p style={{ margin: 0, fontWeight: 600, color: "var(--texto)" }}>
                  {seleccion.contratista.nombre}
                </p>
                <p style={{ margin: "0.15rem 0 0", color: "var(--muted)", fontSize: "0.85rem" }}>
                  {seleccion.contratista.cedula} · {seleccion.contratista.empresa_nombre}
                </p>
              </div>
              <button type="button" className="boton" style={{ fontSize: "0.8rem" }} onClick={cambiarSeleccion}>
                Cambiar
              </button>
            </div>

            {seleccion.tipo === "cargando" && (
              <p style={{ margin: 0, color: "var(--muted)" }}>Verificando…</p>
            )}

            {seleccion.tipo === "bloqueada" && (
              <p className="login-error" role="alert">
                {seleccion.mensaje}
              </p>
            )}

            {seleccion.tipo === "formulario" && (
              <form
                onSubmit={(evento) => {
                  evento.preventDefault();
                  confirmarIngreso();
                }}
                style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}
              >
                {seleccion.preparacion.resultado_acceso === "PermitidoConAdvertencia" && (
                  <p style={{ margin: 0, color: "var(--advertencia)", fontSize: "0.85rem" }}>
                    ⚠ PRAIND próximo a vencer
                  </p>
                )}

                <div className="campo">
                  Medio de ingreso
                  <div style={{ display: "flex", gap: "0.75rem" }}>
                    {(["Caminando", "Vehiculo"] as const).map((opcion) => (
                      <label
                        key={opcion}
                        style={{ display: "flex", alignItems: "center", gap: "0.4rem", color: "var(--texto)" }}
                      >
                        <input
                          type="radio"
                          name="medio"
                          checked={medio === opcion}
                          onChange={() => setMedio(opcion)}
                        />
                        {opcion === "Caminando" ? "Caminando" : "Vehículo"}
                      </label>
                    ))}
                  </div>
                </div>

                {seleccion.preparacion.requiere_gafete && (
                  <label className="campo">
                    Número de gafete
                    <input
                      value={gafeteTexto}
                      onChange={(evento) => setGafeteTexto(evento.target.value.replace(/\D/g, ""))}
                      inputMode="numeric"
                      autoFocus
                      placeholder="Número de gafete"
                    />
                  </label>
                )}

                {error && (
                  <p className="login-error" role="alert">
                    {error}
                  </p>
                )}

                <div style={{ display: "flex", justifyContent: "flex-end" }}>
                  <button
                    ref={confirmarRef}
                    type="submit"
                    className="boton boton-primario"
                    disabled={enviando}
                  >
                    {enviando ? "Registrando…" : "Registrar entrada"}
                  </button>
                </div>
              </form>
            )}
          </div>
        )}
      </div>
    </Modal>
  );
}
