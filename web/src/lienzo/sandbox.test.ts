import { describe, expect, it } from "vitest";
import { crearAyudas } from "../../../sandbox-lienzo/public/ayudas.js";
import { limpiarDibujo } from "../../../sandbox-lienzo/public/limpiar.js";
import { fuenteDelTrabajador } from "../../../sandbox-lienzo/public/trabajador.js";
import { EJEMPLO_LIBRE } from "../../../supabase/functions/lienzo-ia/instrucciones.ts";

// Sandbox del panel "libre" (sandbox-lienzo/): ayudas de dibujo, el Worker que
// corre el código del asistente y el limpiador de lo que devuelve.

const TEMA = { colores: ["#111111", "#222222", "#333333"], otros: "#999999", fondo: "#ffffff", intenso: "#000000" };

/** Corre el Worker como lo arma el sandbox (texto suelto), con un `self` simulado. */
function correrEnTrabajador(codigo: string, datos: unknown[] = [], columnas: unknown[] = []) {
  const enviados: unknown[] = [];
  const falsoSelf: { onmessage?: (e: { data: unknown }) => void; postMessage: (m: unknown) => void } = {
    postMessage: (m) => enviados.push(m),
  };
  new Function("self", fuenteDelTrabajador(crearAyudas))(falsoSelf);
  falsoSelf.onmessage?.({ data: { codigo, datos, columnas, ancho: 600, tema: TEMA } });
  return enviados[0] as { ok: boolean; html?: string; mensaje?: string };
}

describe("ayudas de dibujo", () => {
  const h = crearAyudas(TEMA);

  it("escapan lo que viene de los datos", () => {
    expect(h.esc(`<script>"x" & 'y'</script>`)).toBe("&lt;script&gt;&quot;x&quot; &amp; &#39;y&#39;&lt;/script&gt;");
    expect(h.el("text", { x: 1, "data-tip": `<b>"ALDAMA"</b>` }, h.esc("A<B"))).toBe(
      '<text x="1" data-tip="&lt;b&gt;&quot;ALDAMA&quot;&lt;/b&gt;">A&lt;B</text>',
    );
  });

  it("escalas: lineal con marcas lindas, bandas y fechas", () => {
    const y = h.escalaLineal([0, 10], [200, 0]);
    expect(y(0)).toBe(200);
    expect(y(10)).toBe(0);
    expect(y.marcas(5)).toEqual([0, 2, 4, 6, 8, 10]);
    const x = h.escalaBanda(["a", "b"], [0, 100], 0);
    expect([x("a"), x("b"), x.ancho]).toEqual([0, 50, 50]);
    const f = h.escalaFecha(["2026-10-01", "2026-10-11"], [0, 100]);
    expect(f("2026-10-06")).toBe(50);
    expect(f.dias).toBe(11);
  });

  it("agrupa, cuenta y formatea en español de Costa Rica", () => {
    const filas = [{ e: "A" }, { e: "B" }, { e: "A" }, { e: null }];
    expect(h.contar(filas, "e")).toEqual([
      { clave: "A", n: 2 },
      { clave: "B", n: 1 },
      { clave: "Sin dato", n: 1 },
    ]);
    expect(h.formatoFecha("2026-10-22")).toBe("22 oct");
    expect(h.formatoFecha("2026-10-22", { dia: true })).toBe("jue 22");
    expect(h.diaSemana("2026-10-19")).toBe(0);
    expect(h.diasEntre("2026-10-01", "2026-11-01")).toBe(31);
    expect(h.sumarDias("2026-12-31", 1)).toBe("2027-01-01");
    expect(h.rangoDias("2026-10-30", "2026-11-02")).toEqual(["2026-10-30", "2026-10-31", "2026-11-01", "2026-11-02"]);
    expect(h.rangoDias("2026-10-02", "2026-10-01")).toEqual([]);
    expect(h.cadaCuanto(20, 48)).toBe(3);
    expect(h.cadaCuanto(80, 48)).toBe(1);
  });

  it("colores: cada nombre siempre el mismo, Otros en gris, intensidad entre fondo y tono", () => {
    expect(h.colorPara("ALDAMA")).toBe("#111111");
    expect(h.colorPara("OTRA")).toBe("#222222");
    expect(h.colorPara("ALDAMA")).toBe("#111111");
    expect(h.colorPara("Otros")).toBe("#999999");
    expect(h.intensidad(1)).toBe("#000000");
    expect(h.mezclar("#000000", "#ffffff", 0.5)).toBe("#808080");
  });
});

