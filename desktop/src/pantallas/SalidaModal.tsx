import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import Modal from "../componentes/Modal";
import { listarIngresosActivos, registrarSalida } from "../api";
import type { IngresoActivoResumen } from "../api";

const MAX_RESULTADOS = 4;
const MAX_LARGO_GAFETES = 60;

/** Mismo criterio que `SalidaGafeteState::asignar_texto` (`--comandos`):
 * sólo dígitos, coma (separador de lista) y espacio. */
function sanearGafetes(texto: string): string {
  return texto
    .split("")
    .filter((c) => /[\d,\s]/.test(c))
    .slice(0, MAX_LARGO_GAFETES)
    .join("");
}

function gafetesDe(texto: string): number[] {
  return texto
    .split(",")
    .map((token) => token.trim())
    .filter((token) => token.length > 0)
    .map(Number)
    .filter((n) => Number.isInteger(n));
}

function coincideTexto(activo: IngresoActivoResumen, textoBuscado: string): boolean {
  const buscado = textoBuscado.toLowerCase();
  return (
    activo.contratista_nombre.toLowerCase().includes(buscado) ||
    activo.cedula.toLowerCase().includes(buscado)
  );
}

type Seleccion = { tipo: "ninguna" } | { tipo: "elegido"; activo: IngresoActivoResumen };

/**
 * Un solo modal para las dos formas de encontrar a quién dar salida — un
 * checkbox junto al buscador cambia cómo se interpreta el mismo campo de
 * texto, en vez de mantener dos modales casi idénticos (ambos ya
 * necesitaban la misma lista de activos, el mismo "queda abierto tras
 * confirmar", el mismo foco de vuelta al buscador):
 *
 * - Modo normal (desmarcado): busca por cédula o nombre entre los
 *   ingresos activos — elegir uno expande el panel de confirmación debajo
 *   (mismo patrón que Nuevo Ingreso), Enter/click en "Registrar salida"
 *   confirma esa persona.
 * - Modo gafete (marcado): el texto se interpreta como números de gafete
 *   separados por coma — Enter confirma TODOS los que coincidan de una,
 *   sin paso de confirmación (el gafete ya es único entre activos).
 *
 * En ambos casos el modal NO se cierra al confirmar: limpia el campo y
 * vuelve a quedar listo para la siguiente persona/grupo, igual que Nuevo
 * Ingreso — sólo Esc lo cierra.
 */
