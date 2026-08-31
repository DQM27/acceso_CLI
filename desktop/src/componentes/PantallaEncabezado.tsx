/**
 * Encabezado uniforme de pantalla — sólo el título. Las acciones de cada
 * pantalla ("+ Nuevo…", buscador, etc.) van sobre la grilla (`controles`/
 * `accionesDerecha` de `Tabla`), no acá — ver Ingresos activos como
 * referencia del patrón que siguen las demás pantallas.
 */
export default function PantallaEncabezado({ titulo }: { titulo: string }) {
  return (
    <div className="pantalla-encabezado">
      <h1>{titulo}</h1>
    </div>
  );
}
