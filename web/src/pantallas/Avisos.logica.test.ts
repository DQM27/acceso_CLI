import { describe, expect, it } from "vitest";
import {
  contarDestinatarios,
  destinoDeSeleccion,
  textoAccion,
  textoAlcance,
  textoDestino,
  textoEntrega,
  textoResultado,
} from "./Avisos.logica";

const EQUIPOS = [
  { id: "cel-a1", sitio_id: "unidad-a" },
  { id: "cel-a2", sitio_id: "unidad-a" },
  { id: "cel-b1", sitio_id: "unidad-b" },
  { id: "pc-b2", sitio_id: "unidad-b" },
];
const CON_NOTIFICACIONES = new Set(["cel-a1", "cel-a2", "cel-b1"]);

function seleccion(todos: boolean, sitios: string[] = [], equipos: string[] = []) {
  return { todos, sitios: new Set(sitios), equipos: new Set(equipos) };
}

describe("destinoDeSeleccion", () => {
  it("todos", () => {
    expect(destinoDeSeleccion(seleccion(true), EQUIPOS)).toEqual({ todos: true });
  });

  it("unidades completas y equipos sueltos", () => {
    expect(destinoDeSeleccion(seleccion(false, ["unidad-a"], ["cel-b1"]), EQUIPOS)).toEqual({
      sitio_ids: ["unidad-a"],
      dispositivo_ids: ["cel-b1"],
    });
  });

  it("un equipo de una unidad ya marcada completa no se repite", () => {
    expect(destinoDeSeleccion(seleccion(false, ["unidad-a"], ["cel-a1", "cel-b1"]), EQUIPOS)).toEqual({
      sitio_ids: ["unidad-a"],
      dispositivo_ids: ["cel-b1"],
    });
  });

  it("sin nadie elegido no hay destino", () => {
    expect(destinoDeSeleccion(seleccion(false), EQUIPOS)).toBeNull();
    // Un equipo que ya no existe (se retiró mientras la pantalla estaba abierta) no cuenta.
    expect(destinoDeSeleccion(seleccion(false, [], ["equipo-viejo"]), EQUIPOS)).toBeNull();
  });
});

describe("contarDestinatarios", () => {
  it("sólo cuenta equipos con notificaciones", () => {
    expect(contarDestinatarios(seleccion(true), EQUIPOS, CON_NOTIFICACIONES)).toBe(3);
    expect(contarDestinatarios(seleccion(false, ["unidad-b"]), EQUIPOS, CON_NOTIFICACIONES)).toBe(1);
    expect(contarDestinatarios(seleccion(false, [], ["cel-a2", "pc-b2"]), EQUIPOS, CON_NOTIFICACIONES)).toBe(1);
    expect(contarDestinatarios(seleccion(false), EQUIPOS, CON_NOTIFICACIONES)).toBe(0);
  });

  it("no cuenta dos veces un equipo marcado suelto dentro de su unidad completa", () => {
    expect(contarDestinatarios(seleccion(false, ["unidad-a"], ["cel-a1"]), EQUIPOS, CON_NOTIFICACIONES)).toBe(2);
  });
});

describe("textos", () => {
  it("alcance en singular, plural y cero", () => {
    expect(textoAlcance(0)).toBe("No llegará a ningún equipo");
    expect(textoAlcance(1)).toBe("Llegará a 1 equipo");
    expect(textoAlcance(4)).toBe("Llegará a 4 equipos");
  });

  it("resultado del envío", () => {
    const sinNombres = { no_recibieron: [] };
    expect(textoResultado({ destinatarios: 3, enviados: 3, fallidos: 0, ...sinNombres })).toBe(
      "Aviso enviado a 3 de 3 equipos.",
    );
    expect(textoResultado({ destinatarios: 3, enviados: 2, fallidos: 1, ...sinNombres })).toBe(
      "Aviso enviado a 2 de 3 equipos; 1 no se pudo enviar.",
    );
    expect(textoResultado({ destinatarios: 5, enviados: 3, fallidos: 2, ...sinNombres })).toBe(
      "Aviso enviado a 3 de 5 equipos; 2 no se pudieron enviar.",
    );
    expect(textoResultado({ destinatarios: 0, enviados: 0, fallidos: 0, ...sinNombres })).toBe(
      "No había equipos con notificaciones en ese destino.",
    );
  });

  it("nombra a los equipos a los que no llegó", () => {
    expect(
      textoResultado({ destinatarios: 4, enviados: 2, fallidos: 2, no_recibieron: ["Celular A", "Portería sur"] }),
    ).toBe("Aviso enviado a 2 de 4 equipos; no llegó a Celular A, Portería sur.");
  });

  it("entrega en el historial", () => {
    expect(textoEntrega({ destinatarios: 3, enviados: 3 })).toBe("Llegó a 3 de 3");
    expect(textoEntrega({ destinatarios: 2, enviados: 0 })).toBe("No llegó a ninguno (0 de 2)");
    expect(textoEntrega({ destinatarios: 0, enviados: 0 })).toBe("Sin destinatarios");
  });
});

describe("textoAccion", () => {
  it("quién lo archivó o borró, desde qué celular y cuándo", () => {
    const base = { usuario: "ANA PÉREZ", dispositivo_etiqueta: "Celular A1" };
    expect(textoAccion({ ...base, accion: "borrado" }, "18/10/2026 14:32")).toBe(
      "Borrado por ANA PÉREZ en Celular A1 (18/10/2026 14:32)",
    );
    expect(textoAccion({ ...base, accion: "archivado" }, "18/10/2026 14:30")).toBe(
      "Archivado por ANA PÉREZ en Celular A1 (18/10/2026 14:30)",
    );
  });
});

describe("textoDestino", () => {
  const SITIOS = new Map([["unidad-a", "Unidad A"]]);
  const NOMBRES_EQUIPOS = new Map([["cel-b1", "Celular B1"]]);

  it("todos", () => {
    expect(textoDestino({ todos: true }, SITIOS, NOMBRES_EQUIPOS)).toBe("Todos los equipos");
  });

  it("unidades y equipos con nombre", () => {
    expect(
      textoDestino({ sitio_ids: ["unidad-a"], dispositivo_ids: ["cel-b1"] }, SITIOS, NOMBRES_EQUIPOS),
    ).toBe("Unidad A, Celular B1");
  });

  it("lo que ya no existe se cuenta aparte", () => {
    expect(
      textoDestino({ sitio_ids: ["unidad-a"], dispositivo_ids: ["retirado"] }, SITIOS, NOMBRES_EQUIPOS),
    ).toBe("Unidad A, 1 que ya no existe");
    expect(textoDestino({ sitio_ids: ["x", "y"], dispositivo_ids: [] }, SITIOS, NOMBRES_EQUIPOS)).toBe(
      "2 que ya no existen",
    );
  });
});
