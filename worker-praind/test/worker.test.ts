import { env } from "cloudflare:test";
import { beforeEach, describe, expect, it } from "vitest";
import { manejar } from "../src/index";
import { decodificarEntidades, leerPagina } from "../src/origen";
import { hashHex, vaciarCachePermisos } from "../src/supabase";
import type { Dependencias, Env } from "../src/tipos";

const ENV = env as unknown as Env;
const SUPABASE = ENV.SUPABASE_URL;
const PANEL = "https://panel.megabrisas.com";
const TOKEN = "token-de-prueba";

const ENLACE = "https://prd.pw/codigoDePrueba123";
const PAGINA =
  "https://safety-record.com/api-aleluya/v0-aleluya/trabajadores-aleluya/codigoDePrueba123/despliegue_publico_aleluya";
const IMAGEN = "https://media-chirho.safety-record.com/display_certification_generations_chirho/2026/08/prueba.png";

const PNG = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3, 4, 5]);
const JPEG = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 9, 9, 9]);

// Misma forma que la página real (nombre en el primer <h2>, una sola imagen).
const HTML = `<!DOCTYPE html><html><head><meta charset="utf8"></head>
<div class="columns"><div class="column col-xl-12">
  <h2 style="color: Blue; text-align: center;">
      Persona De Prueba Mu&ntilde;oz
  </h2>
  <h3>Esta persona tiene entrenamientos asignados</h3></div>
  <div><h4>Programa De Contratistas</h4></div></div>
<img src="${IMAGEN}" alt="certificate_chirho" style="width: 100%"/></html>`;

type Manejador = (init: RequestInit | undefined) => Response | Promise<Response>;

interface Simulacion {
  deps: Dependencias;
  llamadas: string[];
  rpc: (nombre: string) => number;
}

function simular(rutas: Record<string, Manejador>): Simulacion {
  const llamadas: string[] = [];
  return {
    llamadas,
    rpc: (nombre) => llamadas.filter((u) => u === `${SUPABASE}/rest/v1/rpc/${nombre}`).length,
    deps: {
      async fetch(entrada, init) {
        llamadas.push(entrada);
        const manejador = rutas[entrada];
        if (!manejador) throw new Error(`salida no esperada: ${entrada}`);
        return manejador(init);
      },
    },
  };
}

const rpcJson = (valor: unknown, estado = 200): Manejador => () =>
  new Response(JSON.stringify(valor), { status: estado, headers: { "Content-Type": "application/json" } });

const permisosRpc = (puede_ver: boolean, puede_cargar: boolean) => ({
  [`${SUPABASE}/rest/v1/rpc/praind_permisos`]: rpcJson({ puede_ver, puede_cargar, es_admin: false }),
});

const origenNormal: Record<string, Manejador> = {
  [ENLACE]: () => new Response(null, { status: 307, headers: { Location: PAGINA } }),
  [PAGINA]: () => new Response(HTML, { headers: { "Content-Type": "text/html" } }),
  [IMAGEN]: () => new Response(PNG, { headers: { "Content-Type": "image/png" } }),
};

function pedido(metodo: string, ruta: string, opciones: { cuerpo?: BodyInit; cabeceras?: Record<string, string> } = {}) {
  return new Request(`https://praind.megabrisas.com${ruta}`, {
    method: metodo,
    body: opciones.cuerpo,
    headers: { Authorization: `Bearer ${TOKEN}`, ...opciones.cabeceras },
  });
}

let contador = 0;
function nuevoContratista(): string {
  contador++;
  return `00000000-0000-4000-8000-${String(contador).padStart(12, "0")}`;
}

async function claveDe(contratista: string, bytes: Uint8Array, ext: string) {
  return `contratistas/${contratista}/${await hashHex(bytes)}.${ext}`;
}

beforeEach(() => vaciarCachePermisos());

