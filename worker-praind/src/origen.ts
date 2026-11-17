// Descarga del carné a partir del enlace de su QR.
//
// El QR lleva a `https://prd.pw/<código>`, que redirige a una página pública
// de safety-record.com con el nombre de la persona (primer <h2>) y una sola
// imagen: el carné original en media-chirho.safety-record.com.
//
// El Worker pide una dirección que le dicta el cliente, así que sólo sigue
// anfitriones conocidos, valida cada redirección y limita los tamaños. No
// se recorren ni se adivinan códigos: sólo se consulta el enlace recibido.

import { ErrorHttp } from "./errores";
import { leerConLimite, MAXIMO_IMAGEN, tipoPorFirma } from "./imagen";
import type { Dependencias, TipoImagen } from "./tipos";

const ANFITRIONES_PAGINA = new Set(["prd.pw", "safety-record.com", "www.safety-record.com"]);
const ANFITRION_IMAGEN = "media-chirho.safety-record.com";
const MAXIMO_PAGINA = 64 * 1024;
const MAXIMO_REDIRECCIONES = 3;
const MAXIMO_ENLACE = 500;

export interface CarneDescargado {
  nombre: string | null;
  bytes: Uint8Array;
  tipo: TipoImagen;
}

function enlaceInvalido(): ErrorHttp {
  return new ErrorHttp(400, "enlace_invalido", "El enlace no es de un gafete PRAIND.");
}

function falloOrigen(detalle: string): ErrorHttp {
  return new ErrorHttp(502, "origen_no_disponible", `No se pudo obtener el gafete PRAIND de la página del QR (${detalle}).`);
}

// Un gafete PRAIND borrado en el sistema PRAIND deja el QR apuntando a una
// página que ya no existe. Se avisa claro, y no como "el sitio falló", en
// los casos en que eso se nota: página o imagen 404/410, o una página que
// abre pero no trae ni el nombre ni la imagen (la de "no existe").
export const MENSAJE_CARNE_NO_EXISTE =
  "Ese gafete PRAIND ya no existe en el sistema PRAIND: pudo haberse borrado. Pida a la persona su gafete PRAIND actualizado.";

function carneNoExiste(): ErrorHttp {
  return new ErrorHttp(404, "carne_no_encontrado", MENSAJE_CARNE_NO_EXISTE);
}

// El acortador (`prd.pw`) responde con error de servidor a un código que
// no conoce (comprobado el 2026-11-17: un código inventado da 502). Puede
// ser un gafete borrado o el acortador caído: no se sabe cuál, y el texto
// lo dice.
export const MENSAJE_ENLACE_SIN_GAFETE =
  "El QR no lleva a ningún gafete PRAIND: puede que se haya borrado en el sistema PRAIND. Si pasa con todos los gafetes, el sistema PRAIND puede estar fuera de servicio.";

const ACORTADOR = "prd.pw";

function esNoExiste(estado: number): boolean {
  return estado === 404 || estado === 410;
}

/** Valida y normaliza el enlace leído del QR. */
export function validarEnlace(enlace: unknown): URL {
  if (typeof enlace !== "string") throw enlaceInvalido();
  const texto = enlace.trim();
  if (!texto || texto.length > MAXIMO_ENLACE) throw enlaceInvalido();
  let url: URL;
  try {
    url = new URL(texto);
  } catch {
    throw enlaceInvalido();
  }
  if (!esPermitida(url, ANFITRIONES_PAGINA)) throw enlaceInvalido();
  return url;
}

function esPermitida(url: URL, anfitriones: Set<string>): boolean {
  return (
    url.protocol === "https:" &&
    anfitriones.has(url.hostname.toLowerCase()) &&
    !url.username &&
    !url.password &&
    (url.port === "" || url.port === "443")
  );
}

async function pedir(deps: Dependencias, url: URL): Promise<Response> {
  try {
    return await deps.fetch(url.toString(), {
      redirect: "manual",
      headers: { "User-Agent": "Mozilla/5.0 (compatible; carnes-praind)", Accept: "*/*" },
    });
  } catch {
    throw falloOrigen("sin respuesta");
  }
}

