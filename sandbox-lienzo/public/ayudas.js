// Ayudas de dibujo para el código que escribe el asistente (panel "libre" del
// lienzo). El código corre en un Web Worker sin DOM ni red: arma el dibujo como
// texto (SVG o HTML) con estas funciones y lo devuelve. Todo lo que viene de los
// datos pasa por `esc` antes de ir al texto.
//
// Todo vive dentro de `crearAyudas` a propósito: el sandbox copia esta función
// tal cual (toString) adentro del Worker, que se arma en memoria porque un
// iframe con origen anónimo no puede cargar un Worker desde un archivo.

export function crearAyudas(tema = {}) {
  const FORMATO_NUMERO = new Intl.NumberFormat("es-CR", { maximumFractionDigits: 2 });
  const FORMATO_ENTERO = new Intl.NumberFormat("es-CR", { maximumFractionDigits: 0 });
  const MESES = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];
  const DIAS = ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"];
  const NOMBRES_DIAS = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];

  /** Texto seguro para meter en SVG/HTML (contenido o atributo). */
  function esc(valor) {
    return String(valor ?? "")
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#39;");
  }

  /** el(etiqueta, {atributo: valor}, contenido): los atributos se escapan; el contenido va tal cual (armado con el/esc). */
  function el(etiqueta, atributos = {}, contenido = "") {
    const attrs = Object.entries(atributos)
      .filter(([, v]) => v !== undefined && v !== null && v !== false)
      .map(([k, v]) => (v === true ? ` ${k}` : ` ${k}="${esc(v)}"`))
      .join("");
    const cuerpo = Array.isArray(contenido) ? contenido.join("") : contenido;
    return `<${etiqueta}${attrs}>${cuerpo}</${etiqueta}>`;
  }

  /** Un SVG que ocupa el ancho del panel y escala con él. */
  function svg(ancho, alto, contenido, { titulo } = {}) {
    const t = titulo ? `<title>${esc(titulo)}</title>` : "";
    const etiqueta = titulo ? ` aria-label="${esc(titulo)}"` : "";
    const cuerpo = Array.isArray(contenido) ? contenido.join("") : contenido;
    return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${ancho} ${alto}" width="100%" role="img"${etiqueta}>${t}${cuerpo}</svg>`;
  }

  function numero(valor) {
    return typeof valor === "number" ? valor : Number(valor);
  }

  function suma(datos, f = (d) => d) {
    return datos.reduce((s, d) => s + (numero(f(d)) || 0), 0);
  }

  function minimo(datos, f = (d) => d) {
    let m;
    for (const d of datos) {
      const v = f(d);
      if (v !== null && v !== undefined && (m === undefined || v < m)) m = v;
    }
    return m;
  }

  function maximo(datos, f = (d) => d) {
    let m;
    for (const d of datos) {
      const v = f(d);
      if (v !== null && v !== undefined && (m === undefined || v > m)) m = v;
    }
    return m;
  }

  /** Map clave → filas, en el orden en que aparecen las claves. */
  function agrupar(datos, clave) {
    const f = typeof clave === "function" ? clave : (d) => d[clave];
    const grupos = new Map();
    for (const d of datos) {
      const k = f(d) ?? "Sin dato";
      if (!grupos.has(k)) grupos.set(k, []);
      grupos.get(k).push(d);
    }
    return grupos;
  }

  /** [{clave, n}] de mayor a menor (o por clave con {orden: "clave"}). */
  function contar(datos, clave, { orden = "n" } = {}) {
    const lista = [...agrupar(datos, clave)].map(([k, filas]) => ({ clave: k, n: filas.length }));
    return orden === "clave"
      ? lista.sort((a, b) => (a.clave < b.clave ? -1 : a.clave > b.clave ? 1 : 0))
      : lista.sort((a, b) => b.n - a.n);
  }

  /** Marcas "lindas" para un eje (1, 2, 5 × 10^n). */
  function marcas(desde, hasta, cantidad = 5) {
    if (!(hasta > desde)) return [desde];
    const paso0 = (hasta - desde) / Math.max(1, cantidad);
    const potencia = 10 ** Math.floor(Math.log10(paso0));
    const paso = [1, 2, 5, 10].map((m) => m * potencia).find((p) => p >= paso0) ?? 10 * potencia;
    const lista = [];
    for (let v = Math.ceil(desde / paso) * paso; v <= hasta + paso / 1e6; v += paso) lista.push(Number(v.toFixed(10)));
    return lista;
  }

  /** escalaLineal([0, max], [alto, 0]): con .marcas(n) e .invertir(px). */
  function escalaLineal([d0, d1], [r0, r1]) {
    const span = d1 - d0 || 1;
    const f = (v) => r0 + ((numero(v) - d0) / span) * (r1 - r0);
    f.marcas = (n = 5) => marcas(Math.min(d0, d1), Math.max(d0, d1), n);
    f.invertir = (px) => d0 + ((px - r0) / (r1 - r0 || 1)) * span;
    f.dominio = [d0, d1];
    return f;
  }

  /** Bandas (barras, filas): f(clave) → inicio; f.ancho → grosor; f.paso → distancia entre bandas. */
  function escalaBanda(claves, [r0, r1], relleno = 0.2) {
    const lista = [...claves];
    const paso = (r1 - r0) / Math.max(1, lista.length);
    const indice = new Map(lista.map((k, i) => [k, i]));
    const f = (k) => r0 + (indice.get(k) ?? 0) * paso + (paso * relleno) / 2;
    f.ancho = paso * (1 - relleno);
    f.paso = paso;
    f.claves = lista;
    return f;
  }

  const tiempo = (fecha) => Date.parse(String(fecha).slice(0, 10) + "T00:00:00Z");

  /** Escala de tiempo para fechas "AAAA-MM-DD" (o con hora): f(fecha) → px; f.dias → días del rango. */
  function escalaFecha([desde, hasta], [r0, r1]) {
    const t0 = tiempo(desde);
    const t1 = tiempo(hasta);
    const lineal = escalaLineal([t0, t1 === t0 ? t0 + 86400000 : t1], [r0, r1]);
    const f = (fecha) => lineal(tiempo(fecha));
    f.dias = Math.round((t1 - t0) / 86400000) + 1;
    return f;
  }

  function formatoNumero(v, { entero = false } = {}) {
    const n = numero(v);
    if (!Number.isFinite(n)) return "—";
    return (entero ? FORMATO_ENTERO : FORMATO_NUMERO).format(n);
  }

  /** "2026-10-22" → "22 oct" ({anio: true} → "22 oct 2026"; {dia: true} → "jue 22"). */
  function formatoFecha(ymd, { anio = false, dia = false } = {}) {
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(String(ymd ?? ""));
    if (!m) return String(ymd ?? "");
    const [, a, mes, d] = m;
    if (dia) return `${DIAS[new Date(Date.UTC(+a, +mes - 1, +d)).getUTCDay()]} ${+d}`;
    return `${+d} ${MESES[+mes - 1]}${anio ? ` ${a}` : ""}`;
  }

  /** Días entre dos fechas "AAAA-MM-DD" (negativo si `hasta` es antes). */
  function diasEntre(desde, hasta) {
    return Math.round((tiempo(hasta) - tiempo(desde)) / 86400000);
  }

  /** Suma días a "AAAA-MM-DD". */
  function sumarDias(ymd, dias) {
    return new Date(tiempo(ymd) + dias * 86400000).toISOString().slice(0, 10);
  }

  /** Día de la semana de "AAAA-MM-DD": 0 = lunes … 6 = domingo. */
  function diaSemana(ymd) {
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(String(ymd ?? ""));
    if (!m) return null;
    return (new Date(Date.UTC(+m[1], +m[2] - 1, +m[3])).getUTCDay() + 6) % 7;
  }

  function recortar(texto, maximoCaracteres = 18) {
    const t = String(texto ?? "");
    return t.length > maximoCaracteres ? `${t.slice(0, maximoCaracteres - 1)}…` : t;
  }

  /** Mezcla dos colores "#rrggbb": t = 0 da `a`, t = 1 da `b`. */
  function mezclar(a, b, t) {
    const rgb = (hex) => {
      const m = /^#?([0-9a-f]{6})$/i.exec(String(hex).trim());
      const n = m ? parseInt(m[1], 16) : 0;
      return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
    };
    const [ca, cb] = [rgb(a), rgb(b)];
    const canal = (i) => Math.round(ca[i] + (cb[i] - ca[i]) * t).toString(16).padStart(2, "0");
    return `#${canal(0)}${canal(1)}${canal(2)}`;
  }

  // Colores del tema del panel: cada nombre siempre con el mismo color, en orden de aparición.
  const paleta = Array.isArray(tema.colores) && tema.colores.length ? tema.colores : ["#2a78d6"];
  const asignados = new Map();
  const color = (i) => paleta[((i % paleta.length) + paleta.length) % paleta.length];
  const colorPara = (nombre) => {
    if (nombre === "Otros" || nombre === "Sin dato") return tema.otros ?? "#9aa3ad";
    if (!asignados.has(nombre)) asignados.set(nombre, color(asignados.size));
    return asignados.get(nombre);
  };
  /** Rampa de un solo tono para intensidad (mapas de calor): t de 0 (casi el fondo) a 1 (pleno). */
  const intensidad = (t) => {
    const x = Math.max(0, Math.min(1, numero(t) || 0));
    return mezclar(tema.fondo ?? "#ffffff", tema.intenso ?? paleta[0], 0.08 + x * 0.92);
  };

  /** Eje izquierdo con rejilla horizontal. */
  function ejeIzquierdo(escala, { x = 40, ancho = 0, formato = formatoNumero, cantidad = 5 } = {}) {
    return escala
      .marcas(cantidad)
      .map((v) => {
        const y = escala(v);
        const rejilla = ancho ? `<line x1="${x}" x2="${x + ancho}" y1="${y}" y2="${y}" class="rejilla"/>` : "";
        return `${rejilla}<text x="${x - 6}" y="${y + 4}" text-anchor="end" class="eje">${esc(formato(v))}</text>`;
      })
      .join("");
  }

  /** Etiquetas abajo: lista de [valor, x]. */
  function ejeAbajo(posiciones, { y, formato = (v) => v, maximoCaracteres = 14 } = {}) {
    return posiciones
      .map(([valor, x]) => `<text x="${x}" y="${y}" text-anchor="middle" class="eje">${esc(recortar(formato(valor), maximoCaracteres))}</text>`)
      .join("");
  }

  /** Leyenda (HTML) para varias series. */
  function leyenda(nombres) {
    return `<div class="leyenda">${nombres
      .map((n) => `<span><i style="background:${esc(colorPara(n))}"></i>${esc(n)}</span>`)
      .join("")}</div>`;
  }

  /** Atributos de tooltip para el(): el sandbox lo muestra con el mouse o el teclado. */
  function tip(texto) {
    return { "data-tip": texto, tabindex: 0 };
  }

  return {
    esc, el, svg, numero, suma, minimo, maximo, agrupar, contar, marcas, escalaLineal, escalaBanda, escalaFecha,
    formatoNumero, formatoFecha, diasEntre, sumarDias, diaSemana, recortar, mezclar, ejeIzquierdo, ejeAbajo, leyenda,
    tip, color, colorPara, intensidad, NOMBRES_DIAS, NOMBRES_MESES: MESES,
  };
}
