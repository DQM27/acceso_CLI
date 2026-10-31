import { useMemo, useState } from "react";
import { toast } from "sonner";
import { ArrowLeftRight, Building2, CircleCheck, Merge } from "lucide-react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import { useLista } from "../componentes/useLista";
import {
  confirmarEmpresaProveedor,
  deshacerFusionEmpresaProveedor,
  fusionarEmpresaProveedor,
  listarEmpresasProveedor,
} from "../api/empresasProveedor";
import type { EmpresaProveedorPanel } from "../api/empresasProveedor";
import { fechaLocalYMD, textoFechaDDMMYYYY } from "../tiempo";
import { mensajeError } from "../mensajeError";
import { plegarTexto } from "../api/historialFiltros";
import {
  destinosPosibles,
  estadoRevision,
  filtrarPorRevision,
  ladosFusion,
} from "./EmpresasProveedor.logica";
import type { FiltroRevision } from "./EmpresasProveedor.logica";

/**
 * Empresas de proveedor de todas las unidades, para mantener el catálogo en
 * orden. La portería puede crear empresas (con buscador que sugiere las
 * parecidas), y lo que crea queda "por revisar": el administrador lo
 * confirma (es nueva de verdad) o lo fusiona con la que ya existía (era la
 * misma, mal escrita: "FUDDY FACTORY" en "FOODIE FACTORY"). Al fusionar, sus
 * ingresos pasan a la que queda y los equipos dejan de ofrecerla.
 */
export default function EmpresasProveedor() {
  const [filtro, setFiltro] = useState<FiltroRevision>("por_revisar");
  const [busqueda, setBusqueda] = useState("");
  const [fusionando, setFusionando] = useState<EmpresaProveedorPanel | null>(
    null,
  );
  const [deshaciendo, setDeshaciendo] = useState<EmpresaProveedorPanel | null>(
    null,
  );
  const [ocupada, setOcupada] = useState<string | null>(null);

  const { datos, cargando, recargar } = useLista(
    ["empresas-proveedor"],
    listarEmpresasProveedor,
    {
      intervaloMs: 120_000,
      tablas: "empresas_proveedor,ingresos_proveedor",
    },
  );
  const empresas = useMemo(() => datos ?? [], [datos]);
  const filas = useMemo(
    () => filtrarPorRevision(empresas, filtro),
    [empresas, filtro],
  );

  const conteo = useMemo(
    () => ({
      por_revisar: filtrarPorRevision(empresas, "por_revisar").length,
      vigentes: filtrarPorRevision(empresas, "vigentes").length,
      fusionadas: filtrarPorRevision(empresas, "fusionadas").length,
    }),
    [empresas],
  );
  const opciones: OpcionSegmentada<FiltroRevision>[] = [
    {
      valor: "por_revisar",
      Icono: Building2,
      titulo: `${conteo.por_revisar} por revisar`,
      cantidad: conteo.por_revisar,
    },
    {
      valor: "vigentes",
      Icono: CircleCheck,
      titulo: `${conteo.vigentes} vigentes`,
      cantidad: conteo.vigentes,
    },
    {
      valor: "fusionadas",
      Icono: Merge,
      titulo: `${conteo.fusionadas} fusionadas`,
      cantidad: conteo.fusionadas,
    },
  ];

  async function confirmar(empresa: EmpresaProveedorPanel) {
    setOcupada(empresa.id);
    try {
      await confirmarEmpresaProveedor(empresa.id);
      toast.success(`${empresa.nombre} confirmada.`);
      await recargar();
    } catch (error) {
      toast.error(mensajeError(error));
    } finally {
      setOcupada(null);
    }
  }

  const columnas = useMemo<ColDef<EmpresaProveedorPanel>[]>(
    () => [
      {
        field: "nombre",
        headerName: "Empresa",
        flex: 1.8,
        minWidth: 180,
        cellStyle: { textAlign: "left" },
      },
      { field: "sitio_nombre", headerName: "Unidad", flex: 1, minWidth: 120 },
      {
        colId: "estado",
        headerName: "Estado",
        flex: 1.6,
        minWidth: 170,
        valueGetter: (p) => {
          if (!p.data) return "";
          const estado = estadoRevision(p.data);
          if (estado === "fusionada")
            return `Fusionada en ${p.data.fusionada_en_nombre ?? "—"}`;
          return estado === "revisada" ? "Revisada" : "Por revisar";
        },
        cellStyle: (p) =>
          p.data && estadoRevision(p.data) === "por_revisar"
            ? { color: "var(--advertencia)", fontWeight: 600 }
            : null,
      },
      {
        field: "ingresos",
        headerName: "Ingresos",
        flex: 0.8,
        minWidth: 100,
        filter: "agNumberColumnFilter",
      },
      {
        field: "ultimo_ingreso",
        headerName: "Último ingreso",
        flex: 1,
        minWidth: 130,
        valueFormatter: (p) =>
          p.value ? textoFechaDDMMYYYY(fechaLocalYMD(p.value as string)) : "—",
      },
      {
        field: "creada_por_equipo",
        headerName: "Creada en",
        flex: 1.1,
        minWidth: 140,
      },
      {
        colId: "acciones",
        headerName: "",
        flex: 1.6,
        minWidth: 210,
        filter: false,
        sortable: false,
        cellRenderer: ({ data }: { data: EmpresaProveedorPanel }) =>
          estadoRevision(data) === "fusionada" ? (
            <button
              type="button"
              className="boton boton-celda-angosto"
              onClick={() => setDeshaciendo(data)}
            >
              Deshacer fusión
            </button>
          ) : (
            <div className="flex gap-2">
              {estadoRevision(data) === "por_revisar" && (
                <button
                  type="button"
                  className="boton boton-celda-angosto"
                  disabled={ocupada === data.id}
                  onClick={() => void confirmar(data)}
                >
                  Confirmar
                </button>
              )}
              <button
                type="button"
                className="boton boton-celda-angosto"
                onClick={() => setFusionando(data)}
              >
                Fusionar…
              </button>
            </div>
          ),
      },
    ],
    // `confirmar` cambia en cada render; lo que importa para dibujar es `ocupada`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [ocupada],
  );

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        <div className="min-h-0 flex-1">
          <Tabla<EmpresaProveedorPanel>
            id="empresas-proveedor"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            filtrosPorColumna
            controles={
              <>
                <SegmentadoOpciones
                  opciones={opciones}
                  valor={filtro}
                  onCambiar={setFiltro}
                  etiqueta="Filtrar por revisión"
                />
                <div className="campo flex-[0_1_16rem]">
                  <input
                    placeholder="Empresa…"
                    value={busqueda}
                    disabled={cargando}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
          />
        </div>
      </div>

      {fusionando && (
        <ModalFusion
          fila={fusionando}
          empresas={empresas}
          onCerrar={() => setFusionando(null)}
          onFusionada={() => {
            setFusionando(null);
            void recargar();
          }}
        />
      )}
      {deshaciendo && (
        <ModalDeshacer
          empresa={deshaciendo}
          onCerrar={() => setDeshaciendo(null)}
          onDeshecha={() => {
            setDeshaciendo(null);
            void recargar();
          }}
        />
      )}
    </div>
  );
}