describe("GET /foto", () => {
  it("pide sesión", async () => {
    const s = simular({});
    const r = await manejar(new Request("https://praind.megabrisas.com/foto/contratistas/x"), ENV, s.deps);
    expect(r.status).toBe(404);
    const contratista = nuevoContratista();
    const clave = await claveDe(contratista, PNG, "png");
    const sinToken = await manejar(new Request(`https://praind.megabrisas.com/foto/${clave}`), ENV, s.deps);
    expect(sinToken.status).toBe(401);
    expect(s.llamadas).toEqual([]);
  });

  it("sin permiso de ver responde 403 sin tocar R2", async () => {
    const contratista = nuevoContratista();
    const clave = await claveDe(contratista, PNG, "png");
    await ENV.FOTOS.put(clave, PNG);
    const s = simular(permisosRpc(false, false));
    const r = await manejar(pedido("GET", `/foto/${clave}`), ENV, s.deps);
    expect(r.status).toBe(403);
  });

  it("entrega el original con caché privada e inmutable, y reutiliza los permisos", async () => {
    const contratista = nuevoContratista();
    const clave = await claveDe(contratista, PNG, "png");
    await ENV.FOTOS.put(clave, PNG);
    const s = simular(permisosRpc(true, false));

    const r = await manejar(pedido("GET", `/foto/${clave}`), ENV, s.deps);
    expect(r.status).toBe(200);
    expect(r.headers.get("Content-Type")).toBe("image/png");
    expect(r.headers.get("Cache-Control")).toBe("private, max-age=31536000, immutable");
    expect(new Uint8Array(await r.arrayBuffer())).toEqual(PNG);

    const etag = r.headers.get("ETag")!;
    const otra = await manejar(pedido("GET", `/foto/${clave}`, { cabeceras: { "If-None-Match": etag } }), ENV, s.deps);
    expect(otra.status).toBe(304);
    expect(s.rpc("praind_permisos")).toBe(1);
  });

  it("una sesión vencida responde 401", async () => {
    const contratista = nuevoContratista();
    const clave = await claveDe(contratista, PNG, "png");
    const s = simular({
      [`${SUPABASE}/rest/v1/rpc/praind_permisos`]: rpcJson({ code: "PGRST301", message: "JWT expired" }, 401),
    });
    const r = await manejar(pedido("GET", `/foto/${clave}`), ENV, s.deps);
    expect(r.status).toBe(401);
  });
});

