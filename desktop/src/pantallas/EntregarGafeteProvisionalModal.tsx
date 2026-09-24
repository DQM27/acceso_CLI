import { useEffect, useMemo, useRef, useState } from "react";
import Modal from "../componentes/Modal";
import {
  FilaListaFlotante,
  ListaFlotante,
  SinResultados,
  useListaFlotante,
  useNavegacionFlechas,
} from "../componentes/ListaFlotante";
import { listarEncargadosRutaSeleccionables } from "../api/rutas";
import type { EncargadoRuta } from "../api/rutas";
import {
  entregarGafeteProvisional,
  listarTodosLosGafetesProvisionalesActivos,
} from "../api/gafetesProvisionales";
import { coincideBusqueda, textoGafete, validarNumeroGafete } from "../busqueda";

const MAX_RESULTADOS = 5;

/** Encargados cuyo nombre o código de empleado contienen todas las palabras
 * de `texto` (sin distinguir tildes ni mayúsculas). El catálogo es chico y
 * ya viene completo (`listarEncargadosRutaSeleccionables`), así que se
 * filtra acá en vez de buscar del lado del servidor. */
export function filtrarEncargados(
  encargados: EncargadoRuta[],
  texto: string,
  maximo: number = MAX_RESULTADOS,
): EncargadoRuta[] {
  return encargados
    .filter((encargado) =>
      coincideBusqueda(texto, `${encargado.nombre} ${encargado.codigo_empleado}`),
    )
    .slice(0, maximo);
}

/**
 * Mismo flujo que `NuevoIngresoModal` (pedido del usuario 2026-09-24: el
 * formulario anterior, con un `<datalist>` nativo, se veía mal): buscador
 * arriba con lista flotante; al elegir un encargado se abre su ficha debajo
 * con el número de gafete ya enfocado, y Enter entrega. Encargado sigue
 * siendo el único campo bloqueante (mismo criterio que la versión móvil):
 * no se presta un gafete a texto libre sin un encargado real detrás.
 *
 * Los préstamos activos (locales y del otro dispositivo) se cargan sólo
 * para avisar en la lista y en la ficha si el encargado ya tiene un gafete
 * provisional sin devolver; no bloquean la entrega.
 */