function ModalFusion({
  fila,
  empresas,
  onCerrar,
  onFusionada,
}: {
  fila: EmpresaProveedorPanel;
  empresas: readonly EmpresaProveedorPanel[];
  onCerrar: () => void;
  onFusionada: () => void;
}) {
  const [filtro, setFiltro] = useState("");
  const [elegida, setElegida] = useState<EmpresaProveedorPanel | null>(null);
  const [invertido, setInvertido] = useState(false);
  const [fusionando, setFusionando] = useState(false);

  const posibles = useMemo(() => {
    const texto = plegarTexto(filtro.trim());
    return destinosPosibles(fila, empresas).filter(
      ({ empresa }) => !texto || plegarTexto(empresa.nombre).includes(texto),
    );
  }, [fila, empresas, filtro]);

  const lados = elegida ? ladosFusion(fila, elegida, invertido) : null;

  async function fusionar() {
    if (!lados) return;
    setFusionando(true);
    try {
      await fusionarEmpresaProveedor(lados.desaparece.id, lados.queda.id);
      toast.success(
        `${lados.desaparece.nombre} quedó fusionada en ${lados.queda.nombre}.`,
      );
      onFusionada();
    } catch (error) {
      toast.error(mensajeError(error));
      setFusionando(false);
    }
  }

  return (
    <Modal titulo={`Fusionar con «${fila.nombre}»`} onCerrar={onCerrar}>
      <div className="flex flex-col gap-3">
        <p className="m-0">
          Elija la empresa que es la misma. La que desaparece deja de ofrecerse
          en la portería y sus ingresos pasan a la que queda.
        </p>
        <div className="campo">
          <input
            placeholder="Buscar empresa…"
            value={filtro}
            onChange={(evento) => setFiltro(evento.target.value)}
          />
        </div>
        <ul
          className="m-0 flex max-h-60 list-none flex-col gap-1 overflow-y-auto p-0"
          aria-label="Empresas posibles"
        >
          {posibles.map(({ empresa, parecida }) => (
            <li key={empresa.id}>
              <label className="flex cursor-pointer items-center gap-2">
                <input
                  type="radio"
                  name="elegida"
                  checked={elegida?.id === empresa.id}
                  onChange={() => setElegida(empresa)}
                />
                <span>{empresa.nombre}</span>
                {parecida && (
                  <span
                    className="chip"
                    style={{ ["--chip-color" as string]: "var(--advertencia)" }}
                  >
                    Parecida
                  </span>
                )}
                <span className="text-(--muted)">
                  · {empresa.ingresos} ingreso(s)
                </span>
              </label>
            </li>
          ))}
          {posibles.length === 0 && (
            <li className="text-(--muted)">
              No hay otra empresa en esta unidad.
            </li>
          )}
        </ul>
        {lados && (
          <div
            className="flex items-center gap-3 rounded-md border border-(--borde) p-3"
            aria-label="Resultado de la fusión"
          >
            <div className="flex min-w-0 flex-1 flex-col gap-1">
              <div>
                <span className="font-semibold text-(--exito)">Queda: </span>
                <strong>{lados.queda.nombre}</strong>
              </div>
              <div>
                <span className="font-semibold text-(--error)">
                  Desaparece:{" "}
                </span>
                <span className="line-through">{lados.desaparece.nombre}</span>
                <span className="text-(--muted)">
                  {" "}
                  · sus {lados.desaparece.ingresos} ingreso(s) pasan a{" "}
                  {lados.queda.nombre}
                </span>
              </div>
            </div>
            <button
              type="button"
              className="boton"
              onClick={() => setInvertido((valor) => !valor)}
              title="Dar vuelta: queda la otra"
            >
              <ArrowLeftRight size={16} aria-hidden /> Invertir
            </button>
          </div>
        )}
        <div className="flex justify-end gap-2">
          <button type="button" className="boton" onClick={onCerrar}>
            Cancelar
          </button>
          <button
            type="button"
            className="boton boton-primario"
            disabled={!lados || fusionando}
            onClick={() => void fusionar()}
          >
            {fusionando
              ? "Fusionando…"
              : lados
                ? `Fusionar: queda ${lados.queda.nombre}`
                : "Elija una empresa"}
          </button>
        </div>
      </div>
    </Modal>
  );
}