describe("POST /cargar-desde-qr", () => {
  const cuerpo = (contratista: string, extra: Record<string, unknown> = {}) =>
    JSON.stringify({ contratista_id: contratista, enlace: ENLACE, ...extra });

  it("baja el carné, lo guarda y devuelve el nombre de la página", async () => {
    const contratista = nuevoContratista();
    let argumentos: Record<string, unknown> = {};
    const s = simular({
      ...permisosRpc(true, true),
      ...origenNormal,
      [`${SUPABASE}/rest/v1/rpc/praind_registrar`]: (init) => {
        argumentos = JSON.parse(String(init?.body));
        return rpcJson(null)(init);
      },
    });
    const r = await manejar(pedido("POST", "/cargar-desde-qr", { cuerpo: cuerpo(contratista) }), ENV, s.deps);
    expect(r.status).toBe(200);
    const datos = (await r.json()) as Record<string, unknown>;
    const clave = await claveDe(contratista, PNG, "png");
    expect(datos).toEqual({ clave, nombre_en_origen: "Persona De Prueba Muñoz", bytes: PNG.byteLength, tipo_mime: "image/png" });
    expect(argumentos).toMatchObject({ p_contratista_id: contratista, p_clave: clave, p_enlace_origen: ENLACE });
    expect(await ENV.FOTOS.head(clave)).not.toBeNull();
  });

  it("solo_previsualizar no guarda nada", async () => {
    const contratista = nuevoContratista();
    const s = simular({ ...permisosRpc(true, true), ...origenNormal });
    const r = await manejar(
      pedido("POST", "/cargar-desde-qr", { cuerpo: cuerpo(contratista, { solo_previsualizar: true }) }),
      ENV,
      s.deps,
    );
    expect(r.status).toBe(200);
    expect(((await r.json()) as Record<string, unknown>).nombre_en_origen).toBe("Persona De Prueba Muñoz");
    expect(s.rpc("praind_registrar")).toBe(0);
    expect((await ENV.FOTOS.list({ prefix: `contratistas/${contratista}/` })).objects).toHaveLength(0);
  });

  it("rechaza enlaces que no son de carnés sin salir a internet", async () => {
    const contratista = nuevoContratista();
    for (const enlace of [
      "http://prd.pw/abc",
      "https://ejemplo.com/abc",
      "https://prd.pw.ejemplo.com/abc",
      "https://usuario@prd.pw/abc",
      "https://prd.pw:8443/abc",
      "javascript:alert(1)",
      "",
    ]) {
      const s = simular({});
      const r = await manejar(
        pedido("POST", "/cargar-desde-qr", { cuerpo: JSON.stringify({ contratista_id: contratista, enlace }) }),
        ENV,
        s.deps,
      );
      expect(r.status, enlace).toBe(400);
      expect(s.llamadas, enlace).toEqual([]);
    }
  });

  it("no sigue una redirección hacia otro sitio", async () => {
    const contratista = nuevoContratista();
    const s = simular({
      ...permisosRpc(true, true),
      [ENLACE]: () => new Response(null, { status: 302, headers: { Location: "http://169.254.169.254/latest" } }),
    });
    const r = await manejar(pedido("POST", "/cargar-desde-qr", { cuerpo: cuerpo(contratista) }), ENV, s.deps);
    expect(r.status).toBe(502);
    expect(s.llamadas).not.toContain("http://169.254.169.254/latest");
  });

  it("corta una cadena larga de redirecciones", async () => {
    const contratista = nuevoContratista();
    const s = simular({
      ...permisosRpc(true, true),
      [ENLACE]: () => new Response(null, { status: 302, headers: { Location: ENLACE } }),
    });
    const r = await manejar(pedido("POST", "/cargar-desde-qr", { cuerpo: cuerpo(contratista) }), ENV, s.deps);
    expect(r.status).toBe(502);
    expect(s.llamadas.filter((u) => u === ENLACE).length).toBe(4);
  });

  it("ignora imágenes de otros anfitriones y rechaza lo que no es imagen", async () => {
    const contratista = nuevoContratista();
    const ajena = HTML.replace(IMAGEN, "https://ejemplo.com/x.png");
    const s1 = simular({ ...permisosRpc(true, true), ...origenNormal, [PAGINA]: () => new Response(ajena) });
    expect((await manejar(pedido("POST", "/cargar-desde-qr", { cuerpo: cuerpo(contratista) }), ENV, s1.deps)).status).toBe(502);
    expect(s1.llamadas).not.toContain("https://ejemplo.com/x.png");

    const s2 = simular({ ...permisosRpc(true, true), ...origenNormal, [IMAGEN]: () => new Response("<html>no</html>") });
    expect((await manejar(pedido("POST", "/cargar-desde-qr", { cuerpo: cuerpo(contratista) }), ENV, s2.deps)).status).toBe(502);
  });

  it("sin permiso de carga no consulta la página", async () => {
    const contratista = nuevoContratista();
    const s = simular(permisosRpc(true, false));
    const r = await manejar(pedido("POST", "/cargar-desde-qr", { cuerpo: cuerpo(contratista) }), ENV, s.deps);
    expect(r.status).toBe(403);
    expect(s.llamadas).not.toContain(ENLACE);
  });

  it("si Supabase rechaza el registro, borra el objeto recién escrito", async () => {
    const contratista = nuevoContratista();
    const s = simular({
      ...permisosRpc(true, true),
      ...origenNormal,
      [`${SUPABASE}/rest/v1/rpc/praind_registrar`]: rpcJson({ code: "42501", message: "No tiene permiso." }, 403),
    });
    const r = await manejar(pedido("POST", "/cargar-desde-qr", { cuerpo: cuerpo(contratista) }), ENV, s.deps);
    expect(r.status).toBe(403);
    expect((await r.json()) as Record<string, unknown>).toEqual({ error: "sin_permiso", detail: "No tiene permiso." });
    expect((await ENV.FOTOS.list({ prefix: `contratistas/${contratista}/` })).objects).toHaveLength(0);
  });
});

