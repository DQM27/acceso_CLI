import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { encode } from "uqr";
import { contenidoQr, tiempoRestante } from "./CodigoVinculacion.logica";

/** QR dibujado como SVG a partir de la matriz, sin `innerHTML`. */
function CodigoQr({ contenido, tamano = 200 }: { contenido: string; tamano?: number }) {
  const { size, data } = useMemo(() => encode(contenido, { border: 2 }), [contenido]);
  const modulos = useMemo(() => {
    const rectangulos: string[] = [];
    data.forEach((fila, y) =>
      fila.forEach((oscuro, x) => {
        if (oscuro) rectangulos.push(`M${x} ${y}h1v1h-1z`);
      }),
    );
    return rectangulos.join("");
  }, [data]);
  return (
    <svg
      role="img"
      aria-label="Código QR de vinculación"
      width={tamano}
      height={tamano}
      viewBox={`0 0 ${size} ${size}`}
      shapeRendering="crispEdges"
      className="rounded-lg bg-white"
    >
      <path d={modulos} fill="#000" />
    </svg>
  );
}

/**
 * Código de vinculación recién emitido: se muestra UNA sola vez, acá. En el
 * celular se escanea el QR; en una PC se escribe el código. Vence y sirve
 * una sola vez, así que no es un secreto que haya que proteger después:
 * aun así conviene no mandarlo por chat.
 */
export default function CodigoVinculacionEmitido({
  codigo,
  expiraEn,
  onListo,
}: {
  codigo: string;
  expiraEn: string;
  onListo: () => void;
}) {
  const [ahora, setAhora] = useState(() => Date.now());
  useEffect(() => {
    const intervalo = window.setInterval(() => setAhora(Date.now()), 1000);
    return () => window.clearInterval(intervalo);
  }, []);
  const restante = tiempoRestante(expiraEn, ahora);

  async function copiar() {
    try {
      await navigator.clipboard.writeText(codigo);
      toast.success("Código copiado.");
    } catch {
      toast.error("No se pudo copiar -- seleccione el texto a mano.");
    }
  }

  return (
    <div className="flex flex-col items-center gap-4">
      <p className="m-0 text-center">
        En el celular, escanee este QR desde la pantalla de inicio de la app. En una PC, escriba el
        código. Sirve una sola vez y no se vuelve a mostrar.
      </p>
      <CodigoQr contenido={contenidoQr(codigo)} />
      <div className="flex items-center gap-2">
        <code className="text-[1.6rem] font-semibold tracking-[0.15em]">{codigo}</code>
        <button type="button" className="boton" onClick={copiar}>
          Copiar
        </button>
      </div>
      <p className={`m-0 ${restante ? "text-muted" : "text-error"}`} role="status">
        {restante ? `Vence en ${restante}` : "Este código ya venció: elimine este dispositivo y regístrelo de nuevo."}
      </p>
      <div className="flex justify-end self-stretch">
        <button type="button" className="boton boton-primario" onClick={onListo}>
          Listo
        </button>
      </div>
    </div>
  );
}
