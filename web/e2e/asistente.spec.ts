import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";

/**
 * El asistente de punta a punta, en un navegador real contra el panel
 * compilado: la base y el modelo se simulan (no se toca ninguna base de
 * verdad). Recorre cada vista de ejemplo, filtra tocando cada gráfico y quita
 * el filtro, y conversa con el asistente. Falla si un panel muestra "No se
 * pudo dibujar", si la página cae, si hay errores en la consola o si el panel
 * pide algo que la prueba no conoce.
 */

const correo = "admin@example.invalid";
const usuario = {
  id: "00000000-0000-4000-8000-000000000001",
  aud: "authenticated",
  role: "authenticated",
  email: correo,
  app_metadata: { provider: "google", providers: ["google"] },
  user_metadata: { full_name: "Admin de prueba" },
  created_at: "2026-09-09T12:00:00Z",
};
const sitio = { id: "00000000-0000-4000-8000-000000000002", nombre: "Brisas", created_at: "2026-09-09T12:00:00Z" };

const DIAS = ["Lunes", "Martes", "Miércoles", "Jueves", "Viernes", "Sábado", "Domingo"];
const EMPRESAS = ["ACME", "Beta S.A.", "Cía. Ñandú", "Delta", "Fénix"];

function hoyCostaRica(): string {
  return new Date(Date.now() - 6 * 3_600_000).toISOString().slice(0, 10);
}
function sumarDias(ymd: string, dias: number): string {
  const d = new Date(`${ymd}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + dias);
  return d.toISOString().slice(0, 10);
}

function contratistas() {
  const hoy = hoyCostaRica();
  const estados = ["VIGENTE", "POR_VENCER", "VENCIDA", "SIN_REGISTRO", "NO_REQUIERE"];
  return Array.from({ length: 40 }, (_, i) => {
    const dias = ((i * 37) % 420) - 60;
    return {
      nombre: `Persona ${i}`,
      empresa: EMPRESAS[i % EMPRESAS.length],
      estado_praind: estados[i % estados.length],
      vencimiento: i % 9 === 0 ? null : sumarDias(hoy, dias),
      dias_para_vencer: i % 9 === 0 ? null : dias,
      tipo_ingreso: ["PRAIND", "IN_HOUSE", "POR_CORREO", "SWAT"][i % 4],
      sitio: i % 3 === 0 ? sitio.nombre : null,
      acceso_activo: i % 5 !== 0,
    };
  });
}

function ingresos() {
  const hoy = hoyCostaRica();
  return Array.from({ length: 80 }, (_, i) => {
    const dia = sumarDias(hoy, -(i % 14));
    const semana = (new Date(`${dia}T00:00:00Z`).getUTCDay() + 6) % 7;
    return {
      dia,
      nombre: `Persona ${i % 25}`,
      empresa: EMPRESAS[i % EMPRESAS.length],
      tipo_ingreso: ["PRAIND", "IN_HOUSE", "POR_CORREO", "SWAT"][i % 4],
      sitio: sitio.nombre,
      hora: (i * 7) % 24,
      dia_semana: DIAS[semana],
      horas_adentro: i % 6 === 0 ? null : ((i * 13) % 90) / 10,
    };
  });
}

type FilaSimulada = Record<string, string | number | boolean | null>;

/**
 * Lo mismo que hacen `lienzo_contratistas` y `lienzo_ingresos` con sus
 * parámetros: al filtrar, el panel recibe menos filas, como con la base real
 * (si devolviera siempre lo mismo, un gráfico nunca quedaría con menos barras).
 */
function filtrarComoLaBase(filas: FilaSimulada[], p: Record<string, unknown>): FilaSimulada[] {
  const texto = (v: unknown) => String(v ?? "").toLowerCase();
  const entre = (v: unknown, desde: unknown, hasta: unknown) =>
    v !== null && v !== undefined && (desde == null || (v as string) >= (desde as string)) && (hasta == null || (v as string) <= (hasta as string));
  return filas.filter(
    (f) =>
      (!p.p_empresas || (p.p_empresas as string[]).includes(String(f.empresa))) &&
      (!p.p_empresas_excluir || !(p.p_empresas_excluir as string[]).includes(String(f.empresa))) &&
      (!p.p_contiene_empresa || texto(f.empresa).includes(texto(p.p_contiene_empresa))) &&
      (!p.p_contiene_nombre || texto(f.nombre).includes(texto(p.p_contiene_nombre))) &&
      (!p.p_estados_praind || (p.p_estados_praind as string[]).includes(String(f.estado_praind))) &&
      (!p.p_tipos_ingreso || (p.p_tipos_ingreso as string[]).includes(String(f.tipo_ingreso))) &&
      (!p.p_sitio || f.sitio === p.p_sitio) &&
      (p.p_acceso_activo === undefined || f.acceso_activo === p.p_acceso_activo) &&
      (p.p_vencimiento_desde === undefined || entre(f.vencimiento, p.p_vencimiento_desde, p.p_vencimiento_hasta)) &&
      (p.p_dias_desde === undefined ||
        (typeof f.dias_para_vencer === "number" &&
          f.dias_para_vencer >= (p.p_dias_desde as number) &&
          f.dias_para_vencer <= (p.p_dias_hasta as number))) &&
      (p.p_desde === undefined || entre(f.dia, p.p_desde, p.p_hasta)),
  );
}

/** Lo que responde el modelo simulado: arma "horas pico" o contesta con texto. */
const vistaDelAsistente = {
  fuente: "ingresos",
  filtros: [],
  widgets: [
    { tipo: "kpi", titulo: "Ingresos de prueba", medida: "contar" },
    { tipo: "calor", titulo: "Horas pico de prueba", filas: "dia_semana", columnas: "hora", medida: "contar" },
  ],
};

async function preparar(page: Page, desconocidas: string[]) {
  await page.addInitScript(
    ({ usuario }) => {
      const ahora = Math.floor(Date.now() / 1000);
      localStorage.setItem(
        "sb-xidaepyaljzkpbsxrqsm-auth-token",
        JSON.stringify({
          access_token: `prueba.${btoa(JSON.stringify({ sub: usuario.id, exp: ahora + 3600 }))}.firma`,
          refresh_token: "solo-prueba",
          token_type: "bearer",
          expires_in: 3600,
          expires_at: ahora + 3600,
          user: usuario,
        }),
      );
    },
    { usuario },
  );
  // Nada sale del navegador hacia el sandbox real (paneles libres y Python no se recorren acá).
  await page.route("https://lienzo.megabrisas.com/**", (ruta) => ruta.abort());
  await page.route("https://xidaepyaljzkpbsxrqsm.supabase.co/**", async (ruta: Route) => {
    const peticion = ruta.request();
    const url = new URL(peticion.url());
    const responder = (datos: unknown, cabeceras: Record<string, string> = {}) =>
      ruta.fulfill({ status: 200, contentType: "application/json", headers: cabeceras, body: JSON.stringify(datos) });

    switch (url.pathname) {
      case "/auth/v1/user":
        return responder(usuario);
      case "/rest/v1/administradores_panel":
        return responder({ correo });
      case "/rest/v1/sitios":
        return responder([sitio]);
      case "/rest/v1/ia_permisos":
      case "/rest/v1/ia_alertas":
      case "/rest/v1/lienzo_vistas_guardadas":
        return responder([]);
      case "/rest/v1/lienzo_ia_uso":
        return responder([], { "content-range": "*/3" });
      case "/rest/v1/rpc/lienzo_resumen_hoy":
        return responder({
          hoy: hoyCostaRica(),
          adentro_contratistas: 4,
          adentro_proveedores: 1,
          ingresos_hoy: 9,
          praind_vencidos: 3,
          praind_por_vencer: 5,
          ingresos_por_dia: Array.from({ length: 14 }, (_, i) => ({ dia: sumarDias(hoyCostaRica(), -13 + i), cantidad: (i * 5) % 11 })),
          ingresos_semana_pasada_a_esta_hora: 7,
        });
      case "/rest/v1/rpc/lienzo_contratistas":
        return responder(filtrarComoLaBase(contratistas(), peticion.postDataJSON() ?? {}));
      case "/rest/v1/rpc/lienzo_ingresos":
        return responder(filtrarComoLaBase(ingresos(), peticion.postDataJSON() ?? {}));
      case "/functions/v1/lienzo-ia": {
        const { pregunta } = peticion.postDataJSON() as { pregunta: string };
        if (pregunta.includes("horas pico"))
          return responder({
            tipo: "editado",
            lienzo: vistaDelAsistente,
            mensaje: "Listo: armé las horas pico.",
            sugerencias: ["Solo los lunes", "Por empresa"],
          });
        return responder({ tipo: "respondido", mensaje: "Puedo armar gráficos de ingresos y contratistas." });
      }
    }
    desconocidas.push(`${peticion.method()} ${url.pathname}`);
    return ruta.fulfill({ status: 404, contentType: "application/json", body: "{}" });
  });
}

test.describe("asistente", () => {
  // El chat y el lienzo lado a lado: se prueba en escritorio.
  test.skip(({ isMobile }) => isMobile, "sólo escritorio");
  // Sin animaciones (el panel respeta "reducir movimiento"): en una máquina lenta, como la
  // de GitHub, los gráficos todavía moviéndose hacían que los clics cayeran a destiempo.
  test.use({ reducedMotion: "reduce" });

  const errores: string[] = [];
  const desconocidas: string[] = [];

  test.beforeEach(async ({ page }) => {
    errores.length = 0;
    desconocidas.length = 0;
    page.on("pageerror", (e) => errores.push(`pageerror: ${e.message}`));
    page.on("console", (m) => {
      if (m.type() !== "error") return;
      // Los pedidos abortados al sandbox no cuentan: se cortan a propósito.
      const deSandbox = (() => {
        try {
          return new URL(m.location().url).hostname === "lienzo.megabrisas.com";
        } catch {
          return false;
        }
      })();
      if (!deSandbox && !m.text().includes("net::ERR_FAILED")) errores.push(m.text());
    });
    // LENTO=4 npx playwright test …: frena la CPU para imitar la máquina de GitHub.
    if (process.env.LENTO) {
      const cdp = await page.context().newCDPSession(page);
      await cdp.send("Emulation.setCPUThrottlingRate", { rate: Number(process.env.LENTO) });
    }
    await preparar(page, desconocidas);
    await page.goto("/asistente");
    await expect(page.getByRole("combobox", { name: "Vista del lienzo" })).toBeVisible();
  });

  test.afterEach(async ({ page }) => {
    await expect(page.getByText("No se pudo dibujar este panel")).toHaveCount(0);
    await expect(page.getByText("Ocurrió un error inesperado")).toHaveCount(0);
    expect(errores).toEqual([]);
    expect(desconocidas).toEqual([]);
  });

  test("cada vista de ejemplo se dibuja y cada gráfico filtra al tocarlo", async ({ page }) => {
    const selector = page.getByRole("combobox", { name: "Vista del lienzo" });
    // Las vistas salen del selector mismo; la de Python necesita el sandbox real y queda afuera.
    const vistas = await selector
      .locator("option")
      .evaluateAll((opciones) =>
        (opciones as HTMLOptionElement[]).map((o) => ({ valor: o.value, texto: o.textContent ?? "" })),
      );
    const deEjemplo = vistas.filter((v) => !v.texto.includes("Python") && !v.texto.startsWith("Alertas"));
    expect(deEjemplo.length).toBeGreaterThan(5);
    const quitar = page.getByLabel("Filtros del lienzo").getByRole("button", { name: /^Quitar filtro/ });

    // Con datos en camino la vista se atenúa (`asistente-actualizando`): se espera a que lleguen.
    const quieta = async () => {
      await expect(page.locator(".lienzo-panel").first()).toBeVisible();
      await expect(page.locator(".asistente-actualizando")).toHaveCount(0, { timeout: 15_000 });
    };

    for (const vista of deEjemplo) {
      await test.step(vista.texto, async () => {
        await selector.selectOption(vista.valor);
        await quieta();
        const filtrosIniciales = await quitar.count();
        const graficos = page.locator(".lienzo-panel").filter({ has: page.locator(".lienzo-zona-seleccionable") });
        const cuantos = await graficos.count();
        // La primera y la última marca de cada gráfico (la última es la que, al filtrar, deja de existir).
        for (let g = 0; g < cuantos * 2; g++) {
          const marcas = graficos.nth(Math.floor(g / 2)).locator(".lienzo-zona-seleccionable");
          const zona = g % 2 === 0 ? marcas.first() : marcas.last();
          // Pasar el puntero (resaltado y tooltip) y el clic directo a la marca: mientras el
          // gráfico anima, un clic por coordenadas puede caer al lado.
          await zona.dispatchEvent("pointerover");
          await zona.dispatchEvent("click");
          // El filtro nuevo aparece como chip (el mapa de calor pone dos: fila y columna)
          // y la vista se vuelve a dibujar sin romperse.
          await expect.poll(() => quitar.count()).toBeGreaterThan(filtrosIniciales);
          await quieta();
          await expect(page.getByText("No se pudo dibujar este panel")).toHaveCount(0);
          while ((await quitar.count()) > filtrosIniciales) {
            const antes = await quitar.count();
            await quitar.last().click();
            await expect(quitar).toHaveCount(antes - 1);
            await quieta();
          }
        }
      });
    }
  });

  test("el asistente arma una vista desde el chat, se filtra y se deshace", async ({ page }) => {
    const mensaje = page.getByRole("textbox", { name: "Mensaje para el asistente" });
    await mensaje.fill("mostrame las horas pico");
    await page.getByRole("button", { name: "Enviar" }).click();
    await expect(page.getByText("Listo: armé las horas pico.")).toBeVisible();
    await expect(page.getByRole("heading", { name: "Horas pico de prueba" })).toBeVisible();
    await expect(page.getByText("Ingresos de prueba")).toBeVisible();
    await expect(page.getByRole("button", { name: "Solo los lunes" })).toBeVisible();

    // Una celda del mapa de calor filtra por día y hora; Deshacer vuelve atrás.
    await page.locator(".lienzo-calor-celda.lienzo-zona-seleccionable").first().click();
    const quitar = page.getByLabel("Filtros del lienzo").getByRole("button", { name: /^Quitar filtro/ });
    await expect(quitar).toHaveCount(2);
    await page.getByRole("button", { name: "Deshacer" }).click();
    await expect(quitar).toHaveCount(0);

    // Una pregunta que no cambia la vista: responde con texto y el lienzo queda igual.
    await mensaje.fill("qué podés hacer con los datos");
    await page.getByRole("button", { name: "Enviar" }).click();
    await expect(page.getByText("Puedo armar gráficos de ingresos y contratistas.")).toBeVisible();
    await expect(page.getByRole("heading", { name: "Horas pico de prueba" })).toBeVisible();
  });
});