describe("Worker del sandbox", () => {
  it("el código del asistente recibe datos y ayudas y devuelve el dibujo (las ayudas viajan enteras como texto)", () => {
    const codigo = `
      const conteo = h.contar(datos, "empresa");
      const x = h.escalaBanda(conteo.map((c) => c.clave), [0, ancho]);
      const y = h.escalaLineal([0, h.maximo(conteo, (c) => c.n)], [100, 0]);
      return h.svg(ancho, 120, conteo.map((c) =>
        h.el("rect", { x: x(c.clave), y: y(c.n), width: x.ancho, height: 100 - y(c.n), fill: h.colorPara(c.clave),
          ...h.tip(c.clave + ": " + h.formatoNumero(c.n)) })), { titulo: "Ingresos" });`;
    const r = correrEnTrabajador(codigo, [{ empresa: "ALDAMA" }, { empresa: "ALDAMA" }, { empresa: "K-9" }]);
    expect(r.ok).toBe(true);
    expect(r.html).toContain('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 120"');
    expect(r.html).toContain('data-tip="ALDAMA: 2"');
    expect(r.html).toContain('fill="#111111"');
  });

  it("el ejemplo que se le da al modelo corre y dibuja (mapa de calor empresa × día)", () => {
    const datos = [
      { dia: "2026-10-20", empresa: "ALDAMA", nombre: "Ana" },
      { dia: "2026-10-20", empresa: "ALDAMA", nombre: "Beto" },
      { dia: "2026-10-21", empresa: "K-9 <INTER>", nombre: "Cris" },
    ];
    const r = correrEnTrabajador(EJEMPLO_LIBRE, datos);
    expect(r.ok, r.mensaje).toBe(true);
    expect(r.html).toContain('aria-label="Ingresos por empresa y día"');
    // 20/10/2026 es martes.
    expect(r.html).toContain("ALDAMA · mar 20: 2 ingresos");
    expect(r.html).toContain("· mié 21: 1 ingreso");
    // El nombre con < > llega escapado: no puede abrir una etiqueta.
    expect(r.html).toContain("K-9 &lt;INTER&gt;");
    expect(r.html).not.toContain("<INTER>");
    // Sin datos, avisa con texto en vez de dibujar un hueco.
    expect(correrEnTrabajador(EJEMPLO_LIBRE, [])).toEqual({ ok: true, html: "<p>Sin ingresos en el rango.</p>" });
  });

  it("el ejemplo dibuja todos los días del rango y sus fechas no se pisan", () => {
    // Dos semanas con un hueco en el medio (días sin ingresos).
    const datos = ["2026-10-08", "2026-10-09", "2026-10-15", "2026-10-21"].map((dia) => ({ dia, empresa: "ALDAMA" }));
    const r = correrEnTrabajador(EJEMPLO_LIBRE, datos);
    expect(r.ok, r.mensaje).toBe(true);
    // 14 celdas: del 8 al 21, también los días vacíos.
    expect(r.html?.match(/<rect /g)).toHaveLength(14);
    expect(r.html).toContain("ALDAMA · mar 13: 0 ingresos");
    // Los rótulos de fecha quedan separados al menos el ancho de un rótulo (~48 px).
    const xs = [...(r.html ?? "").matchAll(/<text x="([\d.]+)" y="16"/g)].map((m) => Number(m[1]));
    expect(xs.length).toBeGreaterThan(1);
    for (let i = 1; i < xs.length; i++) expect(xs[i] - xs[i - 1]).toBeGreaterThanOrEqual(48);
  });

  it("si el código falla o no devuelve texto, avisa en vez de romper", () => {
    expect(correrEnTrabajador("throw new Error('ups')")).toEqual({ ok: false, mensaje: "ups" });
    expect(correrEnTrabajador("return 42").ok).toBe(false);
    expect(correrEnTrabajador("return (").ok).toBe(false);
  });

  it("el Worker anula fetch, XMLHttpRequest y compañía, y no se pueden volver a poner", () => {
    const falsoSelf: Record<string, unknown> = { postMessage: () => undefined, fetch: () => "red" };
    new Function("self", fuenteDelTrabajador(crearAyudas))(falsoSelf);
    for (const nombre of ["fetch", "XMLHttpRequest", "WebSocket", "importScripts"]) {
      expect(falsoSelf[nombre]).toBeUndefined();
      expect(Object.getOwnPropertyDescriptor(falsoSelf, nombre)?.writable).toBe(false);
    }
  });
});

describe("limpiarDibujo", () => {
  function limpio(html: string): string {
    const div = document.createElement("div");
    div.append(limpiarDibujo(html));
    return div.innerHTML;
  }

  it("deja pasar el dibujo: SVG, degradados internos, tooltips, tablas", () => {
    const html = limpio(
      '<svg viewBox="0 0 10 10"><defs><linearGradient id="g"><stop offset="0" stop-color="#fff"/></linearGradient></defs>' +
        '<rect fill="url(#g)" data-tip="A: 2" tabindex="0" width="5" height="5"/><use href="#g"/></svg><table><tr><td>x</td></tr></table>',
    );
    expect(html).toContain('fill="url(#g)"');
    expect(html).toContain('data-tip="A: 2"');
    expect(html).toContain('href="#g"');
    expect(html).toContain("<td>x</td>");
  });

  it("saca scripts, eventos, enlaces, cargas externas y lo que navega", () => {
    const html = limpio(
      '<script>alert(1)</script><img src="https://x.test/a.png" onerror="alert(1)">' +
        '<svg onload="alert(1)"><a href="javascript:alert(1)"><text>clic</text></a>' +
        '<rect style="fill:url(https://x.test/robar)" width="1"/><use href="https://x.test/s.svg#a"/>' +
        '<foreignObject><iframe src="https://x.test"></iframe></foreignObject></svg>' +
        '<meta http-equiv="refresh" content="0;url=https://x.test"><form action="https://x.test"><input></form>' +
        '<div onclick="alert(1)" style="color:red">texto</div>',
    );
    for (const prohibido of ["<script", "<img", "onerror", "onload", "onclick", "javascript:", "x.test", "<iframe", "<meta", "<form", "<a "]) {
      expect(html).not.toContain(prohibido);
    }
    // Lo inofensivo sigue: el texto del enlace y el estilo sin direcciones.
    expect(html).toContain("<text>clic</text>");
    expect(html).toContain('style="color:red"');
  });
});
