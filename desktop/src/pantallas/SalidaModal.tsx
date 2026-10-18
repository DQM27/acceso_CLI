import { useEffect, useMemo, useRef, useState } from "react";
import { IdCard, UserSearch } from "lucide-react";
import Modal from "../componentes/Modal";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import { FilaListaFlotante, ListaFlotante, SinResultados } from "../componentes/ListaFlotante";
import { useListaFlotante, useNavegacionFlechas } from "../componentes/ListaFlotante.logica";
import { gafetesDe, sanearGafetes } from "../api";
import { cerrarActivo, listarActivosUnificados } from "../api/activosUnificados";
import type { ActivoUnificado } from "../api/activosUnificados";
import { CLASE_SINGULAR } from "./Activos.logica";
import { activoParaGafete, agruparPorGafete, coincideTexto, textoGafete } from "./SalidaModal.logica";

const MAX_RESULTADOS = 4;

type ModoBusqueda = "nombre" | "gafete";

const OPCIONES_MODO: OpcionSegmentada<ModoBusqueda>[] = [
  { valor: "nombre", Icono: UserSearch, titulo: "Nombre o cédula" },
  { valor: "gafete", Icono: IdCard, titulo: "Gafete" },
];

type Seleccion = { tipo: "ninguna" } | { tipo: "elegido"; activo: ActivoUnificado };

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
  const [activos, setActivos] = useState<ActivoUnificado[]>([]);
  // Un número de gafete puede tenerlo más de uno (conjuntos de gafetes
  // distintos): a cuál se le da la salida, por número, cuando hay varios.
  const [elecciones, setElecciones] = useState<Record<number, string>>({});
  const [modoGafete, setModoGafete] = useState(false);
  const [texto, setTexto] = useState("");
  const [seleccion, setSeleccion] = useState<Seleccion>({ tipo: "ninguna" });
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const buscadorRef = useRef<HTMLInputElement>(null);

  // Incluye lo abierto por el otro dispositivo del mismo sitio, no sólo lo
  // local, y de TODAS las clases (contratistas, proveedores, visitas por correo
  // y KOF), la misma lista que la grilla de Activos: antes sólo buscaba
  // contratistas y el resto sólo se podía sacar desde la grilla.
  const cargarActivos = () => listarActivosUnificados().then(({ activos }) => setActivos(activos));

  useEffect(() => {
    cargarActivos().catch((error) => setError(String(error)));
  }, []);

  const porGafete = useMemo(() => agruparPorGafete(activos), [activos]);

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
    setElecciones({});
    setSeleccion({ tipo: "ninguna" });
    setMensaje(null);
    setError(null);
    // El click en el selector de modo se lleva el foco — sin esto, hay que hacer
    // un segundo click aparte en el campo antes de poder escribir.
    buscadorRef.current?.focus();
  }

  // Hay un número con varios dueños y todavía no se eligió a cuál: no se
  // puede confirmar, para no dar la salida a quien no era.
  const faltaElegir = modoGafete && gafetes.some((numero) => {
    const coincidencias = porGafete.get(numero);
    return (coincidencias?.length ?? 0) > 1 && !activoParaGafete(coincidencias, elecciones[numero]);
  });

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
      await cerrarActivo(seleccion.activo);
      setMensaje(`✓ Salida registrada — ${seleccion.activo.nombre}`);
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
      const activo = activoParaGafete(porGafete.get(numero), elecciones[numero]);
      if (!activo) {
        fallidos.push(`gafete ${numero}: sin ingreso activo`);
        continue;
      }
      try {
        await cerrarActivo(activo);
        registrados.push(activo.nombre);
      } catch (error) {
        fallidos.push(`gafete ${numero}: ${String(error)}`);
      }
    }

    setTexto("");
    setElecciones({});
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
                key={activo.clave}
                resaltada={indice === resaltado}
                onClick={() => setSeleccion({ tipo: "elegido", activo })}
                onMouseEnter={() => setResaltado(indice)}
              >
                <span>
                  {activo.nombre}{" "}
                  <span style={{ color: "var(--muted)" }}>· {activo.cedula ?? "—"}</span>
                </span>
                {/* La clase va siempre: el buscador mezcla las cuatro. */}
                <span style={{ color: "var(--muted)", fontSize: "0.85rem" }}>
                  {CLASE_SINGULAR[activo.clase]} · {activo.detalle ?? "—"}
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
                {seleccion.activo.nombre}
              </p>
              <p style={{ margin: "0.15rem 0 0", color: "var(--muted)", fontSize: "0.85rem" }}>
                {CLASE_SINGULAR[seleccion.activo.clase]} · {seleccion.activo.cedula ?? "—"} ·{" "}
                {seleccion.activo.detalle ?? "—"} · {textoGafete(seleccion.activo.gafete_numero)}
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
                const coincidencias = porGafete.get(numero) ?? [];
                const activo = activoParaGafete(coincidencias, elecciones[numero]);
                return (
                  <div
                    key={`${numero}-${indice}`}
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      gap: "0.4rem",
                      padding: "0.6rem 0.85rem",
                      borderTop: indice > 0 ? "1px solid var(--borde)" : undefined,
                      fontSize: "0.9rem",
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", gap: "0.75rem" }}>
                      {/* Primero quién (nombre y empresa), después el gafete
                          -- pedido del usuario 2026-09-23. */}
                      {coincidencias.length === 0 ? (
                        <span style={{ color: "var(--error)" }}>Sin ingreso activo</span>
                      ) : coincidencias.length === 1 ? (
                        <span style={{ color: "var(--texto)" }}>
                          <span style={{ fontWeight: 500 }}>{CLASE_SINGULAR[coincidencias[0].clase]}</span>
                          {` · ${coincidencias[0].nombre}`}
                          {coincidencias[0].detalle ? ` · ${coincidencias[0].detalle}` : ""}
                        </span>
                      ) : (
                        <span style={{ color: "var(--advertencia)" }}>
                          {coincidencias.length} personas tienen este gafete — elegí a quién:
                        </span>
                      )}
                      <span style={{ color: "var(--muted)", whiteSpace: "nowrap" }}>
                        {textoGafete(numero)}
                      </span>
                    </div>
                    {/* Los gafetes de cada clase numeran por su cuenta: el 1 de
                        un proveedor y el 1 de un contratista son dos gafetes
                        distintos. No se adivina, se elige. */}
                    {coincidencias.length > 1 && (
                      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.4rem" }}>
                        {coincidencias.map((opcion) => (
                          <button
                            key={opcion.clave}
                            type="button"
                            className={
                              activo?.clave === opcion.clave ? "boton boton-primario" : "boton"
                            }
                            aria-pressed={activo?.clave === opcion.clave}
                            onClick={() =>
                              setElecciones((actuales) => ({ ...actuales, [numero]: opcion.clave }))
                            }
                          >
                            {/* El tipo con más peso y el nombre normal: lo primero
                                que se lee es de quién es el gafete. */}
                            <span style={{ fontWeight: 500 }}>{CLASE_SINGULAR[opcion.clase]}</span>
                            <span style={{ fontWeight: 400 }}> · {opcion.nombre}</span>
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>

            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button
                type="submit"
                className="boton boton-primario"
                disabled={enviando || faltaElegir}
                title={faltaElegir ? "Elegí a quién corresponde cada gafete repetido" : undefined}
              >
                {enviando ? "Registrando…" : "Registrar salida"}
              </button>
            </div>
          </>
        )}
      </form>
    </Modal>
  );
}
