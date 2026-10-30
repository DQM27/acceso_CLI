import { useMemo, useState } from "react";
import { toast } from "sonner";
import { Building2, CircleCheck, Merge } from "lucide-react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import { useLista } from "../componentes/useLista";
import {
  confirmarEmpresaProveedor,
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
          estadoRevision(data) === "fusionada" ? null : (
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
          origen={fusionando}
          empresas={empresas}
          onCerrar={() => setFusionando(null)}
          onFusionada={() => {
            setFusionando(null);
            void recargar();
          }}
        />
      )}
    </div>
  );
}

function ModalFusion({
  origen,
  empresas,
  onCerrar,
  onFusionada,
}: {
  origen: EmpresaProveedorPanel;
  empresas: readonly EmpresaProveedorPanel[];
  onCerrar: () => void;
  onFusionada: () => void;
}) {
  const [filtro, setFiltro] = useState("");
  const [destino, setDestino] = useState<EmpresaProveedorPanel | null>(null);
  const [fusionando, setFusionando] = useState(false);

  const posibles = useMemo(() => {
    const texto = plegarTexto(filtro.trim());
    return destinosPosibles(origen, empresas).filter(
      ({ empresa }) => !texto || plegarTexto(empresa.nombre).includes(texto),
    );
  }, [origen, empresas, filtro]);

  async function fusionar() {
    if (!destino) return;
    setFusionando(true);
    try {
      await fusionarEmpresaProveedor(origen.id, destino.id);
      toast.success(`${origen.nombre} quedó fusionada en ${destino.nombre}.`);
      onFusionada();
    } catch (error) {
      toast.error(mensajeError(error));
      setFusionando(false);
    }
  }

  return (
    <Modal titulo={`Fusionar «${origen.nombre}»`} onCerrar={onCerrar}>
      <div className="flex flex-col gap-3">
        <p className="m-0">
          Elija la empresa que es la misma (bien escrita). «{origen.nombre}»
          deja de ofrecerse en la portería y sus {origen.ingresos} ingreso(s)
          pasan a la que elija.
        </p>
        <div className="campo">
          <input
            placeholder="Buscar empresa…"
            value={filtro}
            onChange={(evento) => setFiltro(evento.target.value)}
          />
        </div>
        <ul
          className="m-0 flex max-h-72 list-none flex-col gap-1 overflow-y-auto p-0"
          aria-label="Empresas posibles"
        >
          {posibles.map(({ empresa, parecida }) => (
            <li key={empresa.id}>
              <label className="flex cursor-pointer items-center gap-2">
                <input
                  type="radio"
                  name="destino"
                  checked={destino?.id === empresa.id}
                  onChange={() => setDestino(empresa)}
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
                <span className="text-[var(--muted)]">
                  · {empresa.ingresos} ingreso(s)
                </span>
              </label>
            </li>
          ))}
          {posibles.length === 0 && (
            <li className="text-[var(--muted)]">
              No hay otra empresa en esta unidad.
            </li>
          )}
        </ul>
        <div className="flex justify-end gap-2">
          <button type="button" className="boton" onClick={onCerrar}>
            Cancelar
          </button>
          <button
            type="button"
            className="boton boton-primario"
            disabled={!destino || fusionando}
            onClick={() => void fusionar()}
          >
            {fusionando
              ? "Fusionando…"
              : destino
                ? `Fusionar en ${destino.nombre}`
                : "Elija una empresa"}
          </button>
        </div>
      </div>
    </Modal>
  );
}
