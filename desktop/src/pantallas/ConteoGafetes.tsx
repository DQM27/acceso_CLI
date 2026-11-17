import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CircleAlert, LayoutGrid, ListChecks, SquareDashed } from "lucide-react";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import { catalogoConteoGafetes, comprobarConteoGafetes, leerNumeroConteoGafetes } from "../api";
import type { GafeteParaConteo, ResultadoConteo, TipoGafeteEntrada } from "../api";
import {
  agruparTablero,
  avisoLectura,
  avisoPrevio,
  avisosParaRevisar,
  detalleFicha,
  estadoFicha,
  parsearNumero,
  pasaFiltro,
  textoNumero,
} from "./ConteoGafetes.logica";
import type { AvisoNumero, FiltroTablero, NivelAviso } from "./ConteoGafetes.logica";

const COLOR_AVISO: Record<NivelAviso, string> = {
  bien: "var(--exito)",
  atencion: "var(--advertencia)",
  error: "var(--error)",
};

/**
 * Conteo físico de gafetes (pedido del usuario, 2026-11-16), como vista de la
 * pantalla Gafetes y no como ventana: con muchos gafetes una ventana obligaba
 * a hacer scroll. El operador toma los gafetes en el orden en que estén,
 * escribe cada número y pulsa Enter; el tablero se pinta en vivo: gris lo que
 * falta, verde lo contado, borde punteado lo asignado a alguien adentro,
 * rayado lo perdido y ámbar lo que se contó sin esperarse. "Revisar" aparece
 * sólo si hay algo que mirar (cruces 6/9, asignados o perdidos contados).
 * Las reglas viven en el núcleo (`domain::gafete`); acá sólo se dibuja. Nada
 * se guarda: lo que haya que corregir se hace desde la lista, con su reporte.
 */
