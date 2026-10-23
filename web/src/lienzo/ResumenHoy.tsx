import type { CSSProperties } from "react";
import type { ResumenHoy } from "../api/lienzo";
import { etiquetaFecha, type DatosGrafico } from "./calculo";
import { COLOR_UNICO } from "./colores";
import GraficoBarras from "./graficos/Barras";
import { MarcoGrafico } from "./graficos/comun";
import { TarjetaKpi } from "./widgets";

function Atencion({
  tono,
  hecho,
  texto,
  aviso,
}: {
  tono: "error" | "advertencia" | "exito";
  hecho: string;
  texto: string;
  aviso: string;
}) {
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

const panel = (i: number) => ({ "--i": i }) as CSSProperties;

/** Pantalla inicial del lienzo (propuesta B): los números de hoy, en vivo. */
export default function ResumenDeHoy({ resumen }: { resumen: ResumenHoy }) {
  const datos: DatosGrafico = {
    categorias: resumen.ingresos_por_dia.map((d) => d.dia),
    series: [{ nombre: "Ingresos", valores: resumen.ingresos_por_dia.map((d) => d.cantidad) }],
    agrupacion: "dia",
  };
  const ultima = datos.categorias.length - 1;
  const etiquetar = (clave: string, i: number) => `${etiquetaFecha(clave, "dia")}${i === ultima ? " · hoy" : ""}`;

  const vencidos = resumen.praind_vencidos;
  const porVencer = resumen.praind_por_vencer;
  const plural = (n: number, uno: string, varios: string) => (n === 1 ? uno : varios);

  return (
    <div className="lienzo-documento lienzo-cuadricula">
      <div className="lienzo-panel lienzo-ancho-cuarto" style={panel(0)}>
        <TarjetaKpi
          valor={resumen.adentro_contratistas}
          titulo="Contratistas adentro ahora"
          detalle={`${resumen.adentro_proveedores} ${plural(resumen.adentro_proveedores, "proveedor", "proveedores")}`}
        />
      </div>
      <div className="lienzo-panel lienzo-ancho-cuarto" style={panel(1)}>
        <TarjetaKpi valor={resumen.ingresos_hoy} titulo="Ingresos hoy" detalle="Día en curso" />
      </div>
      <div className="lienzo-panel lienzo-ancho-cuarto" style={panel(2)}>
        <TarjetaKpi valor={vencidos} titulo="PRAIND vencidos" detalle="No pueden entrar" tono={vencidos > 0 ? "error" : "neutro"} />
      </div>
      <div className="lienzo-panel lienzo-ancho-cuarto" style={panel(3)}>
        <TarjetaKpi
          valor={porVencer}
          titulo="PRAIND por vencer"
          detalle="En los próximos 30 días"
          tono={porVencer > 0 ? "advertencia" : "neutro"}
        />
      </div>
      <div className="lienzo-panel lienzo-ancho-dos_tercios" style={panel(4)}>
        <MarcoGrafico
          titulo="Ingresos por día"
          subtitulo="últimas 2 semanas"
          datos={{
            columnas: ["Día", "Ingresos"],
            filas: resumen.ingresos_por_dia.map((d) => [d.dia.split("-").reverse().join("/"), d.cantidad]),
          }}
        >
          <GraficoBarras
            datos={datos}
            colores={[COLOR_UNICO]}
            titulo="Ingresos por día, últimas 2 semanas"
            alto={260}
            resaltarUltima
            etiquetar={etiquetar}
          />
        </MarcoGrafico>
      </div>
      <div className="lienzo-panel lienzo-ancho-tercio" style={panel(5)}>
        <section className="tarjeta lienzo-bloque">
          <header className="lienzo-bloque-cabecera">
            <h3>Requiere atención</h3>
            <span className="lienzo-subtitulo">{(vencidos > 0 ? 1 : 0) + (porVencer > 0 ? 1 : 0)}</span>
          </header>
          {vencidos === 0 && porVencer === 0 && (
            <Atencion tono="exito" hecho="Nada" texto="pendiente por PRAIND" aviso="Ningún PRAIND vencido ni por vencer" />
          )}
          {vencidos > 0 && (
            <Atencion
              tono="error"
              hecho={`${vencidos} ${plural(vencidos, "contratista", "contratistas")}`}
              texto="con PRAIND vencido"
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
