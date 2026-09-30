import { describe, expect, it } from "vitest";
import { COLUMNAS_FILTRABLES, expresionesDeFiltros, plegarTexto } from "./historialFiltros";

describe("plegarTexto", () => {
  it("quita tildes, pasa a minúsculas y trata la ñ como n (igual que unaccent en la base)", () => {
    expect(plegarTexto("José Ñandú")).toBe("jose nandu");
  });
});

describe("expresionesDeFiltros -- texto", () => {
  const filtro = (type: string, filter?: string) => ({ usuario_entrada_nombre: { filterType: "text", type, filter } });

  it("contiene: cada palabra debe aparecer, sin tildes ni mayúsculas", () => {
    expect(expresionesDeFiltros(filtro("contains", "Carlos Sá"))).toEqual([
      'and(usuario_entrada_p.ilike."*carlos*",usuario_entrada_p.ilike."*sa*")',
    ]);
    expect(expresionesDeFiltros(filtro("contains", "Pérez"))).toEqual(['usuario_entrada_p.ilike."*perez*"']);
  });

  it("igual, distinto, empieza con, termina con y no contiene", () => {
    expect(expresionesDeFiltros(filtro("equals", "Ana"))).toEqual(['usuario_entrada_p.eq."ana"']);
    expect(expresionesDeFiltros(filtro("notEqual", "Ana"))).toEqual(['or(usuario_entrada_p.is.null,usuario_entrada_p.neq."ana")']);
    expect(expresionesDeFiltros(filtro("startsWith", "An"))).toEqual(['usuario_entrada_p.ilike."an*"']);
    expect(expresionesDeFiltros(filtro("endsWith", "na"))).toEqual(['usuario_entrada_p.ilike."*na"']);
    expect(expresionesDeFiltros(filtro("notContains", "an"))).toEqual([
      'or(usuario_entrada_p.is.null,usuario_entrada_p.not.ilike."*an*")',
    ]);
  });

  it("vacío y no vacío", () => {
    expect(expresionesDeFiltros(filtro("blank"))).toEqual(['or(usuario_entrada_p.is.null,usuario_entrada_p.eq."")']);
    expect(expresionesDeFiltros(filtro("notBlank"))).toEqual(['and(usuario_entrada_p.not.is.null,usuario_entrada_p.neq."")']);
  });

  it("escapa % _ \\ y * para like, y comillas para la sintaxis de PostgREST", () => {
    expect(expresionesDeFiltros(filtro("contains", "100%_x"))).toEqual(['usuario_entrada_p.ilike."*100\\\\%\\\\_x*"']);
    expect(expresionesDeFiltros(filtro("equals", 'a,"b")'))).toEqual(['usuario_entrada_p.eq."a,\\"b\\")"']);
  });

  it("un filtro sin texto no genera condición", () => {
    expect(expresionesDeFiltros(filtro("contains", ""))).toEqual([]);
    expect(expresionesDeFiltros(filtro("contains", "   "))).toEqual([]);
  });

  it("usa la columna plegada de cada columna de la grilla", () => {
    expect(COLUMNAS_FILTRABLES.contratista_cedula.columna).toBe("cedula_p");
    expect(COLUMNAS_FILTRABLES.tipo_ingreso.columna).toBe("tipo_p");
    expect(COLUMNAS_FILTRABLES.hora_salida.columna).toBe("hora_salida_txt");
  });
});