export default function EntregarGafeteProvisionalModal({
  onRegistrado,
  onCerrar,
}: {
  onRegistrado: () => void;
  onCerrar: () => void;
}) {
  const [encargados, setEncargados] = useState<EncargadoRuta[]>([]);
  const [gafetesPrestados, setGafetesPrestados] = useState<Map<string, number[]>>(new Map());
  const [filtro, setFiltro] = useState("");
  const [elegido, setElegido] = useState<EncargadoRuta | null>(null);
  const [gafeteTexto, setGafeteTexto] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);
  const buscadorRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    listarEncargadosRutaSeleccionables()
      .then(setEncargados)
      .catch((error) => setError(String(error)));
    listarTodosLosGafetesProvisionalesActivos()
      .then((activos) => {
        const porCodigo = new Map<string, number[]>();
        for (const prestamo of activos) {
          const lista = porCodigo.get(prestamo.encargado_codigo_empleado) ?? [];
          lista.push(prestamo.gafete_numero);
          porCodigo.set(prestamo.encargado_codigo_empleado, lista);
        }
        setGafetesPrestados(porCodigo);
      })
      // Sólo alimenta un aviso: si falla, se entrega igual sin él.
      .catch(() => {});
  }, []);

  const resultados = useMemo(() => filtrarEncargados(encargados, filtro), [encargados, filtro]);
  const listaVisible = elegido === null && filtro.trim().length > 0;
  const { campoRef, posicion: posicionLista } = useListaFlotante(listaVisible);
  const { resaltado, setResaltado, manejarTecla } = useNavegacionFlechas(
    resultados,
    listaVisible,
    elegirEncargado,
  );

  function cambiarFiltro(texto: string) {
    setFiltro(texto);
    setError(null);
    // Escribir de nuevo abandona al elegido: vuelve a buscar.
    if (elegido) setElegido(null);
  }

  function elegirEncargado(encargado: EncargadoRuta) {
    setError(null);
    setGafeteTexto("");
    setElegido(encargado);
  }

  function cambiarEncargado() {
    setError(null);
    setElegido(null);
    buscadorRef.current?.focus();
  }

  async function entregar() {
    if (!elegido) return;
    const resultado = validarNumeroGafete(gafeteTexto);
    if (!resultado.valido) {
      setError(resultado.mensaje);
      return;
    }
    setError(null);
    setEnviando(true);
    try {
      await entregarGafeteProvisional(elegido.id, resultado.numero);
      onRegistrado();
    } catch (error) {
      setError(String(error));
      setEnviando(false);
    }
  }

  const prestadosElegido = elegido ? (gafetesPrestados.get(elegido.codigo_empleado) ?? []) : [];

  return (
    <Modal titulo="Entregar gafete provisional KOF" onCerrar={onCerrar}>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
        <div ref={campoRef}>
          <label className="campo">
            Buscar encargado
            <input
              ref={buscadorRef}
              value={filtro}
              onChange={(evento) => cambiarFiltro(evento.target.value)}
              onKeyDown={manejarTecla}
              autoFocus
              autoComplete="off"
              placeholder="Nombre o código de empleado…"
            />
          </label>
        </div>

        {listaVisible && posicionLista && (
          <ListaFlotante posicion={posicionLista}>
            {resultados.length === 0 && <SinResultados />}
            {resultados.map((encargado, indice) => {
              const prestados = gafetesPrestados.get(encargado.codigo_empleado) ?? [];
              return (
                <FilaListaFlotante
                  key={encargado.id}
                  resaltada={indice === resaltado}
                  onClick={() => elegirEncargado(encargado)}
                  onMouseEnter={() => setResaltado(indice)}
                >
                  <span>
                    {encargado.nombre}{" "}
                    <span style={{ color: "var(--muted)" }}>· {encargado.codigo_empleado}</span>
                  </span>
                  {prestados.length > 0 && (
                    <span
                      className="chip"
                      style={{
                        ["--chip-color" as string]: "var(--advertencia)",
                        alignSelf: "center",
                        flexShrink: 0,
                        whiteSpace: "nowrap",
                      }}
                    >
                      Tiene {prestados.map(textoGafete).join(", ")}
                    </span>
                  )}
                </FilaListaFlotante>
              );
            })}
          </ListaFlotante>
        )}

        {error && !elegido && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        {elegido && (
          <form
            onSubmit={(evento) => {
              evento.preventDefault();
              entregar();
            }}
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
                <p style={{ margin: 0, fontWeight: 600, color: "var(--texto)" }}>{elegido.nombre}</p>
                <p style={{ margin: "0.15rem 0 0", color: "var(--muted)", fontSize: "0.85rem" }}>
                  Código de empleado {elegido.codigo_empleado}
                </p>
              </div>
              <button
                type="button"
                className="boton"
                style={{ fontSize: "0.8rem" }}
                onClick={cambiarEncargado}
              >
                Cambiar
              </button>
            </div>

            {prestadosElegido.length > 0 && (
              <p style={{ margin: 0, color: "var(--advertencia)", fontSize: "0.85rem" }}>
                ⚠ Ya tiene prestado el gafete {prestadosElegido.map(textoGafete).join(", ")} sin
                devolver
              </p>
            )}

            <label className="campo">
              Número de gafete provisional
              <input
                value={gafeteTexto}
                onChange={(evento) => setGafeteTexto(evento.target.value.replace(/\D/g, ""))}
                inputMode="numeric"
                autoFocus
                autoComplete="off"
                placeholder="Número de gafete"
              />
            </label>

            {error && (
              <p className="login-error" role="alert">
                {error}
              </p>
            )}

            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button type="submit" className="boton boton-primario" disabled={enviando}>
                {enviando ? "Entregando…" : "Entregar gafete"}
              </button>
            </div>
          </form>
        )}
      </div>
    </Modal>
  );
}
