import { useEffect, useMemo, useRef, useState } from "react";
import Modal from "../componentes/Modal";
import {
  FilaListaFlotante,
  ListaFlotante,
  SinResultados,
  useListaFlotante,
  useNavegacionFlechas,
} from "../componentes/ListaFlotante";
import { gafetesDe, listarIngresosActivos, registrarSalida, sanearGafetes } from "../api";
import type { IngresoActivoResumen } from "../api";

const MAX_RESULTADOS = 4;

export function coincideTexto(activo: IngresoActivoResumen, textoBuscado: string): boolean {
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
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const buscadorRef = useRef<HTMLInputElement>(null);

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
  const { campoRef, posicion: posicionLista } = useListaFlotante(listaNombreVisible);
  const { resaltado, setResaltado, manejarTecla: manejarTeclaBuscador } = useNavegacionFlechas(
    resultadosNombre,
    listaNombreVisible,
    (activo) => setSeleccion({ tipo: "elegido", activo }),
  );

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

        {listaNombreVisible && posicionLista && (
          <ListaFlotante posicion={posicionLista}>
            {resultadosNombre.length === 0 && <SinResultados />}
            {resultadosNombre.map((activo, indice) => (
              <FilaListaFlotante
                key={activo.registro_id}
                resaltada={indice === resaltado}
                onClick={() => setSeleccion({ tipo: "elegido", activo })}
                onMouseEnter={() => setResaltado(indice)}
              >
                <span>
                  {activo.contratista_nombre}{" "}
                  <span style={{ color: "var(--muted)" }}>· {activo.cedula}</span>
                </span>
                <span style={{ color: "var(--muted)", fontSize: "0.85rem" }}>
                  {activo.empresa_nombre}
                </span>
              </FilaListaFlotante>
            ))}
          </ListaFlotante>
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