export default function ConteoGafetes({ tipo }: { tipo: TipoGafeteEntrada }) {
  const [catalogo, setCatalogo] = useState<GafeteParaConteo[]>([]);
  const [anotados, setAnotados] = useState<number[]>([]);
  const [resultado, setResultado] = useState<ResultadoConteo | null>(null);
  const [texto, setTexto] = useState("");
  const [aviso, setAviso] = useState<AvisoNumero | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [ocupado, setOcupado] = useState(false);
  const [terminado, setTerminado] = useState(false);
  const [filtro, setFiltro] = useState<FiltroTablero>("todos");
  const campoRef = useRef<HTMLInputElement>(null);

  // Se compara en vivo con cada cambio de lo anotado: el resultado del
  // núcleo es la única fuente de faltantes, cruces y avisos.
  const actualizar = useCallback(
    (numeros: number[]) => {
      setAnotados(numeros);
      comprobarConteoGafetes(tipo, numeros)
        .then(setResultado)
        .catch((error) => setError(String(error)));
    },
    [tipo],
  );

  // Un conteo por tipo: la pantalla monta este componente con `key={tipo}`,
  // así que cambiar de tipo empieza otro conteo desde cero.
  useEffect(() => {
    let vigente = true;
    Promise.all([catalogoConteoGafetes(tipo), comprobarConteoGafetes(tipo, [])])
      .then(([datos, inicial]) => {
        if (!vigente) return;
        setCatalogo(datos);
        setResultado(inicial);
      })
      .catch((error) => {
        if (vigente) setError(String(error));
      });
    return () => {
      vigente = false;
    };
  }, [tipo]);

  async function anotar() {
    if (ocupado || terminado) return;
    const previo = avisoPrevio(texto, anotados);
    const numero = parsearNumero(texto);
    setTexto("");
    if (previo || numero === null) {
      setAviso(previo);
      return;
    }
    setOcupado(true);
    setError(null);
    try {
      const nuevo = avisoLectura(numero, await leerNumeroConteoGafetes(tipo, numero));
      setAviso(nuevo);
      if (nuevo.anotar) actualizar([...anotados, numero]);
    } catch (error) {
      setError(String(error));
    } finally {
      setOcupado(false);
      campoRef.current?.focus();
    }
  }

  function quitar(numero: number) {
    if (terminado) return;
    actualizar(anotados.filter((n) => n !== numero));
    setAviso({ nivel: "bien", texto: `${textoNumero(numero)} quitado.`, anotar: false });
    campoRef.current?.focus();
  }

  function empezarDeNuevo() {
    actualizar([]);
    setAviso(null);
    setTerminado(false);
    setFiltro("todos");
    setTimeout(() => campoRef.current?.focus(), 0);
  }

  function terminar() {
    setTerminado(true);
    if (resultado && resultado.faltantes.length > 0) setFiltro("faltan");
  }

  function seguirContando() {
    setTerminado(false);
    setFiltro("todos");
    setTimeout(() => campoRef.current?.focus(), 0);
  }

  const contados = useMemo(() => new Set(anotados), [anotados]);
  const fichas = useMemo(
    () => catalogo.map((gafete) => ({ gafete, estado: estadoFicha(gafete, contados.has(gafete.numero)) })),
    [catalogo, contados],
  );
  const grupos = useMemo(
    () =>
      agruparTablero(
        fichas.filter((ficha) => pasaFiltro(ficha.estado, filtro)).map((ficha) => ficha.gafete),
      ),
    [fichas, filtro],
  );
  const estadoDe = useMemo(
    () => new Map(fichas.map((ficha) => [ficha.gafete.numero, ficha.estado])),
    [fichas],
  );

  const esperados = resultado?.esperados ?? 0;
  const presentes = resultado?.presentes.length ?? 0;
  const faltan = resultado?.faltantes.length ?? 0;
  const avisos = resultado ? avisosParaRevisar(resultado) : [];
  const porcentaje = esperados > 0 ? Math.round((presentes / esperados) * 100) : 0;

  const opcionesFiltro: OpcionSegmentada<FiltroTablero>[] = [
    { valor: "todos", Icono: LayoutGrid, titulo: "Todos", cantidad: catalogo.length },
    { valor: "faltan", Icono: SquareDashed, titulo: "Faltan", cantidad: faltan },
    { valor: "contados", Icono: ListChecks, titulo: "Contados", cantidad: anotados.length },
  ];

  return (
    <div className="conteo" data-terminado={terminado}>
      <div className="conteo-cabecera">
        <form
          style={{ display: "flex", gap: "0.5rem", alignItems: "flex-end", flex: "0 1 22rem" }}
          onSubmit={(evento) => {
            evento.preventDefault();
            anotar();
          }}
        >
          <label className="campo" style={{ flex: 1 }}>
            Número de gafete
            <input
              ref={campoRef}
              value={texto}
              onChange={(evento) => setTexto(evento.target.value.replace(/\D/g, ""))}
              inputMode="numeric"
              autoFocus
              autoComplete="off"
              disabled={terminado}
              placeholder={terminado ? "Conteo terminado" : "Número y Enter…"}
            />
          </label>
        </form>

        <div className="conteo-resumen">
          <div className="conteo-cifras">
            <span>
              Contados <strong>{presentes}</strong> / {esperados}
            </span>
            <span>
              Faltan <strong style={faltan > 0 && terminado ? { color: "var(--error)" } : undefined}>{faltan}</strong>
            </span>
            {avisos.length > 0 && (
              <span>
                Revisar <strong style={{ color: "var(--advertencia)" }}>{avisos.length}</strong>
              </span>
            )}
          </div>
          <div className="conteo-barra" aria-hidden="true">
            <span style={{ width: `${porcentaje}%` }} />
          </div>
        </div>

        <SegmentadoOpciones
          etiqueta="Mostrar en el tablero"
          opciones={opcionesFiltro}
          valor={filtro}
          onCambiar={setFiltro}
        />
      </div>

      <p
        role="status"
        style={{
          margin: 0,
          minHeight: "1.4em",
          fontWeight: 500,
          color: aviso ? COLOR_AVISO[aviso.nivel] : "var(--muted)",
        }}
      >
        {error ??
          aviso?.texto ??
          (terminado
            ? faltan === 0
              ? `Están los ${esperados} gafetes disponibles.`
              : `Faltan ${faltan} de los ${esperados} gafetes disponibles: en rojo en el tablero.`
            : "Tome los gafetes uno por uno, en el orden en que estén: escriba el número, pulse Enter y póngalo aparte.")}
      </p>

      <div className="conteo-cuerpo">
        <div className="conteo-tablero tarjeta">
          {grupos.every((grupo) => grupo.gafetes.length === 0) ? (
            <p style={{ margin: 0, color: "var(--muted)" }}>
              {catalogo.length === 0 ? "No hay gafetes de este tipo." : "Nada que mostrar con este filtro."}
            </p>
          ) : (
            grupos.map((grupo) => (
              <section key={grupo.titulo ?? "todos"} className="conteo-grupo">
                {grupo.titulo && <p className="conteo-grupo-titulo">{grupo.titulo}</p>}
                <div className="conteo-fichas">
                  {grupo.gafetes.map((gafete) => {
                    const estado = estadoDe.get(gafete.numero) ?? "pendiente";
                    const quitable = !terminado && (estado === "contado" || estado === "revisar");
                    return (
                      <button
                        key={gafete.numero}
                        type="button"
                        className="conteo-ficha"
                        data-estado={estado}
                        title={detalleFicha(gafete, estado)}
                        aria-label={detalleFicha(gafete, estado)}
                        disabled={!quitable}
                        onClick={() => quitar(gafete.numero)}
                      >
                        {textoNumero(gafete.numero)}
                      </button>
                    );
                  })}
                </div>
              </section>
            ))
          )}
        </div>

        {avisos.length > 0 && (
          <aside className="conteo-revisar tarjeta" aria-label="Revisar">
            <strong style={{ display: "flex", alignItems: "center", gap: "0.4rem", color: "var(--advertencia)" }}>
              <CircleAlert size={16} aria-hidden="true" />
              Revisar
            </strong>
            {avisos.map((texto) => (
              <p key={texto}>{texto}</p>
            ))}
          </aside>
        )}
      </div>

      <div className="conteo-pie">
        <div className="conteo-leyenda" aria-hidden="true">
          <span>
            <i className="conteo-ficha" data-estado="pendiente" /> Falta contarlo
          </span>
          <span>
            <i className="conteo-ficha" data-estado="contado" /> Contado
          </span>
          <span>
            <i className="conteo-ficha" data-estado="asignado" /> Asignado (adentro)
          </span>
          <span>
            <i className="conteo-ficha" data-estado="perdido" /> Perdido
          </span>
          <span>
            <i className="conteo-ficha" data-estado="revisar" /> Revisar
          </span>
        </div>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <button type="button" className="boton" onClick={empezarDeNuevo} disabled={ocupado}>
            Empezar de nuevo
          </button>
          {terminado ? (
            <button type="button" className="boton boton-primario" onClick={seguirContando}>
              Seguir contando
            </button>
          ) : (
            <button
              type="button"
              className="boton boton-primario"
              onClick={terminar}
              disabled={ocupado || catalogo.length === 0}
            >
              Terminar conteo
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
