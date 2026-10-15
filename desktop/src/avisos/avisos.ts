import { toast } from "sonner";
import { duracionDe, textoDeError, tituloPorDefecto } from "./avisos.logica";
import type { TipoAviso } from "./avisos.logica";

/**
 * La puerta única de los avisos de la app. Ninguna pantalla importa `sonner`
 * directo (lo vigila `avisos.arquitectura.test.ts`): llaman acá, y acá se
 * decide cómo se ven (título, duración, ícono) según el tipo. El motor
 * (`sonner`) se puede cambiar sin tocar a nadie.
 *
 * Regla: este módulo NO decide qué avisar ni con qué palabras. Quien llama
 * (o el núcleo, que ya redacta los mensajes) elige el tipo y el texto.
 */

export interface AccionAviso {
  etiqueta: string;
  alAccionar: () => void;
}

export interface OpcionesAviso {
  /** Reemplaza el título por defecto del tipo. */
  titulo?: string;
  /** Identificador para reemplazar o cerrar este mismo aviso después. */
  id?: string;
  /** No se va solo y se puede cerrar con la cruz. */
  persistente?: boolean;
  accion?: AccionAviso;
}

function mostrar(tipo: TipoAviso, mensaje: string, opciones: OpcionesAviso = {}): void {
  const persistente = opciones.persistente ?? false;
  const configuracion = {
    id: opciones.id,
    description: mensaje,
    duration: duracionDe(tipo, persistente),
    closeButton: persistente,
    action: opciones.accion && {
      label: opciones.accion.etiqueta,
      onClick: opciones.accion.alAccionar,
    },
  };
  const titulo = opciones.titulo ?? tituloPorDefecto(tipo);
  switch (tipo) {
    case "exito":
      toast.success(titulo, configuracion);
      break;
    case "info":
      toast.info(titulo, configuracion);
      break;
    case "advertencia":
      toast.warning(titulo, configuracion);
      break;
    case "error":
      toast.error(titulo, configuracion);
      break;
  }
}

export interface TextosProgreso<T> {
  cargando: string;
  exito: string | ((resultado: T) => string);
  /** Si falta, se muestra el texto del error tal cual. */
  error?: (error: unknown) => string;
}

export const avisos = {
  /** Algo salió bien. */
  exito: (mensaje: string, opciones?: OpcionesAviso) => mostrar("exito", mensaje, opciones),

  /** Algo que informar, ni bueno ni malo. */
  info: (mensaje: string, opciones?: OpcionesAviso) => mostrar("info", mensaje, opciones),

  /** Requiere atención pero no impide seguir trabajando (un conflicto, un
   * dato incompleto). */
  advertencia: (mensaje: string, opciones?: OpcionesAviso) =>
    mostrar("advertencia", mensaje, opciones),

  /** Algo falló. Acepta el error tal como lo entrega `catch` (un `string`,
   * un `Error` o cualquier cosa) o un texto ya redactado. */
  error: (causa: unknown, opciones?: OpcionesAviso) =>
    mostrar("error", textoDeError(causa), opciones),

  /** Una tarea larga: "cargando…" que se vuelve éxito o error al terminar. */
  progreso<T>(promesa: Promise<T>, textos: TextosProgreso<T>): void {
    toast.promise(promesa, {
      loading: textos.cargando,
      success: textos.exito,
      error: (causa: unknown) => (textos.error ? textos.error(causa) : textoDeError(causa)),
    });
  },

  /** Cierra un aviso por su `id`. */
  cerrar: (id: string) => {
    toast.dismiss(id);
  },
};
