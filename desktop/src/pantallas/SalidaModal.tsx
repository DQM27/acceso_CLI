import { useEffect, useMemo, useRef, useState } from "react";
import { IdCard, UserSearch } from "lucide-react";
import Modal from "../componentes/Modal";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import {
  FilaListaFlotante,
  ListaFlotante,
  SinResultados,
  useListaFlotante,
  useNavegacionFlechas,
} from "../componentes/ListaFlotante";
import { cerrarFilaActiva, claveFilaActiva, gafetesDe, listarTodosLosActivos, sanearGafetes } from "../api";
import type { FilaActiva } from "../api";

const MAX_RESULTADOS = 4;

type ModoBusqueda = "nombre" | "gafete";

/** "GAFETE 2" / "S/G" (sin gafete), en mayúsculas como el resto de los
 * datos del contratista. */
export function textoGafete(numero: number | null): string {
  return numero == null ? "S/G" : `GAFETE ${numero}`;
}

const OPCIONES_MODO: OpcionSegmentada<ModoBusqueda>[] = [
  { valor: "nombre", Icono: UserSearch, titulo: "Nombre o cédula" },
  { valor: "gafete", Icono: IdCard, titulo: "Gafete" },
];

export function coincideTexto(activo: FilaActiva, textoBuscado: string): boolean {
  const buscado = textoBuscado.toLowerCase();
  return (
    activo.contratista_nombre.toLowerCase().includes(buscado) ||
    (activo.cedula?.toLowerCase().includes(buscado) ?? false)
  );
}

type Seleccion = { tipo: "ninguna" } | { tipo: "elegido"; activo: FilaActiva };

/**
 * Un solo modal para las dos formas de encontrar a quién dar salida — un
 * selector "Nombre o cédula / Gafete" sobre el buscador cambia cómo se interpreta el mismo campo de
 * texto, en vez de mantener dos modales casi idénticos (ambos ya
 * necesitaban la misma lista de activos, el mismo "queda abierto tras
 * confirmar", el mismo foco de vuelta al buscador):
 *
 * - Modo "Nombre o cédula": busca por cédula o nombre entre los
 *   ingresos activos — elegir uno expande el panel de confirmación debajo
 *   (mismo patrón que Nuevo Ingreso), Enter/click en "Registrar salida"
 *   confirma esa persona.
 * - Modo "Gafete": el texto se interpreta como números de gafete
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
  const [activos, setActivos] = useState<FilaActiva[]>([]);
  const [modoGafete, setModoGafete] = useState(false);
  const [texto, setTexto] = useState("");
  const [seleccion, setSeleccion] = useState<Seleccion>({ tipo: "ninguna" });
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const buscadorRef = useRef<HTMLInputElement>(null);

  // Incluye lo abierto por el otro dispositivo del mismo sitio, no sólo lo
  // local -- antes este modal sólo buscaba entre locales, así que una
  // persona activa en otro dispositivo se podía cerrar desde la grilla de
  // Activos pero no aparecía acá al buscarla, inconsistencia real entre las
  // dos vías de dar salida.
  const cargarActivos = () => listarTodosLosActivos().then(({ filas }) => setActivos(filas));

  useEffect(() => {
    cargarActivos().catch((error) => setError(String(error)));
  }, []);

  const porGafete = useMemo(() => {
    const mapa = new Map<number, FilaActiva>();
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
    // El click en el selector de modo se lleva el foco — sin esto, hay que hacer
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
      await cerrarFilaActiva(seleccion.activo);
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
        await cerrarFilaActiva(activo);
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
        {/* Modo de búsqueda al lado del buscador, discreto, donde estaba el
            checkbox "Por gafete" -- pero como botones de ícono, fáciles de
            pulsar (el checkbox era difícil de atinar; pedido del usuario
            2026-09-23). Mismo relleno deslizante del resto de la app. */}
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
          <SegmentadoOpciones
            opciones={OPCIONES_MODO}
            valor={modoGafete ? "gafete" : "nombre"}
            onCambiar={(modo) => cambiarModo(modo === "gafete")}
            etiqueta="Buscar por"
          />
        </div>

        {listaNombreVisible && posicionLista && (
          <ListaFlotante posicion={posicionLista}>
            {resultadosNombre.length === 0 && <SinResultados />}
            {resultadosNombre.map((activo, indice) => (
              <FilaListaFlotante
                key={claveFilaActiva(activo)}
                resaltada={indice === resaltado}
                onClick={() => setSeleccion({ tipo: "elegido", activo })}
                onMouseEnter={() => setResaltado(indice)}
              >
                <span>
                  {activo.contratista_nombre}{" "}
                  <span style={{ color: "var(--muted)" }}>· {activo.cedula ?? "—"}</span>
                </span>
                <span style={{ color: "var(--muted)", fontSize: "0.85rem" }}>
                  {activo.empresa_nombre ?? "—"}
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
            className="ficha-desplegable"
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
                {seleccion.activo.cedula ?? "—"} · {seleccion.activo.empresa_nombre ?? "—"} ·{" "}
                {textoGafete(seleccion.activo.gafete_numero)}
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
              className="ficha-desplegable"
              style={{
                display: "flex",
                flexDirection: "column",
                border: "1px solid var(--borde)",
                borderRadius: "var(--radio-chico)",
                // Mismo fondo oscuro que la ficha del modo nombre, pero en
                // filas compactas: acá pueden ser varios gafetes a la vez.
                background: "var(--campo-fondo)",
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
                      padding: "0.6rem 0.85rem",
                      borderTop: indice > 0 ? "1px solid var(--borde)" : undefined,
                      fontSize: "0.9rem",
                    }}
                  >
                    {/* Primero quién (nombre y empresa), después el gafete
                        -- pedido del usuario 2026-09-23. */}
                    {activo ? (
                      <span style={{ color: "var(--texto)", fontWeight: 600 }}>
                        {activo.contratista_nombre} · {activo.empresa_nombre}
                      </span>
                    ) : (
                      <span style={{ color: "var(--error)" }}>Sin ingreso activo</span>
                    )}
                    <span style={{ color: "var(--muted)", whiteSpace: "nowrap" }}>
                      {textoGafete(numero)}
                    </span>
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