describe("expresionesDeFiltros -- prefiltro con índice (texto_busqueda)", () => {
  const filtro = (type: string, filter?: string) => ({ contratista_nombre: { filterType: "text", type, filter } });

  it("cédula, nombre y empresa suman una condición por palabra contra texto_busqueda", () => {
    expect(expresionesDeFiltros(filtro("contains", "Carlos Sá"))).toEqual([
      'and(nombre_p.ilike."*carlos*",nombre_p.ilike."*sa*")',
      'texto_busqueda.like."*carlos*"',
      'texto_busqueda.like."*sa*"',
    ]);
    expect(
      expresionesDeFiltros({ contratista_cedula: { filterType: "text", type: "startsWith", filter: "1-05" } }),
    ).toEqual(['cedula_p.ilike."1-05*"', 'texto_busqueda.like."*1-05*"']);
    expect(expresionesDeFiltros({ empresa_nombre: { filterType: "text", type: "equals", filter: "BAC" } })).toEqual([
      'empresa_p.eq."bac"',
      'texto_busqueda.like."*bac*"',
    ]);
  });

  it("escapa los comodines de like igual que la condición de la columna", () => {
    expect(expresionesDeFiltros(filtro("endsWith", "100%"))).toEqual([
      'nombre_p.ilike."*100\\\\%"',
      'texto_busqueda.like."*100\\\\%*"',
    ]);
  });

  it("no se agrega cuando podría descartar filas que la columna acepta", () => {
    // Negaciones y vacíos: el valor no tiene por qué aparecer en el texto.
    for (const tipo of ["notEqual", "notContains", "blank", "notBlank"]) {
      expect(expresionesDeFiltros(filtro(tipo, "ana")).join()).not.toContain("texto_busqueda");
    }
    // Dos condiciones unidas por OR: basta con una, el prefiltro exigiría ambas.
    const dosCondiciones = {
      contratista_nombre: {
        filterType: "text",
        operator: "OR",
        conditions: [
          { type: "equals", filter: "ana" },
          { type: "equals", filter: "luis" },
        ],
      },
    };
    expect(expresionesDeFiltros(dosCondiciones).join()).not.toContain("texto_busqueda");
    // Columnas que no forman parte de texto_busqueda.
    expect(
      expresionesDeFiltros({ sitio_nombre: { filterType: "text", type: "contains", filter: "brisas" } }),
    ).toEqual(['unidad_p.ilike."*brisas*"']);
  });
});

describe("expresionesDeFiltros -- número", () => {
  const filtro = (type: string, filter?: number, filterTo?: number) => ({
    gafete_numero: { filterType: "number", type, filter, filterTo },
  });

  it("comparaciones simples", () => {
    expect(expresionesDeFiltros(filtro("equals", 12))).toEqual(["gafete_numero.eq.12"]);
    expect(expresionesDeFiltros(filtro("greaterThan", 12))).toEqual(["gafete_numero.gt.12"]);
    expect(expresionesDeFiltros(filtro("greaterThanOrEqual", 12))).toEqual(["gafete_numero.gte.12"]);
    expect(expresionesDeFiltros(filtro("lessThan", 12))).toEqual(["gafete_numero.lt.12"]);
    expect(expresionesDeFiltros(filtro("lessThanOrEqual", 12))).toEqual(["gafete_numero.lte.12"]);
    expect(expresionesDeFiltros(filtro("notEqual", 12))).toEqual(["or(gafete_numero.is.null,gafete_numero.neq.12)"]);
  });

  it("entre (extremos excluidos como AG Grid), vacío y no vacío", () => {
    expect(expresionesDeFiltros(filtro("inRange", 5, 9))).toEqual(["and(gafete_numero.gt.5,gafete_numero.lt.9)"]);
    expect(expresionesDeFiltros(filtro("blank"))).toEqual(["gafete_numero.is.null"]);
    expect(expresionesDeFiltros(filtro("notBlank"))).toEqual(["gafete_numero.not.is.null"]);
  });

  it("sin número válido no genera condición", () => {
    expect(expresionesDeFiltros(filtro("equals"))).toEqual([]);
    expect(expresionesDeFiltros(filtro("inRange", 5))).toEqual([]);
  });
});

