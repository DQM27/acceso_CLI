import { assert, assertEquals } from "jsr:@std/assert@1";
import type { ResultadoFirma } from "../_shared/dispositivos.ts";
import {
  type Decision,
  decidirAutenticacion,
  ESPERAS_REINTENTO_MS,
  type FilaEquipo,
  type Puertos,
  respuestaDeRechazo,
} from "./autenticacion.ts";

const CLAVE = { kty: "EC", crv: "P-256", x: "x", y: "y" } as const;

const VIGENTE: FilaEquipo = {
  id: "ecbef05c-a188-439a-be85-4758502bf2c6",
  sitio_id: "b7da3917-c838-4d44-942e-4d438e276006",
  tipo: "mobile",
  clave_huella: "Dg10ghjkxO3nbhNxl9cz2CzNWc5iZqp_VpOMuKXWPKo",
  clave_publica_jwk: CLAVE,
  identificador_hardware: null,
  etiqueta: "Celular Brisas",
  sitios: { nombre: "Brisas" },
  revoked_at: null,
};
const RETIRADO: FilaEquipo = { ...VIGENTE, revoked_at: "2026-10-26T12:10:49Z" };
const SIN_CLAVE: FilaEquipo = { ...VIGENTE, clave_publica_jwk: null };

/** Un 401 de PostgREST como el del incidente, lanzado por el puerto. */
const FALLA_DE_LA_BASE = () => new Error("PGRST301: JWT inválido (respuesta 401 de la base)");

interface Escenario {
  huella?: string | null;
  /** Lo que devuelve cada lectura de la base, en orden; una función lanza. */
  lecturas: Array<FilaEquipo | null | (() => Error)>;
  firma?: ResultadoFirma | (() => Error);
  registrarFalla?: boolean;
}

function puertosDe(escenario: Escenario) {
  const registro = { lecturas: 0, firmasVerificadas: 0, eventos: [] as string[], esperas: [] as number[] };
  const puertos: Puertos = {
    huellaDe: () => (escenario.huella === undefined ? VIGENTE.clave_huella : escenario.huella),
    buscarPorHuella: () => {
      const indice = Math.min(registro.lecturas, escenario.lecturas.length - 1);
      registro.lecturas += 1;
      const lectura = escenario.lecturas[indice];
      return typeof lectura === "function" ? Promise.reject(lectura()) : Promise.resolve(lectura);
    },
    verificarFirma: () => {
      registro.firmasVerificadas += 1;
      const firma = escenario.firma ?? "valida";
      return typeof firma === "function" ? Promise.reject(firma()) : Promise.resolve(firma);
    },
    registrarFirmaInvalida: (id) => {
      registro.eventos.push(id);
      return escenario.registrarFalla ? Promise.reject(new Error("insert falló")) : Promise.resolve();
    },
    esperar: (ms) => {
      registro.esperas.push(ms);
      return Promise.resolve();
    },
  };
  return { puertos, registro };
}

function estadoHttp(decision: Decision): number {
  return decision.tipo === "vigente" ? 200 : respuestaDeRechazo(decision).estado;
}

// ---- El incidente del 2026-10-26 ----

Deno.test("incidente: un 401 pasajero de la base se reintenta y el equipo sigue vigente", async () => {
  const { puertos, registro } = puertosDe({ lecturas: [FALLA_DE_LA_BASE, VIGENTE] });

  const decision = await decidirAutenticacion("asercion", puertos);

  assertEquals(decision, { tipo: "vigente", equipo: VIGENTE });
  assertEquals(registro.lecturas, 2);
  assertEquals(registro.eventos, [], "una falla de la base no es una firma inválida");
});

Deno.test("incidente: si la base nunca responde bien es 503, jamás 401 ni 403", async () => {
  const { puertos, registro } = puertosDe({ lecturas: [FALLA_DE_LA_BASE] });

  const decision = await decidirAutenticacion("asercion", puertos);

  assertEquals(decision.tipo, "no_verificable");
  assertEquals(estadoHttp(decision), 503);
  assertEquals(registro.lecturas, ESPERAS_REINTENTO_MS.length + 1, "tres intentos");
  assertEquals(registro.esperas, ESPERAS_REINTENTO_MS);
  assertEquals(registro.firmasVerificadas, 0);
});

