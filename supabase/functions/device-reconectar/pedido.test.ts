import { assertEquals } from "jsr:@std/assert@1";
import { leerPedidoReconexion } from "./pedido.ts";

Deno.test("normaliza el código y por defecto sólo consulta", () => {
  assertEquals(leerPedidoReconexion({ codigo: "k7qm-r4xt-2p" }), {
    codigo: "K7QMR4XT2P",
    confirmar: false,
    detalle: { pendientes_descartados: 0, confirmado_por: null },
  });
});

Deno.test("sin código, o con uno de otro largo, no hay pedido", () => {
  assertEquals(leerPedidoReconexion(null), null);
  assertEquals(leerPedidoReconexion({}), null);
  assertEquals(leerPedidoReconexion({ codigo: 12345 }), null);
  assertEquals(leerPedidoReconexion({ codigo: "K7QM-R4XT" }), null);
});

Deno.test("confirmar sólo con true; el detalle se acota y se recorta", () => {
  const pedido = leerPedidoReconexion({
    codigo: "K7QMR4XT2P",
    confirmar: true,
    pendientes_descartados: 3,
    confirmado_por: "  Ana Mora  ",
  });
  assertEquals(pedido?.confirmar, true);
  assertEquals(pedido?.detalle, { pendientes_descartados: 3, confirmado_por: "Ana Mora" });

  assertEquals(leerPedidoReconexion({ codigo: "K7QMR4XT2P", confirmar: "true" })?.confirmar, false);
  for (const raro of [-1, 2.5, "3", 10_000_000]) {
    assertEquals(
      leerPedidoReconexion({ codigo: "K7QMR4XT2P", pendientes_descartados: raro })?.detalle.pendientes_descartados,
      0,
    );
  }
  assertEquals(
    leerPedidoReconexion({ codigo: "K7QMR4XT2P", confirmado_por: "x".repeat(300) })?.detalle.confirmado_por?.length,
    120,
  );
});
