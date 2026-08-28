import { useEffect, useState } from "react";
import Modal from "../componentes/Modal";
import {
  buscarContratistas,
  mensajeBloqueo,
  prepararIngreso,
  puedeContinuar,
  registrarIngreso,
} from "../api";
import type { ContratistaResumen, MedioIngreso, PreparacionIngreso } from "../api";

const DEBOUNCE_MS = 120;

type Etapa =
  | { tipo: "buscar" }
  | { tipo: "bloqueado"; nombre: string; mensaje: string }
  | { tipo: "formulario"; preparacion: PreparacionIngreso };

/**
 * Modal que se transforma en dos etapas (buscar → formulario) en vez de
 * navegar a otra pantalla — así el operador no pierde el listado/contexto
 * de búsqueda entre un registro y el siguiente. Al confirmar un ingreso NO
 * se cierra: vuelve a la etapa de búsqueda con un mensaje de confirmación,
 * lista para la siguiente persona (mismo criterio que
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
  const [etapa, setEtapa] = useState<Etapa>({ tipo: "buscar" });
  const [medio, setMedio] = useState<MedioIngreso>("Caminando");
  const [gafeteTexto, setGafeteTexto] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [cargando, setCargando] = useState(false);
  const [enviando, setEnviando] = useState(false);

  useEffect(() => {
    const id = setTimeout(() => {
      buscarContratistas({ texto: filtro || undefined })
        .then((pagina) => setResultados(pagina.items))
        .catch((error) => setError(String(error)));
    }, DEBOUNCE_MS);
    return () => clearTimeout(id);
  }, [filtro]);

  async function elegirContratista(contratista: ContratistaResumen) {
    setError(null);
    setMensaje(null);
    setCargando(true);
    try {
      const preparacion = await prepararIngreso(contratista.id);
      if (puedeContinuar(preparacion)) {
        setMedio("Caminando");
        setGafeteTexto("");
        setEtapa({ tipo: "formulario", preparacion });
      } else {
        setEtapa({
          tipo: "bloqueado",
          nombre: contratista.nombre,
          mensaje: mensajeBloqueo(preparacion),
        });
      }
    } catch (error) {
      setError(String(error));
    } finally {
      setCargando(false);
    }
  }

  async function confirmarIngreso() {
    if (etapa.tipo !== "formulario") return;
    const { preparacion } = etapa;
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
      setEtapa({ tipo: "buscar" });
      onRegistrado();
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  function volverABuscar() {
    setError(null);
    setEtapa({ tipo: "buscar" });
  }

  return (
    <Modal titulo="Nuevo ingreso" onCerrar={onCerrar}>
      {etapa.tipo === "buscar" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
          <label className="campo">
            Buscar contratista
            <input
              value={filtro}
              onChange={(evento) => {
                setFiltro(evento.target.value);
                setMensaje(null);
              }}
              autoFocus
              placeholder="Cédula o nombre…"
            />
          </label>

          {mensaje && <p style={{ color: "var(--exito)", margin: 0 }}>{mensaje}</p>}
          {error && <p style={{ color: "var(--error)", margin: 0 }}>{error}</p>}

          <div
            style={{
              display: "flex",
              flexDirection: "column",
              maxHeight: "18rem",
              overflowY: "auto",
              border: "1px solid var(--borde)",
              borderRadius: "var(--radio-chico)",
            }}
          >
            {resultados.length === 0 && (
              <p style={{ margin: 0, padding: "0.75rem", color: "var(--muted)" }}>
                {filtro ? "Sin resultados." : "Escriba para buscar un contratista."}
              </p>
            )}
            {resultados.map((contratista) => (
              <button
                key={contratista.id}
                type="button"
                disabled={cargando}
                onClick={() => elegirContratista(contratista)}
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  gap: "0.75rem",
                  padding: "0.6rem 0.8rem",
                  border: "none",
                  borderBottom: "1px solid var(--borde)",
                  background: "transparent",
                  color: "var(--texto)",
                  textAlign: "left",
                  cursor: "pointer",
                }}
              >
                <span>
                  <strong>{contratista.nombre}</strong>{" "}
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
              </button>
            ))}
          </div>
        </div>
      )}

      {etapa.tipo === "bloqueado" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          <p style={{ margin: 0 }}>
            <strong>{etapa.nombre}</strong>
          </p>
          <p className="login-error" role="alert">
            {etapa.mensaje}
          </p>
          <button type="button" className="boton" onClick={volverABuscar}>
            Volver a buscar
          </button>
        </div>
      )}

      {etapa.tipo === "formulario" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          <div>
            <p style={{ margin: 0, fontWeight: 600, color: "var(--texto)" }}>
              {etapa.preparacion.nombre}
            </p>
            <p style={{ margin: "0.15rem 0 0", color: "var(--muted)", fontSize: "0.85rem" }}>
              {etapa.preparacion.cedula} · {etapa.preparacion.empresa_nombre}
            </p>
            {etapa.preparacion.resultado_acceso === "PermitidoConAdvertencia" && (
              <p style={{ margin: "0.5rem 0 0", color: "var(--advertencia)", fontSize: "0.85rem" }}>
                ⚠ PRAIND próximo a vencer
              </p>
            )}
          </div>

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

          {etapa.preparacion.requiere_gafete && (
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

          <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
            <button type="button" className="boton" onClick={volverABuscar}>
              Volver
            </button>
            <button
              type="button"
              className="boton boton-primario"
              disabled={enviando}
              onClick={confirmarIngreso}
            >
              {enviando ? "Registrando…" : "Registrar entrada"}
            </button>
          </div>
        </div>
      )}
    </Modal>
  );
}