Deno.test("la base que falla dos veces y responde a la tercera no deja rastro en la respuesta", async () => {
  const { puertos } = puertosDe({ lecturas: [FALLA_DE_LA_BASE, FALLA_DE_LA_BASE, VIGENTE] });
  assertEquals((await decidirAutenticacion("asercion", puertos)).tipo, "vigente");
});

// ---- Retiro: sólo con prueba ----

Deno.test("retirado con la firma de su propia clave: 403 dispositivo_retirado con la fecha", async () => {
  const { puertos } = puertosDe({ lecturas: [RETIRADO] });

  const decision = await decidirAutenticacion("asercion", puertos);

  assertEquals(decision, { tipo: "retirado", dispositivo_id: RETIRADO.id, retirado_en: RETIRADO.revoked_at! });
  assertEquals(respuestaDeRechazo(decision as Exclude<Decision, { tipo: "vigente" }>), {
    estado: 403,
    cuerpo: { error: "dispositivo_retirado", retirado_en: RETIRADO.revoked_at! },
  });
});

Deno.test("una fila retirada con firma ajena no confirma el retiro: 401 y queda el evento", async () => {
  const { puertos, registro } = puertosDe({ lecturas: [RETIRADO], firma: "firma_invalida" });

  const decision = await decidirAutenticacion("asercion", puertos);

  assertEquals(decision, { tipo: "no_reconocido", motivo: "firma_invalida" });
  assertEquals(registro.eventos, [RETIRADO.id]);
});

Deno.test("una fila retirada con el desafío vencido pide otro desafío, no confirma el retiro", async () => {
  const { puertos } = puertosDe({ lecturas: [RETIRADO], firma: "desafio_invalido" });
  const decision = await decidirAutenticacion("asercion", puertos);
  assertEquals(decision, { tipo: "desafio_invalido" });
  assertEquals(estadoHttp(decision), 400);
});

Deno.test("un equipo que no existe es 401 equipo_desconocido, nunca retirado", async () => {
  const { puertos, registro } = puertosDe({ lecturas: [null] });
  const decision = await decidirAutenticacion("asercion", puertos);
  assertEquals(decision, { tipo: "no_reconocido", motivo: "equipo_desconocido" });
  assertEquals(registro.firmasVerificadas, 0);
});

Deno.test("una fila sin clave registrada no se puede verificar: 401 equipo_desconocido", async () => {
  const { puertos } = puertosDe({ lecturas: [SIN_CLAVE] });
  assertEquals(await decidirAutenticacion("asercion", puertos), {
    tipo: "no_reconocido",
    motivo: "equipo_desconocido",
  });
});

Deno.test("una aserción sin huella legible no consulta la base", async () => {
  const { puertos, registro } = puertosDe({ huella: null, lecturas: [VIGENTE] });
  assertEquals(await decidirAutenticacion("basura", puertos), {
    tipo: "no_reconocido",
    motivo: "asercion_ilegible",
  });
  assertEquals(registro.lecturas, 0);
});

// ---- El servidor que no puede verificar no dice "no" ----

Deno.test("si el servidor no puede verificar la firma (sus claves) es 503", async () => {
  const { puertos, registro } = puertosDe({ lecturas: [VIGENTE], firma: () => new Error("DEVICE_SIGNING_KEY") });
  const decision = await decidirAutenticacion("asercion", puertos);
  assertEquals(decision.tipo, "no_verificable");
  assertEquals(estadoHttp(decision), 503);
  assertEquals(registro.eventos, [], "no es culpa del equipo");
});

Deno.test("si no se puede anotar la firma inválida, la respuesta sigue siendo 401", async () => {
  const { puertos } = puertosDe({ lecturas: [VIGENTE], firma: "firma_invalida", registrarFalla: true });
  assertEquals(estadoHttp(await decidirAutenticacion("asercion", puertos)), 401);
});

Deno.test("vigente con el desafío vencido: 400 desafio_invalido para que pida otro", async () => {
  const { puertos } = puertosDe({ lecturas: [VIGENTE], firma: "desafio_invalido" });
  assertEquals(estadoHttp(await decidirAutenticacion("asercion", puertos)), 400);
});

// ---- Propiedades sobre todas las combinaciones ----