describe("PUT /cargar", () => {
  it("guarda la imagen y borra el carné anterior", async () => {
    const contratista = nuevoContratista();
    const anterior = await claveDe(contratista, PNG, "png");
    await ENV.FOTOS.put(anterior, PNG);
    const s = simular({ ...permisosRpc(true, true), [`${SUPABASE}/rest/v1/rpc/praind_registrar`]: rpcJson(anterior) });
    const r = await manejar(pedido("PUT", `/cargar/${contratista}`, { cuerpo: JPEG }), ENV, s.deps);
    expect(r.status).toBe(200);
    const nueva = await claveDe(contratista, JPEG, "jpg");
    expect(((await r.json()) as Record<string, unknown>).clave).toBe(nueva);
    expect(await ENV.FOTOS.head(nueva)).not.toBeNull();
    expect(await ENV.FOTOS.head(anterior)).toBeNull();
  });

  it("rechaza lo que no es PNG ni JPEG y lo que pasa de 10 MB", async () => {
    const contratista = nuevoContratista();
    const s = simular(permisosRpc(true, true));
    const texto = await manejar(pedido("PUT", `/cargar/${contratista}`, { cuerpo: "hola" }), ENV, s.deps);
    expect(texto.status).toBe(415);
    const grande = await manejar(
      pedido("PUT", `/cargar/${contratista}`, { cuerpo: PNG, cabeceras: { "Content-Length": String(11 * 1024 * 1024) } }),
      ENV,
      s.deps,
    );
    expect(grande.status).toBe(413);
  });
});

describe("DELETE /foto", () => {
  it("quita el carné y borra el objeto", async () => {
    const contratista = nuevoContratista();
    const clave = await claveDe(contratista, PNG, "png");
    await ENV.FOTOS.put(clave, PNG);
    const s = simular({ [`${SUPABASE}/rest/v1/rpc/praind_quitar`]: rpcJson(clave) });
    const r = await manejar(pedido("DELETE", `/foto/${contratista}`), ENV, s.deps);
    expect(r.status).toBe(200);
    expect(await ENV.FOTOS.head(clave)).toBeNull();
  });

  it("un equipo no puede quitar", async () => {
    const contratista = nuevoContratista();
    const s = simular({
      [`${SUPABASE}/rest/v1/rpc/praind_quitar`]: rpcJson({ code: "42501", message: "Sólo el panel puede quitar carnés PRAIND." }, 403),
    });
    const r = await manejar(pedido("DELETE", `/foto/${contratista}`), ENV, s.deps);
    expect(r.status).toBe(403);
  });
});

describe("CORS", () => {
  it("sólo responde al panel", async () => {
    const s = simular({});
    const ok = await manejar(
      new Request("https://praind.megabrisas.com/cargar-desde-qr", { method: "OPTIONS", headers: { Origin: PANEL } }),
      ENV,
      s.deps,
    );
    expect(ok.status).toBe(204);
    expect(ok.headers.get("Access-Control-Allow-Origin")).toBe(PANEL);
    const ajeno = await manejar(
      new Request("https://praind.megabrisas.com/cargar-desde-qr", { method: "OPTIONS", headers: { Origin: "https://ejemplo.com" } }),
      ENV,
      s.deps,
    );
    expect(ajeno.status).toBe(403);
    expect(ajeno.headers.get("Access-Control-Allow-Origin")).toBeNull();
  });
});

describe("lectura de la página", () => {
  it("decodifica entidades", () => {
    expect(decodificarEntidades("Mu&ntilde;oz &amp; Pe&#241;a &#xE9;")).toBe("Muñoz & Peña é");
    expect(decodificarEntidades("&desconocida;")).toBe("&desconocida;");
  });

  it("toma sólo el primer h2", async () => {
    const html = `<h2>Primera  Persona</h2><h2>Otra</h2><img src="${IMAGEN}">`;
    const { nombre, imagen } = await leerPagina(html, new URL(PAGINA));
    expect(nombre).toBe("Primera Persona");
    expect(imagen?.toString()).toBe(IMAGEN);
  });
});
