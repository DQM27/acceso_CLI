import { describe, expect, it } from "vitest";
import { contarDestinatarios, destinoDeSeleccion, textoAlcance, textoResultado } from "./Avisos.logica";

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
    expect(textoResultado({ destinatarios: 3, enviados: 3, fallidos: 0 })).toBe("Aviso enviado a 3 de 3 equipos.");
    expect(textoResultado({ destinatarios: 3, enviados: 2, fallidos: 1 })).toBe(
      "Aviso enviado a 2 de 3 equipos; 1 no se pudo enviar.",
    );
    expect(textoResultado({ destinatarios: 5, enviados: 3, fallidos: 2 })).toBe(
      "Aviso enviado a 3 de 5 equipos; 2 no se pudieron enviar.",
    );
    expect(textoResultado({ destinatarios: 0, enviados: 0, fallidos: 0 })).toBe(
      "No había equipos con notificaciones en ese destino.",
    );
  });
});