async function pedirPagina(deps: Dependencias, inicio: URL): Promise<{ respuesta: Response; url: URL }> {
  let url = inicio;
  for (let saltos = 0; ; saltos++) {
    const respuesta = await pedir(deps, url);
    if (respuesta.status >= 300 && respuesta.status < 400) {
      const destino = respuesta.headers.get("Location");
      await respuesta.body?.cancel();
      if (!destino || saltos >= MAXIMO_REDIRECCIONES) throw falloOrigen("demasiadas redirecciones");
      let siguiente: URL;
      try {
        siguiente = new URL(destino, url);
      } catch {
        throw falloOrigen("redirección inválida");
      }
      if (!esPermitida(siguiente, ANFITRIONES_PAGINA)) throw falloOrigen("redirección a un sitio no permitido");
      url = siguiente;
      continue;
    }
    if (esNoExiste(respuesta.status)) {
      await respuesta.body?.cancel();
      throw carneNoExiste();
    }
    if (respuesta.status >= 500 && url.hostname.toLowerCase() === ACORTADOR) {
      await respuesta.body?.cancel();
      throw new ErrorHttp(404, "carne_no_encontrado", MENSAJE_ENLACE_SIN_GAFETE);
    }
    if (!respuesta.ok) {
      await respuesta.body?.cancel();
      throw falloOrigen(`estado ${respuesta.status}`);
    }
    return { respuesta, url };
  }
}

const ENTIDADES: Record<string, string> = {
  amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " ",
  aacute: "á", eacute: "é", iacute: "í", oacute: "ó", uacute: "ú",
  Aacute: "Á", Eacute: "É", Iacute: "Í", Oacute: "Ó", Uacute: "Ú",
  ntilde: "ñ", Ntilde: "Ñ", uuml: "ü", Uuml: "Ü",
};

/** El texto de HTMLRewriter llega sin decodificar. */
export function decodificarEntidades(texto: string): string {
  return texto.replace(/&(#x[0-9a-f]+|#[0-9]+|[a-z]+);/gi, (original, cuerpo: string) => {
    if (cuerpo[0] === "#") {
      const codigo = cuerpo[1] === "x" || cuerpo[1] === "X" ? parseInt(cuerpo.slice(2), 16) : parseInt(cuerpo.slice(1), 10);
      return Number.isFinite(codigo) && codigo > 0 && codigo <= 0x10ffff ? String.fromCodePoint(codigo) : original;
    }
    return ENTIDADES[cuerpo] ?? original;
  });
}

/** Extrae el nombre (primer <h2>) y la imagen del carné de la página. */
export async function leerPagina(html: string, base: URL): Promise<{ nombre: string | null; imagen: URL | null }> {
  let h2Vistos = 0;
  let nombre = "";
  let imagen: URL | null = null;

  const reescritor = new HTMLRewriter()
    .on("h2", {
      element() {
        h2Vistos++;
      },
      text(trozo) {
        if (h2Vistos === 1) nombre += trozo.text;
      },
    })
    .on("img", {
      element(elemento) {
        if (imagen) return;
        const src = elemento.getAttribute("src");
        if (!src) return;
        try {
          const url = new URL(decodificarEntidades(src), base);
          if (esPermitida(url, new Set([ANFITRION_IMAGEN]))) imagen = url;
        } catch {
          // src inválido: se ignora.
        }
      },
    });

  await reescritor.transform(new Response(html)).arrayBuffer();
  const limpio = decodificarEntidades(nombre).replace(/\s+/g, " ").trim();
  return { nombre: limpio || null, imagen };
}

export async function descargarCarne(deps: Dependencias, enlace: URL): Promise<CarneDescargado> {
  const pagina = await pedirPagina(deps, enlace);
  const html = new TextDecoder().decode(
    await leerConLimite(pagina.respuesta.body, MAXIMO_PAGINA, falloOrigen("la página es demasiado grande")),
  );
  const { nombre, imagen } = await leerPagina(html, pagina.url);
  // Sin nombre ni imagen es la página de "no existe"; con nombre pero sin
  // imagen válida, algo cambió en el sitio.
  if (!imagen && !nombre) throw carneNoExiste();
  if (!imagen) throw falloOrigen("la página no trae la imagen del gafete PRAIND");

  const respuesta = await pedir(deps, imagen);
  if (esNoExiste(respuesta.status)) {
    await respuesta.body?.cancel();
    throw carneNoExiste();
  }
  if (!respuesta.ok) {
    await respuesta.body?.cancel();
    throw falloOrigen(`la imagen respondió ${respuesta.status}`);
  }
  const bytes = await leerConLimite(
    respuesta.body,
    MAXIMO_IMAGEN,
    new ErrorHttp(413, "imagen_grande", "La imagen del gafete PRAIND pasa de 10 MB."),
  );
  const tipo = tipoPorFirma(bytes);
  if (!tipo) throw falloOrigen("el archivo no es una imagen PNG ni JPEG");
  return { nombre, bytes, tipo };
}
