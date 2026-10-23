import type { ResumenHoy } from "../api/lienzo";
import type { Barra } from "./calculo";
import { GraficoBarras, TarjetaKpi } from "./widgets";

/** "09" de "2026-10-09": la etiqueta corta de cada día del gráfico. */
function etiquetaDia(ymd: string): string {
  return ymd.slice(8, 10);
}

function Atencion({ tono, hecho, texto, aviso }: { tono: "error" | "advertencia" | "exito"; hecho: string; texto: string; aviso: string }) {
  return (
    <div className="lienzo-atencion-fila">
      <span className={`lienzo-punto lienzo-punto-${tono}`} aria-hidden="true" />
      <span>
        <b>{hecho}</b> {texto}
        <br />
        <span className="lienzo-atenuado">{aviso}</span>
      </span>
    </div>
  );
}

/** Pantalla inicial del lienzo (propuesta B): los números de hoy, en vivo. */
export default function ResumenDeHoy({ resumen }: { resumen: ResumenHoy }) {
  const barras: Barra[] = resumen.ingresos_por_dia.map((d) => ({ etiqueta: etiquetaDia(d.dia), valor: d.cantidad }));
  const hoy = barras.length - 1;
  if (hoy >= 0) barras[hoy] = { ...barras[hoy], etiqueta: `${barras[hoy].etiqueta}·hoy` };

  const vencidos = resumen.praind_vencidos;
  const porVencer = resumen.praind_por_vencer;
  const plural = (n: number, uno: string, varios: string) => (n === 1 ? uno : varios);

  return (
    <div className="lienzo-documento">
      <div className="lienzo-kpis">
        <TarjetaKpi
          valor={resumen.adentro_contratistas}
          titulo="Contratistas adentro ahora"
          detalle={`${resumen.adentro_proveedores} ${plural(resumen.adentro_proveedores, "proveedor", "proveedores")}`}
        />
        <TarjetaKpi valor={resumen.ingresos_hoy} titulo="Ingresos hoy" detalle="Día en curso" />
        <TarjetaKpi valor={vencidos} titulo="PRAIND vencidos" detalle="No pueden entrar" tono={vencidos > 0 ? "error" : "neutro"} />
        <TarjetaKpi
          valor={porVencer}
          titulo="PRAIND por vencer"
          detalle="En los próximos 30 días"
          tono={porVencer > 0 ? "advertencia" : "neutro"}
        />
      </div>
      <div className="lienzo-abajo">
        <section className="tarjeta lienzo-bloque">
          <h3>
            Ingresos por día <span>últimas 2 semanas</span>
          </h3>
          <GraficoBarras barras={barras} resaltarUltima titulo="Ingresos por día, últimas 2 semanas" />
        </section>
        <section className="tarjeta lienzo-bloque">
          <h3>
            Requiere atención <span>{(vencidos > 0 ? 1 : 0) + (porVencer > 0 ? 1 : 0)}</span>
          </h3>
          {vencidos === 0 && porVencer === 0 && (
            <Atencion tono="exito" hecho="Nada" texto="pendiente por PRAIND" aviso="Ningún PRAIND vencido ni por vencer" />
          )}
          {vencidos > 0 && (
            <Atencion
              tono="error"
              hecho={`${vencidos} ${plural(vencidos, "contratista", "contratistas")}`}
              texto={`con PRAIND vencido`}
              aviso="Revisar antes de su próximo ingreso"
            />
          )}
          {porVencer > 0 && (
            <Atencion
              tono="advertencia"
              hecho={`${porVencer} PRAIND`}
              texto={plural(porVencer, "vence en 30 días", "vencen en 30 días")}
              aviso="Avisar a sus empresas"
            />
          )}
        </section>
      </div>
    </div>
  );
}
