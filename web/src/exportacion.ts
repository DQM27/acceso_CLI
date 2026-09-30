/**
 * Exportación de tablas del panel a Excel, CSV y PDF, común a todas las
 * pantallas (Historial, Sesiones). Una sola definición de columnas
 * (`DefinicionColumnaExport`) alimenta los tres formatos, así la tabla en
 * pantalla y lo exportado no divergen -- mismo criterio que
 * `src/historial/exportacion.rs` del lado de escritorio.
 */

/** Una columna exportable: su `colId` coincide con el de la grilla, así se
 * exportan las columnas visibles y en el orden que eligió la persona. */
export interface DefinicionColumnaExport<T> {
  colId: string;
  etiqueta: string;
  izquierda?: boolean;
  valor: (fila: T) => string;
}

/** Columnas a exportar según lo visible en la grilla (en su orden). Sin
 * grilla todavía, todas las definidas. */
export function definicionesVisibles<T>(
  definiciones: DefinicionColumnaExport<T>[],
  colIdsVisibles: string[] | undefined,
): DefinicionColumnaExport<T>[] {
  const colIds = colIdsVisibles ?? definiciones.map((d) => d.colId);
  return colIds
    .map((colId) => definiciones.find((d) => d.colId === colId))
    .filter((d): d is DefinicionColumnaExport<T> => d !== undefined);
}

/** Escapa lo mínimo indispensable para HTML -- mismo motivo que
 * `escapar` en `desktop/src-tauri/src/pdf/html.rs`: nombres/empresas
 * reales nunca se validaron como "sin `<`/`&`". */
export function escaparHtml(texto: string): string {
  return texto
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** Misma paleta clara y layout que `desktop/src-tauri/src/pdf/html.rs`
 * (título + subtítulo de filtro a la izquierda, "Generado por"/fecha a la
 * derecha, tabla en cebra, horizontal/landscape) -- así el PDF que sale de
 * la web se ve igual al que ya conocen del escritorio, aunque el motor que
 * lo imprime sea el navegador en vez de WebView2. */
export function generarHtmlTabla<T>(
  filas: T[],
  columnas: DefinicionColumnaExport<T>[],
  opciones: { titulo: string; generadoPor: string; filtro: string },
): string {
  const encabezados = columnas
    .map((c) => `<th${c.izquierda ? ' class="izquierda"' : ""}>${escaparHtml(c.etiqueta.toUpperCase())}</th>`)
    .join("");

  const filasHtml = filas
    .map((fila) => {
      const celdas = columnas
        .map((c) => `<td${c.izquierda ? ' class="izquierda"' : ""}>${escaparHtml(c.valor(fila))}</td>`)
        .join("");
      return `<tr>${celdas}</tr>`;
    })
    .join("");

  const generadoEn = new Date().toLocaleString("es-CR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });

  return `<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8" />
<title>${escaparHtml(opciones.titulo)}</title>
<style>
  :root {
    --acento: #087f91;
    --texto: #172026;
    --muted: #63717c;
    --borde: #d8e0e5;
    --panel-suave: #eef3f5;
    --zebra: #d9eaf7;
  }
  @page { size: letter landscape; margin: 1.1cm 0.9cm; }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    font-family: Arial, Helvetica, sans-serif;
    font-size: 8.5pt;
    color: var(--texto);
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    border-bottom: 2px solid var(--acento);
    padding: 0 0.3cm 0.5em;
    margin-bottom: 0.7em;
  }
  header h1 { margin: 0; font-size: 14pt; color: var(--acento); }
  header .subtitulo { margin: 0.15em 0 0; font-size: 8.5pt; color: var(--muted); }
  .meta { text-align: right; font-size: 8pt; color: var(--muted); line-height: 1.5; }
  .meta strong { color: var(--texto); }
  table { width: 100%; border-collapse: collapse; }
  thead { display: table-header-group; }
  tr { page-break-inside: avoid; }
  th, td { border: 1px solid var(--borde); padding: 0.28em 0.4em; text-align: center; }
  th { background: var(--panel-suave); font-weight: bold; font-size: 7.5pt; }
  td.izquierda, th.izquierda { text-align: left; }
  tbody tr:nth-child(even) { background: var(--zebra); }
</style>
</head>
<body>
  <header>
    <div>
      <h1>${escaparHtml(opciones.titulo)}</h1>
      <p class="subtitulo">${escaparHtml(opciones.filtro)}</p>
    </div>
    <div class="meta">
      Generado por: <strong>${escaparHtml(opciones.generadoPor)}</strong><br />
      ${escaparHtml(generadoEn)}
    </div>
  </header>
  <table>
    <thead><tr>${encabezados}</tr></thead>
    <tbody>${filasHtml}</tbody>
  </table>
</body>
</html>`;
}

/** CSV como el de escritorio: separador `;`, comillas si el texto lleva `;`,
 * comillas o saltos de línea. Empieza con BOM para que Excel lea las tildes. */
export function generarCsv<T>(filas: T[], columnas: DefinicionColumnaExport<T>[]): string {
  const celda = (texto: string) => (/[;"\r\n]/.test(texto) ? `"${texto.replace(/"/g, '""')}"` : texto);
  const lineas = [
    columnas.map((c) => celda(c.etiqueta)).join(";"),
    ...filas.map((fila) => columnas.map((c) => celda(c.valor(fila))).join(";")),
  ];
  return "\uFEFF" + lineas.join("\r\n") + "\r\n";
}

export function descargarArchivo(nombre: string, contenido: BlobPart, tipo: string) {
  const url = URL.createObjectURL(new Blob([contenido], { type: tipo }));
  const enlace = document.createElement("a");
  enlace.href = url;
  enlace.download = nombre;
  enlace.click();
  URL.revokeObjectURL(url);
}


/** Hoja de Excel con las columnas y filas dadas (SheetJS, cargado sólo al
 * exportar). */
export async function descargarExcel<T>(
  filas: T[],
  columnas: DefinicionColumnaExport<T>[],
  { hoja, archivo }: { hoja: string; archivo: string },
): Promise<void> {
  const XLSX = await import("xlsx");
  const encabezados = columnas.map((d) => d.etiqueta);
  const filasHoja = filas.map((fila) => columnas.map((d) => d.valor(fila)));
  const libro = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(libro, XLSX.utils.aoa_to_sheet([encabezados, ...filasHoja]), hoja);
  XLSX.writeFile(libro, archivo);
}

/** Abre el diálogo de impresión del navegador ("Guardar como PDF") con el
 * HTML dado. Un iframe oculto aísla el documento de impresión del resto de
 * la página -- `window.print()` imprime la ventana completa si no se aísla,
 * arrastrando el sidebar y la grilla. Devuelve `false` si no se pudo
 * preparar. */
export function imprimirHtml(html: string): boolean {
  const iframe = document.createElement("iframe");
  iframe.style.position = "fixed";
  iframe.style.top = "-10000px";
  iframe.style.left = "-10000px";
  document.body.appendChild(iframe);

  const documento = iframe.contentWindow?.document;
  if (!documento) {
    document.body.removeChild(iframe);
    return false;
  }
  documento.open();
  documento.write(html);
  documento.close();

  iframe.contentWindow?.addEventListener("afterprint", () => {
    document.body.removeChild(iframe);
  });
  // Esperar a que el iframe termine de pintar el HTML recién escrito --
  // sin esto, `print()` puede dispararse sobre un documento todavía en
  // blanco en algunos navegadores.
  setTimeout(() => iframe.contentWindow?.print(), 150);
  return true;
}