const FILAS: Array<[string, FilaEquipo | null]> = [
  ["vigente", VIGENTE],
  ["retirado", RETIRADO],
  ["sin_clave", SIN_CLAVE],
  ["inexistente", null],
];
const FIRMAS: Array<[string, ResultadoFirma | (() => Error)]> = [
  ["valida", "valida"],
  ["firma_invalida", "firma_invalida"],
  ["desafio_invalido", "desafio_invalido"],
  ["servidor_sin_claves", () => new Error("sin claves")],
];
const BASES: Array<[string, (fila: FilaEquipo | null) => Escenario["lecturas"]]> = [
  ["responde", (fila) => [fila]],
  ["falla_y_responde", (fila) => [FALLA_DE_LA_BASE, fila]],
  ["caida", () => [FALLA_DE_LA_BASE]],
];

Deno.test("403 sale si y sólo si la fila está retirada, la firma es suya y la base respondió", async () => {
  for (const [nombreFila, fila] of FILAS) {
    for (const [nombreFirma, firma] of FIRMAS) {
      for (const [nombreBase, lecturas] of BASES) {
        const { puertos } = puertosDe({ lecturas: lecturas(fila), firma });
        const estado = estadoHttp(await decidirAutenticacion("asercion", puertos));
        const esperaRetiro = nombreFila === "retirado" && nombreFirma === "valida" && nombreBase !== "caida";
        assertEquals(
          estado === 403,
          esperaRetiro,
          `fila=${nombreFila} firma=${nombreFirma} base=${nombreBase} respondió ${estado}`,
        );
      }
    }
  }
});

Deno.test("ninguna falla de infraestructura produce 401 ni 403: siempre 503", async () => {
  for (const [nombreFila, fila] of FILAS) {
    // La base caída (sea cual sea la fila que hubiera devuelto) y, si hay
    // clave para verificar, el servidor sin poder verificar.
    const casos: Array<[string, Escenario]> = [["base_caida", { lecturas: [FALLA_DE_LA_BASE] }]];
    if (fila?.clave_publica_jwk) {
      casos.push(["servidor_sin_claves", { lecturas: [fila], firma: () => new Error("sin claves") }]);
    }
    for (const [nombreCaso, caso] of casos) {
      const { puertos } = puertosDe(caso);
      const estado = estadoHttp(await decidirAutenticacion("asercion", puertos));
      assertEquals(estado, 503, `fila=${nombreFila} caso=${nombreCaso}`);
    }
  }
});

Deno.test("200 sale si y sólo si la fila está vigente y la firma es válida", async () => {
  for (const [nombreFila, fila] of FILAS) {
    for (const [nombreFirma, firma] of FIRMAS) {
      for (const [nombreBase, lecturas] of BASES) {
        const { puertos } = puertosDe({ lecturas: lecturas(fila), firma });
        const decision = await decidirAutenticacion("asercion", puertos);
        const esperaVigente = nombreFila === "vigente" && nombreFirma === "valida" && nombreBase !== "caida";
        assertEquals(decision.tipo === "vigente", esperaVigente, `fila=${nombreFila} firma=${nombreFirma} base=${nombreBase}`);
      }
    }
  }
});

Deno.test("cada rechazo tiene su código y ninguno es invalid_credentials (el que las apps viejas tomaban como retiro)", () => {
  const rechazos: Array<Exclude<Decision, { tipo: "vigente" }>> = [
    { tipo: "retirado", dispositivo_id: "d", retirado_en: "t" },
    { tipo: "no_reconocido", motivo: "asercion_ilegible" },
    { tipo: "no_reconocido", motivo: "equipo_desconocido" },
    { tipo: "no_reconocido", motivo: "firma_invalida" },
    { tipo: "desafio_invalido" },
    { tipo: "no_verificable", detalle: "x" },
  ];
  const esperados = [403, 401, 401, 401, 400, 503];
  rechazos.forEach((rechazo, i) => {
    const { estado, cuerpo } = respuestaDeRechazo(rechazo);
    assertEquals(estado, esperados[i]);
    assert(cuerpo.error && cuerpo.error !== "invalid_credentials");
  });
  assertEquals(respuestaDeRechazo({ tipo: "no_verificable", detalle: "secreto interno" }).cuerpo, {
    error: "no_verificable",
  }, "el detalle interno no viaja al equipo");
});