export default function SalidaModal({
  onRegistrado,
  onCerrar,
}: {
  onRegistrado: () => void;
  onCerrar: () => void;
}) {
  const [activos, setActivos] = useState<IngresoActivoResumen[]>([]);
  const [modoGafete, setModoGafete] = useState(false);
  const [texto, setTexto] = useState("");
  const [seleccion, setSeleccion] = useState<Seleccion>({ tipo: "ninguna" });
  const [resaltado, setResaltado] = useState(0);
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const campoRef = useRef<HTMLDivElement>(null);
  const buscadorRef = useRef<HTMLInputElement>(null);
  const [posicionLista, setPosicionLista] = useState<{ top: number; left: number; width: number } | null>(
    null,
  );

  const cargarActivos = () => listarIngresosActivos().then((p) => setActivos(p.items));

  useEffect(() => {
    cargarActivos().catch((error) => setError(String(error)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const porGafete = useMemo(() => {
    const mapa = new Map<number, IngresoActivoResumen>();
    for (const activo of activos) {
      if (activo.gafete_numero !== null) mapa.set(activo.gafete_numero, activo);
    }
    return mapa;
  }, [activos]);

  const gafetes = useMemo(() => (modoGafete ? gafetesDe(texto) : []), [modoGafete, texto]);

  const resultadosNombre = useMemo(() => {
    if (modoGafete || !texto.trim()) return [];
    return activos.filter((a) => coincideTexto(a, texto.trim())).slice(0, MAX_RESULTADOS);
  }, [modoGafete, texto, activos]);

  const listaNombreVisible = !modoGafete && seleccion.tipo === "ninguna" && texto.trim().length > 0;

  useEffect(() => {
    setResaltado(0);
  }, [resultadosNombre]);

  // Igual que en Nuevo Ingreso: portal a `document.body` para que la lista
  // flote sobre el resto del modal sin que el overflow del contenedor la
  // recorte, posicionada por coordenadas reales del campo.
  useLayoutEffect(() => {
    if (!listaNombreVisible || !campoRef.current) {
      setPosicionLista(null);
      return;
    }
    const actualizar = () => {
      const rect = campoRef.current!.getBoundingClientRect();
      setPosicionLista({ top: rect.bottom + 4, left: rect.left, width: rect.width });
    };
    actualizar();
    window.addEventListener("resize", actualizar);
    return () => window.removeEventListener("resize", actualizar);
  }, [listaNombreVisible]);

  function cambiarModo(gafete: boolean) {
    setModoGafete(gafete);
    setTexto("");
    setSeleccion({ tipo: "ninguna" });
    setMensaje(null);
    setError(null);
    // El click en el checkbox se lleva el foco — sin esto, hay que hacer
    // un segundo click aparte en el campo antes de poder escribir.
    buscadorRef.current?.focus();
  }

  function cambiarTexto(valor: string) {
    setTexto(modoGafete ? sanearGafetes(valor) : valor);
    setMensaje(null);
    setError(null);
    if (seleccion.tipo !== "ninguna") setSeleccion({ tipo: "ninguna" });
  }

  function manejarTeclaBuscador(evento: KeyboardEvent<HTMLInputElement>) {
    if (modoGafete || !listaNombreVisible || resultadosNombre.length === 0) return;
    if (evento.key === "ArrowDown") {
      evento.preventDefault();
      setResaltado((actual) => Math.min(actual + 1, resultadosNombre.length - 1));
    } else if (evento.key === "ArrowUp") {
      evento.preventDefault();
      setResaltado((actual) => Math.max(actual - 1, 0));
    } else if (evento.key === "Enter") {
      evento.preventDefault();
      setSeleccion({ tipo: "elegido", activo: resultadosNombre[resaltado] });
    }
  }

  async function confirmarNombre() {
    if (seleccion.tipo !== "elegido") return;
    setError(null);
    setEnviando(true);
    try {
      await registrarSalida(seleccion.activo.registro_id);
      setMensaje(`✓ Salida registrada — ${seleccion.activo.contratista_nombre}`);
      setSeleccion({ tipo: "ninguna" });
      setTexto("");
      buscadorRef.current?.focus();
      await cargarActivos();
      onRegistrado();
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  async function confirmarGafete() {
    if (gafetes.length === 0) return;
    setEnviando(true);
    setError(null);
    const registrados: string[] = [];
    const fallidos: string[] = [];
    for (const numero of gafetes) {
      const activo = porGafete.get(numero);
      if (!activo) {
        fallidos.push(`gafete ${numero}: sin ingreso activo`);
        continue;
      }
      try {
        await registrarSalida(activo.registro_id);
        registrados.push(activo.contratista_nombre);
      } catch (error) {
        fallidos.push(`gafete ${numero}: ${String(error)}`);
      }
    }

    setTexto("");
    setMensaje(registrados.length > 0 ? `✓ Salida registrada — ${registrados.join(", ")}` : null);
    setError(fallidos.length > 0 ? fallidos.join(" · ") : null);
    setEnviando(false);
    buscadorRef.current?.focus();
    await cargarActivos();
    if (registrados.length > 0) onRegistrado();
  }

  return (
    <Modal titulo="Salida" onCerrar={onCerrar}>
      <form
        onSubmit={(evento) => {
          evento.preventDefault();
          if (modoGafete) confirmarGafete();
          else confirmarNombre();
        }}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}
      >
        <div style={{ display: "flex", gap: "0.6rem", alignItems: "flex-end" }}>
          <div ref={campoRef} style={{ flex: 1 }}>
            <label className="campo">
              {modoGafete ? "Números de gafete" : "Buscar por cédula o nombre"}
              <input
                ref={buscadorRef}
                value={texto}
                onChange={(evento) => cambiarTexto(evento.target.value)}
                onKeyDown={manejarTeclaBuscador}
                autoFocus
                inputMode={modoGafete ? "numeric" : "text"}
                placeholder={modoGafete ? "Ej. 2, 25, 85" : "Cédula o nombre…"}
              />
            </label>
          </div>

          <label
            style={{
              display: "flex",
              alignItems: "center",
              gap: "0.4rem",
              paddingBottom: "0.6rem",
              color: "var(--texto)",
              whiteSpace: "nowrap",
              fontSize: "0.9rem",
            }}
          >
            <input
              type="checkbox"
              checked={modoGafete}
              onChange={(evento) => cambiarModo(evento.target.checked)}
            />
            Por gafete
          </label>
        </div>

        {/* Lista de nombre/cédula, flotante — no debe empujar el resto del
            modal (ver mismo criterio en NuevoIngresoModal). */}
        {listaNombreVisible &&
          posicionLista &&
          createPortal(
            <div
              className="tarjeta"
              style={{
                position: "fixed",
                top: posicionLista.top,
                left: posicionLista.left,
                width: posicionLista.width,
                zIndex: 1000,
                display: "flex",
                flexDirection: "column",
                overflow: "hidden",
                boxShadow: "var(--sombra-panel)",
              }}
            >
              {resultadosNombre.length === 0 && (
                <p style={{ margin: 0, padding: "0.75rem", color: "var(--muted)" }}>
                  Sin resultados.
                </p>
              )}
              {resultadosNombre.map((activo, indice) => (
                <button
                  key={activo.registro_id}
                  type="button"
                  onClick={() => setSeleccion({ tipo: "elegido", activo })}
                  onMouseEnter={() => setResaltado(indice)}
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    gap: "0.75rem",
                    padding: "0.6rem 0.8rem",
                    border: "none",
                    borderBottom: "1px solid var(--borde)",
                    background: indice === resaltado ? "var(--acento-suave)" : "var(--panel)",
                    boxShadow: indice === resaltado ? "inset 3px 0 0 var(--acento)" : "none",
                    color: "var(--texto)",
                    textAlign: "left",
                    cursor: "pointer",
                  }}
                >
                  <span>
                    {activo.contratista_nombre}{" "}
                    <span style={{ color: "var(--muted)" }}>· {activo.cedula}</span>
                  </span>
                  <span style={{ color: "var(--muted)", fontSize: "0.85rem" }}>
                    {activo.empresa_nombre}
                  </span>
                </button>
              ))}
            </div>,
            document.body,
          )}

        {mensaje && <p style={{ color: "var(--exito)", margin: 0 }}>{mensaje}</p>}
        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        {!modoGafete && seleccion.tipo === "elegido" && (
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              gap: "0.75rem",
              padding: "0.85rem",
              border: "1px solid var(--borde)",
              borderRadius: "var(--radio-chico)",
              background: "var(--campo-fondo)",
            }}
          >
            <div>
              <p style={{ margin: 0, fontWeight: 600, color: "var(--texto)" }}>
                {seleccion.activo.contratista_nombre}
              </p>
              <p style={{ margin: "0.15rem 0 0", color: "var(--muted)", fontSize: "0.85rem" }}>
                {seleccion.activo.cedula} · {seleccion.activo.empresa_nombre}
              </p>
            </div>
            <button type="submit" className="boton boton-primario" disabled={enviando}>
              {enviando ? "Registrando…" : "Registrar salida"}
            </button>
          </div>
        )}

        {modoGafete && gafetes.length > 0 && (
          <>
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                border: "1px solid var(--borde)",
                borderRadius: "var(--radio-chico)",
                overflow: "hidden",
              }}
            >
              {gafetes.map((numero, indice) => {
                const activo = porGafete.get(numero);
                return (
                  <div
                    key={`${numero}-${indice}`}
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      gap: "0.75rem",
                      padding: "0.5rem 0.8rem",
                      borderBottom: "1px solid var(--borde)",
                      fontSize: "0.9rem",
                    }}
                  >
                    <span style={{ color: "var(--muted)" }}>Gafete {numero}</span>
                    {activo ? (
                      <span style={{ color: "var(--texto)" }}>
                        {activo.contratista_nombre} · {activo.empresa_nombre}
                      </span>
                    ) : (
                      <span style={{ color: "var(--error)" }}>Sin ingreso activo</span>
                    )}
                  </div>
                );
              })}
            </div>

            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button type="submit" className="boton boton-primario" disabled={enviando}>
                {enviando ? "Registrando…" : "Registrar salida"}
              </button>
            </div>
          </>
        )}
      </form>
    </Modal>
  );
}