function ModalDeshacer({
  empresa,
  onCerrar,
  onDeshecha,
}: {
  empresa: EmpresaProveedorPanel;
  onCerrar: () => void;
  onDeshecha: () => void;
}) {
  const [deshaciendo, setDeshaciendo] = useState(false);
  const otra = empresa.fusionada_en_nombre ?? "la otra";

  async function deshacer() {
    setDeshaciendo(true);
    try {
      await deshacerFusionEmpresaProveedor(empresa.id);
      toast.success(`${empresa.nombre} vuelve a estar vigente.`);
      onDeshecha();
    } catch (error) {
      toast.error(mensajeError(error));
      setDeshaciendo(false);
    }
  }

  return (
    <Modal
      titulo={`Deshacer la fusión de «${empresa.nombre}»`}
      onCerrar={onCerrar}
    >
      <div className="flex flex-col gap-3">
        <p className="m-0">
          «{empresa.nombre}» vuelve a ofrecerse en la portería, queda por
          revisar y recupera sus ingresos, que hoy están en «{otra}».
        </p>
        <p className="m-0 text-(--muted)">
          Las fusiones hechas antes de esta opción no guardaron qué ingresos
          eran suyos: esos se quedan en «{otra}». Si se fusionó al revés,
          después fusione en el sentido correcto y quedan todos juntos.
        </p>
        <div className="flex justify-end gap-2">
          <button type="button" className="boton" onClick={onCerrar}>
            Cancelar
          </button>
          <button
            type="button"
            className="boton boton-primario"
            disabled={deshaciendo}
            onClick={() => void deshacer()}
          >
            {deshaciendo ? "Deshaciendo…" : "Deshacer fusión"}
          </button>
        </div>
      </div>
    </Modal>
  );
}
