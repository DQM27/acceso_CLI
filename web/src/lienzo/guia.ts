/**
 * Guía "¿Qué puedo pedir?" del asistente del lienzo: lo que sabe hacer, por
 * tema, con ejemplos que se envían con un toque. Es fija (no llama al modelo,
 * no cuesta) y cambia según dónde está la persona: sobre una vista abierta
 * suma ideas para esa vista.
 */

export interface TemaGuia {
  titulo: string;
  explicacion: string;
  /** Pedidos de ejemplo: se envían tal cual al tocarlos. */
  ejemplos: string[];
}

const TEMAS_GENERALES: TemaGuia[] = [
  {
    titulo: "Ver y filtrar",
    explicacion: "Contratistas, PRAIND e ingresos, con los filtros que quiera.",
    ejemplos: ["Ver los PRAIND vencidos por empresa", "¿Qué PRAIND vencen el mes que viene?", "Ingresos de esta semana por sitio"],
  },
  {
    titulo: "Horas pico y calendario",
    explicacion: "Cuándo entra más gente, y cuándo vencen los permisos, día por día.",
    ejemplos: ["Horas pico de las últimas 4 semanas", "Calendario de vencimientos PRAIND de los próximos 6 meses"],
  },
  {
    titulo: "Comparar y ver tendencias",
    explicacion: "Contra la semana o el mes anterior, día por día.",
    ejemplos: ["Ingresos de las últimas 2 semanas contra las 2 anteriores", "Ingresos por día de este mes en una línea"],
  },
  {
    titulo: "Análisis",
    explicacion: "Cálculos que se hacen con Python: pronósticos, promedios y lo que se sale de lo normal.",
    ejemplos: ["¿Cuántos ingresos espero la próxima semana?", "¿Qué días de este mes fueron fuera de lo normal?"],
  },
  {
    titulo: "Cruzar datos",
    explicacion: "Lo que el catálogo no tiene: proveedores, visitas, gafetes, rutas, quién registró.",
    ejemplos: [
      "Contratistas con PRAIND por vencer que no ingresaron en 60 días",
      "¿Qué proveedores vinieron más veces este mes?",
      "¿Qué operador registró más ingresos esta semana?",
    ],
  },
  {
    titulo: "Dibujos a medida",
    explicacion: "Cuando ningún gráfico común sirve, el asistente arma uno especial.",
    ejemplos: ["Línea de tiempo de quién estuvo adentro hoy", "Embudo de ingresos por tipo de ingreso"],
  },
];

const TEMA_DE_LA_VISTA: TemaGuia = {
  titulo: "Sobre esta vista",
  explicacion: "Pídale cambios a lo que está viendo; todo se puede deshacer.",
  ejemplos: ["Agregue un gráfico de torta por empresa", "Muéstrelo sólo como tabla", "Quite todos los filtros", "Ordene la tabla por fecha"],
};

/** Lo que se hace con los botones, sin pedírselo al asistente. */
export const BOTONES_DE_LA_VISTA = [
  "Guardar: la vista queda en «Mis vistas guardadas» y abrirla no vuelve a gastar consultas.",
  "Exportar: los datos en Excel, CSV o PDF, o la vista entera para imprimir.",
  "El ícono de imagen en cada gráfico lo baja como PNG.",
  "Tocar una barra o una porción filtra por ella; Deshacer y Rehacer van y vuelven.",
];

/** Los temas de la guía: sobre una vista abierta, primero las ideas para esa vista. */
export function temasDeLaGuia(hayVistaAbierta: boolean): TemaGuia[] {
  return hayVistaAbierta ? [TEMA_DE_LA_VISTA, ...TEMAS_GENERALES] : TEMAS_GENERALES;
}

/** Saca tildes y signos para comparar sin importar cómo se escribió. */
function plano(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[¿?¡!.,]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

const PEDIDOS_DE_AYUDA = [
  /^(ayuda|help|opciones|comandos|menu|instrucciones)$/,
  /^(que|q) (podes|puedes|puede|sabes|sabe) hacer( vos| usted| tu)?$/,
  /^(que|q) (le |te )?puedo (pedir|preguntar|hacer)( aca| aqui)?$/,
  /^(como|cómo) (te|lo|la) (uso|usa|usamos)$/,
  /^para (que|q) (servis|sirves|sirve)( vos)?$/,
  /^(que|q) (cosas )?(haces|hace)$/,
];

/**
 * ¿El mensaje es un pedido de ayuda general ("¿qué podés hacer?", "ayuda")?
 * Esos abren la guía sin llamar al modelo. Sólo frases cortas y completas: "ayuda
 * con los vencidos de ALDAMA" es un pedido de datos y va al asistente.
 */
export function esPedidoDeAyuda(texto: string): boolean {
  const limpio = plano(texto);
  return limpio.length <= 40 && PEDIDOS_DE_AYUDA.some((r) => r.test(limpio));
}
