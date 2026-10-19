import { assertEquals } from "jsr:@std/assert@1";
import { mensajeFcm } from "../_shared/fcm.ts";
import { resumirEnvio, validarPedido } from "./envio.ts";

const SITIO = "11111111-1111-4111-8111-111111111111";
const EQUIPO = "22222222-2222-4222-8222-222222222222";

Deno.test("acepta un aviso a todos, recortando y con tipo normal por defecto", () => {
  assertEquals(validarPedido({ titulo: "  Corte  ", cuerpo: " Hasta las 14 ", destino: { todos: true } }), {
    titulo: "Corte",
    cuerpo: "Hasta las 14",
    tipo: "normal",
    destino: { todos: true },
  });
});

Deno.test("acepta unidades y equipos, sin repetir ids", () => {
  const pedido = validarPedido({
    titulo: "T",
    cuerpo: "C",
    tipo: "emergente",
    destino: { sitio_ids: [SITIO, SITIO], dispositivo_ids: [EQUIPO] },
  });
  assertEquals(pedido, {
    titulo: "T",
    cuerpo: "C",
    tipo: "emergente",
    destino: { sitio_ids: [SITIO], dispositivo_ids: [EQUIPO] },
  });
});

Deno.test("rechaza lo que vino mal", () => {
  assertEquals(validarPedido(null), { error: "bad_request" });
  assertEquals(validarPedido({ titulo: "T", cuerpo: "  ", destino: { todos: true } }), { error: "bad_request" });
  assertEquals(validarPedido({ titulo: "T", cuerpo: "C", tipo: "urgente", destino: { todos: true } }), {
    error: "bad_request",
  });
  assertEquals(validarPedido({ titulo: "T", cuerpo: "C", destino: { sitio_ids: ["no-es-uuid"] } }), {
    error: "bad_request",
  });
  assertEquals(validarPedido({ titulo: "T", cuerpo: "C", destino: { sitio_ids: [], dispositivo_ids: [] } }), {
    error: "sin_destino",
  });
  assertEquals(validarPedido({ titulo: "T", cuerpo: "C" }), { error: "sin_destino" });
});

Deno.test("resume el envío: quién no lo recibió y qué tokens borrar", () => {
  const resumen = resumirEnvio(
    [
      { token: "t1", etiqueta: "Portería norte" },
      { token: "t2", etiqueta: "Celular B" },
      { token: "t3", etiqueta: "Celular A" },
      { token: "t4", etiqueta: "Portería sur" },
    ],
    ["enviado", "token_invalido", "error", "enviado"],
  );
  assertEquals(resumen, { enviados: 2, no_recibieron: ["Celular A", "Celular B"], tokens_invalidos: ["t2"] });
});

Deno.test("un resultado que falta cuenta como no recibido", () => {
  assertEquals(resumirEnvio([{ token: "t1", etiqueta: "Celular A" }], []), {
    enviados: 0,
    no_recibieron: ["Celular A"],
    tokens_invalidos: [],
  });
});

Deno.test("el mensaje lleva id y hora de envío para la bandeja del teléfono", () => {
  const mensaje = mensajeFcm("token", {
    id: "33333333-3333-4333-8333-333333333333",
    enviado_en: "2026-10-18T15:00:00.000Z",
    titulo: "T",
    cuerpo: "C",
    tipo: "emergente",
  });
  assertEquals(mensaje.message.data, {
    id: "33333333-3333-4333-8333-333333333333",
    enviado_en: "2026-10-18T15:00:00.000Z",
    titulo: "T",
    cuerpo: "C",
    tipo: "emergente",
  });
  assertEquals(mensaje.message.android.ttl, "3600s");
});
