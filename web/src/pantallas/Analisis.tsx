import { Suspense, lazy, useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import SelectorUnidadesOperativas from "../componentes/SelectorUnidadesOperativas";
import { listarUnidadesOperativas } from "../api/historial";
import {
  ingresosPorHora,
  obtenerResumenMovimientos,
  problemaDelRango,
  tendenciaDiaria,
  textoPermanencia,
} from "../api/analisis";
import { mensajeError } from "../mensajeError";
import { fechaHaceMeses, fechaYMD } from "../tiempo";
import { GraficoEmpresas, GraficoHoras, GraficoTendencia } from "./analisis/Graficos";

// La tabla dinámica arrastra la grilla completa de Syncfusion (el bloque más
// pesado): va aparte, así indicadores y gráficos aparecen sin esperarla.
const TablaDinamica = lazy(() => import("./analisis/TablaDinamica"));

/**
 * Análisis del historial de TODAS las unidades: indicadores, tendencia por
 * día, horas pico, empresas con más ingresos y una tabla dinámica para cruzar
 * lo que haga falta. Los números los agrega la base
 * (`panel_resumen_movimientos`): el navegador recibe un resumen de pocos miles
 * de filas aunque el historial tenga millones. Gráficos y tabla dinámica son
 * de Syncfusion y se descargan sólo al entrar a esta sección.
 */
export default function Analisis() {
  // Tres meses por defecto: suficiente para ver tendencia y horas pico, y la
  // consulta sigue siendo liviana. Hasta un año por período.
  const [desde, setDesde] = useState(() => fechaHaceMeses(3));
  const [hasta, setHasta] = useState("");
  // Mismo criterio que Historial: se guarda lo DESMARCADO; vacío = todas.
  const [unidadesExcluidas, setUnidadesExcluidas] = useState<Set<string>>(new Set());
  const hoy = fechaYMD(new Date());
  const hastaEfectivo = hasta || hoy;

  const { data: unidades = [], error: errorUnidades } = useQuery({
    queryKey: ["historial", "unidades"],
    queryFn: listarUnidadesOperativas,
    staleTime: 5 * 60_000,
  });

  const sitioIds =
    unidadesExcluidas.size === 0
      ? undefined
      : unidades.filter((u) => !unidadesExcluidas.has(u.id)).map((u) => u.id);

  const problema = problemaDelRango(desde, hastaEfectivo);
  const {
    data: resumen,
    error: errorResumen,
    isFetching,
  } = useQuery({
    queryKey: ["analisis", "resumen", desde, hastaEfectivo, sitioIds ?? null],
    queryFn: () => obtenerResumenMovimientos({ desde, hasta: hastaEfectivo, sitioIds }, hoy),
    enabled: problema === null,
    staleTime: 60_000,
    placeholderData: (anterior) => anterior,
  });

  useEffect(() => {
    const error = errorUnidades ?? errorResumen;
    if (error) toast.error(mensajeError(error));
  }, [errorUnidades, errorResumen]);

  const tendencia = useMemo(
    () => (resumen ? tendenciaDiaria(resumen.diario, desde, hastaEfectivo) : []),
    [resumen, desde, hastaEfectivo],
  );
  const horas = useMemo(
    () => (resumen ? ingresosPorHora(resumen.por_hora, desde, hastaEfectivo) : []),
    [resumen, desde, hastaEfectivo],
  );

  const total = resumen?.total;
  const dias = tendencia.length || 1;

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1 gap-[0.375rem] overflow-y-auto">
        <div className="flex flex-wrap items-end justify-between gap-[0.375rem]">
          <span role="status" className="text-(--muted)">
            {problema ?? (isFetching ? "Calculando…" : "")}
          </span>
          <div className="flex items-center gap-[0.375rem]">
            <SelectorUnidadesOperativas
              unidades={unidades}
              excluidas={unidadesExcluidas}
              onCambiar={setUnidadesExcluidas}
            />
            <SelectorRangoFecha
              desde={desde}
              hasta={hasta}
              onAplicar={(nuevoDesde, nuevoHasta) => {
                setDesde(nuevoDesde);
                setHasta(nuevoHasta);
              }}
            />
          </div>
        </div>

        {resumen && total && (
          <>
            <div className="grid grid-cols-2 gap-[0.375rem] lg:grid-cols-4">
              <Indicador titulo="Ingresos" valor={total.ingresos.toLocaleString("es-CR")} />
              <Indicador titulo="Personas distintas" valor={total.personas.toLocaleString("es-CR")} />
              <Indicador
                titulo="Ingresos por día"
                valor={(Math.round((total.ingresos / dias) * 10) / 10).toLocaleString("es-CR")}
              />
              <Indicador
                titulo="Permanencia promedio"
                valor={textoPermanencia(total.minutos_adentro, total.con_salida)}
                detalle={`${total.con_salida.toLocaleString("es-CR")} con salida registrada`}
              />
            </div>

            <GraficoTendencia puntos={tendencia} />
            <div className="grid gap-[0.375rem] lg:grid-cols-2">
              <GraficoHoras horas={horas} />
              <GraficoEmpresas empresas={resumen.empresas} />
            </div>
            <Suspense
              fallback={
                <div className="tarjeta p-3 text-(--muted)" role="status">
                  Cargando tabla dinámica…
                </div>
              }
            >
              <TablaDinamica diario={resumen.diario} />
            </Suspense>
          </>
        )}
      </div>
    </div>
  );
}

function Indicador({ titulo, valor, detalle }: { titulo: string; valor: string; detalle?: string }) {
  return (
    <div className="tarjeta flex flex-col gap-1 p-3">
      <span className="text-xs text-(--muted)">{titulo}</span>
      <span className="text-2xl font-semibold text-(--texto)">{valor}</span>
      {detalle && <span className="text-xs text-(--muted)">{detalle}</span>}
    </div>
  );
}