describe("expresionesDeFiltros -- fecha (día calendario de Costa Rica)", () => {
  const filtro = (type: string, dateFrom?: string, dateTo?: string) => ({
    fecha_ingreso: { filterType: "date", type, dateFrom, dateTo },
  });

  it("igual: todo ese día en hora de Costa Rica", () => {
    expect(expresionesDeFiltros(filtro("equals", "2026-09-09 00:00:00"))).toEqual([
      'and(hora_entrada.gte."2026-09-09T00:00:00-06:00",hora_entrada.lt."2026-09-10T00:00:00-06:00")',
    ]);
  });

  it("antes, después y distinto", () => {
    expect(expresionesDeFiltros(filtro("lessThan", "2026-09-30 00:00:00"))).toEqual([
      'hora_entrada.lt."2026-09-30T00:00:00-06:00"',
    ]);
    expect(expresionesDeFiltros(filtro("greaterThan", "2026-09-30 00:00:00"))).toEqual([
      'hora_entrada.gte."2026-10-01T00:00:00-06:00"',
    ]);
    expect(expresionesDeFiltros(filtro("notEqual", "2026-09-09 00:00:00"))).toEqual([
      'or(hora_entrada.lt."2026-09-09T00:00:00-06:00",hora_entrada.gte."2026-09-10T00:00:00-06:00")',
    ]);
  });

  it("entre: después del primer día y antes del último", () => {
    expect(expresionesDeFiltros(filtro("inRange", "2026-09-01 00:00:00", "2026-09-10 00:00:00"))).toEqual([
      'and(hora_entrada.gte."2026-09-02T00:00:00-06:00",hora_entrada.lt."2026-09-10T00:00:00-06:00")',
    ]);
  });

  it("vacío en la fecha de salida es 'sigue adentro'", () => {
    expect(expresionesDeFiltros({ fecha_salida: { filterType: "date", type: "blank" } })).toEqual([
      "hora_salida.is.null",
    ]);
    expect(expresionesDeFiltros({ fecha_salida: { filterType: "date", type: "notBlank" } })).toEqual([
      "hora_salida.not.is.null",
    ]);
  });

  it("sin fecha válida no genera condición", () => {
    expect(expresionesDeFiltros(filtro("equals"))).toEqual([]);
    expect(expresionesDeFiltros(filtro("equals", "no es fecha"))).toEqual([]);
  });
});

describe("expresionesDeFiltros -- combinaciones", () => {
  it("dos condiciones de una columna se unen con su operador (AND por defecto)", () => {
    const modelo = {
      contratista_nombre: {
        filterType: "text",
        operator: "OR",
        conditions: [
          { type: "startsWith", filter: "Ana" },
          { type: "startsWith", filter: "Luis" },
        ],
      },
      empresa_nombre: {
        filterType: "text",
        conditions: [
          { type: "contains", filter: "a" },
          { type: "notContains", filter: "b" },
        ],
      },
    };
    expect(expresionesDeFiltros(modelo)).toEqual([
      'or(nombre_p.ilike."ana*",nombre_p.ilike."luis*")',
      'and(empresa_p.ilike."*a*",or(empresa_p.is.null,empresa_p.not.ilike."*b*"))',
    ]);
  });

  it("acepta el formato viejo condition1/condition2", () => {
    const modelo = {
      contratista_nombre: {
        filterType: "text",
        operator: "OR",
        condition1: { type: "equals", filter: "Ana" },
        condition2: { type: "equals", filter: "Luis" },
      },
    };
    expect(expresionesDeFiltros(modelo)).toEqual(['or(nombre_p.eq."ana",nombre_p.eq."luis")']);
  });

  it("varias columnas dan una expresión cada una (AND entre columnas) e ignora las desconocidas", () => {
    const modelo = {
      empresa_nombre: { filterType: "text", type: "equals", filter: "BAC" },
      gafete_numero: { filterType: "number", type: "equals", filter: 3 },
      inventada: { filterType: "text", type: "equals", filter: "x" },
    };
    expect(expresionesDeFiltros(modelo)).toEqual([
      'empresa_p.eq."bac"',
      'texto_busqueda.like."*bac*"',
      "gafete_numero.eq.3",
    ]);
    expect(expresionesDeFiltros(undefined)).toEqual([]);
    expect(expresionesDeFiltros({})).toEqual([]);
  });
});
