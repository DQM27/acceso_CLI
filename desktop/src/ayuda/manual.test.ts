import { describe, expect, it } from "vitest";
import {
  ancla,
  leerEnLinea,
  leerMarkdown,
  textoPlano,
} from "../../../supabase/functions/_shared/manual/markdown.ts";
import {
  armarManual,
  resolverEnlace,
} from "../../../supabase/functions/_shared/manual/manual.ts";
import { buscarEnManual, normalizar } from "../../../supabase/functions/_shared/manual/buscar.ts";
import type { Bloque } from "../../../supabase/functions/_shared/manual/markdown.ts";
import { manual } from "./contenido";

describe("leerMarkdown", () => {
  it("une las líneas cortadas de un párrafo y lee los títulos con su ancla", () => {
    const bloques = leerMarkdown("# Registrar una salida\n\n## Para qué sirve\n\nCerrar el ingreso\nde una persona.\n");
    expect(bloques).toEqual([
      { tipo: "titulo", nivel: 1, texto: "Registrar una salida", ancla: "registrar-una-salida" },
      { tipo: "titulo", nivel: 2, texto: "Para qué sirve", ancla: "para-qué-sirve" },
      { tipo: "parrafo", texto: "Cerrar el ingreso de una persona." },
    ]);
  });

  it("anclas como las de GitHub: sin signos, con tildes, y numeradas si se repiten", () => {
    expect(ancla('"Tiene un ingreso activo acá Y en otro sitio"')).toBe(
      "tiene-un-ingreso-activo-acá-y-en-otro-sitio",
    );
    expect(ancla("Pasos — por gafete (una o varias)")).toBe("pasos--por-gafete-una-o-varias");
    const anclas = leerMarkdown("## Pasos\n\n## Pasos\n").map((b) => (b.tipo === "titulo" ? b.ancla : ""));
    expect(anclas).toEqual(["pasos", "pasos-1"]);
  });

  it("listas numeradas con viñetas anidadas y texto que sigue en la línea de abajo", () => {
    const [lista] = leerMarkdown(
      [
        "1. Pulse **Ctrl+S**.",
        "2. Elija una opción:",
        "   - Si la persona **ya salió**, registre",
        "     su salida.",
        "   - Si sigue adentro, no haga nada.",
        "3. Listo, se ve",
        "   el mensaje.",
      ].join("\n"),
    );
    expect(lista).toMatchObject({ tipo: "lista", ordenada: true, inicio: 1 });
    const elementos = (lista as Bloque & { tipo: "lista" }).elementos;
    expect(elementos.map((e) => e.texto)).toEqual([
      "Pulse **Ctrl+S**.",
      "Elija una opción:",
      "Listo, se ve el mensaje.",
    ]);
    expect(elementos[1].hijos).toEqual([
      {
        tipo: "lista",
        ordenada: false,
        inicio: 1,
        elementos: [
          { texto: "Si la persona **ya salió**, registre su salida.", hijos: [] },
          { texto: "Si sigue adentro, no haga nada.", hijos: [] },
        ],
      },
    ]);
  });

  it("una línea en blanco entre elementos no parte la lista; un párrafo después sí la cierra", () => {
    const bloques = leerMarkdown("- uno\n\n- dos\n\nDespués.\n");
    expect(bloques).toHaveLength(2);
    expect((bloques[0] as Bloque & { tipo: "lista" }).elementos).toHaveLength(2);
    expect(bloques[1]).toEqual({ tipo: "parrafo", texto: "Después." });
  });

  it("tablas: encabezado, filas, sin la línea separadora y con barras escapadas", () => {
    const [tabla] = leerMarkdown(
      "| Mensaje | Qué hacer |\n|---|---|\n| \"Sin ingreso activo\" | Revise el número. |\n| a \\| b | c |\n",
    );
    expect(tabla).toEqual({
      tipo: "tabla",
      encabezado: ["Mensaje", "Qué hacer"],
      filas: [
        ['"Sin ingreso activo"', "Revise el número."],
        ["a | b", "c"],
      ],
    });
  });
});

describe("leerEnLinea", () => {
  it("negrita (con un enlace adentro), código y enlaces", () => {
    expect(leerEnLinea("Pulse **[Salida](registrar-salida.md)** o `Esc`.")).toEqual([
      { tipo: "texto", texto: "Pulse " },
      {
        tipo: "negrita",
        hijos: [{ tipo: "enlace", destino: "registrar-salida.md", hijos: [{ tipo: "texto", texto: "Salida" }] }],
      },
      { tipo: "texto", texto: " o " },
      { tipo: "codigo", texto: "Esc" },
      { tipo: "texto", texto: "." },
    ]);
  });

  it("los asteriscos sueltos quedan como texto", () => {
    expect(textoPlano(leerEnLinea("2 * 3 ** y"))).toBe("2 * 3 ** y");
  });
});

describe("resolverEnlace", () => {
  it("relativo a la carpeta del proceso, con ancla", () => {
    expect(resolverEnlace("escritorio/a.md", "b.md")).toEqual({ ruta: "escritorio/b.md", ancla: null });
    expect(resolverEnlace("escritorio/a.md", "../movil/c.md#paso-2")).toEqual({
      ruta: "movil/c.md",
      ancla: "paso-2",
    });
    expect(resolverEnlace("escritorio/a.md", "#mensajes")).toEqual({ ruta: "escritorio/a.md", ancla: "mensajes" });
  });

  it("direcciones de afuera o que no son del manual no se siguen", () => {
    expect(resolverEnlace("escritorio/a.md", "https://example.com")).toBeNull();
    expect(resolverEnlace("escritorio/a.md", "imagen.png")).toBeNull();
  });
});

describe("armarManual", () => {
  const archivos = {
    "README.md": [
      "# Manual",
      "## Índice — Escritorio (puesto de control)",
      "### Ingresos",
      "- ✅ [Ingreso](escritorio/ingreso.md)",
      "### Fuera del manual",
      "- **Rutas**: no se documentan.",
      "## Índice — Panel web",
      "- ✅ [Usuarios](panel/usuarios.md)",
      "## Mejoras de pantalla",
      "1. [Ingreso](escritorio/ingreso.md) otra vez, no cuenta.",
    ].join("\n"),
    "escritorio/ingreso.md": "# Registrar un ingreso\n\nTexto.",
    "escritorio/suelto.md": "# Proceso sin índice",
    "panel/usuarios.md": "# Usuarios del panel",
  };

  it("arma el índice de cada app en el orden del README; lo que falta queda en Otros", () => {
    const { indice, procesos } = armarManual(archivos);
    expect(procesos.get("escritorio/ingreso.md")?.titulo).toBe("Registrar un ingreso");
    expect(indice.escritorio.map((g) => [g.titulo, g.procesos.map((p) => p.ruta)])).toEqual([
      ["Ingresos", ["escritorio/ingreso.md"]],
      ["Otros", ["escritorio/suelto.md"]],
    ]);
    expect(indice.panel.map((g) => [g.titulo, g.procesos.map((p) => p.ruta)])).toEqual([
      ["", ["panel/usuarios.md"]],
    ]);
    expect(indice.movil).toEqual([]);
  });
});

describe("buscarEnManual", () => {
  const { procesos } = armarManual({
    "escritorio/salida.md":
      "# Registrar una salida\n\n## Mensajes que pueden aparecer\n\n| Mensaje | Qué hacer |\n|---|---|\n| \"Sin ingreso activo\" | Revise el número del gafete. |\n",
    "escritorio/gafete.md": "# Gafete perdido\n\n## Pasos\n\nMarque el gafete como perdido. El contratista queda sin ingreso.\n",
    "escritorio/praind.md": "# PRAIND vencido\n\nEl contratista no puede entrar.",
  });
  const todos = [...procesos.values()];

  it("sin importar tildes ni mayúsculas, y una palabra encuentra sus derivadas", () => {
    expect(normalizar("¿Cómo REGISTRO la Salida?")).toBe("como registro la salida");
    expect(buscarEnManual(todos, "gafet").map((r) => r.proceso.ruta)).toEqual([
      "escritorio/gafete.md",
      "escritorio/salida.md",
    ]);
    expect(buscarEnManual(todos, "práind")[0].proceso.ruta).toBe("escritorio/praind.md");
  });

  it("prefiere los que tienen todas las palabras útiles, ignorando las vacías", () => {
    expect(buscarEnManual(todos, "el gafete perdido").map((r) => r.proceso.ruta)).toEqual([
      "escritorio/gafete.md",
    ]);
  });

  it("si ninguno tiene todas, los que tienen más (al menos la mitad)", () => {
    // "marco" no está: el manual dice "Marque".
    expect(buscarEnManual(todos, "cómo marco un gafete perdido").map((r) => r.proceso.ruta)).toEqual([
      "escritorio/gafete.md",
    ]);
    expect(buscarEnManual(todos, "tractor amarillo")).toEqual([]);
    expect(buscarEnManual(todos, "gafete tractor amarillo")).toEqual([]);
  });

  it("el mensaje pegado entero lleva a su fila, con la sección para ir directo", () => {
    const [primero] = buscarEnManual(todos, "Sin ingreso activo");
    expect(primero.proceso.ruta).toBe("escritorio/salida.md");
    expect(primero.seccion).toBe("Mensajes que pueden aparecer");
    expect(primero.ancla).toBe("mensajes-que-pueden-aparecer");
    expect(primero.fragmento).toBe('"Sin ingreso activo" — Revise el número del gafete.');
  });

  it("menos de dos letras no busca nada", () => {
    expect(buscarEnManual(todos, "a")).toEqual([]);
    expect(buscarEnManual(todos, "  ")).toEqual([]);
  });
});

/** Todos los enlaces de unos bloques, recorriendo listas y tablas. */
function enlaces(bloques: Bloque[]): string[] {
  const destinos: string[] = [];
  const deTexto = (texto: string) => {
    const recorrer = (partes: ReturnType<typeof leerEnLinea>) => {
      for (const parte of partes) {
        if (parte.tipo === "enlace") destinos.push(parte.destino);
        if (parte.tipo === "negrita" || parte.tipo === "enlace") recorrer(parte.hijos);
      }
    };
    recorrer(leerEnLinea(texto));
  };
  for (const bloque of bloques) {
    if (bloque.tipo === "titulo" || bloque.tipo === "parrafo") deTexto(bloque.texto);
    else if (bloque.tipo === "lista")
      for (const elemento of bloque.elementos) {
        deTexto(elemento.texto);
        destinos.push(...enlaces(elemento.hijos));
      }
    else for (const celda of [...bloque.encabezado, ...bloque.filas.flat()]) deTexto(celda);
  }
  return destinos;
}

function anclasDe(bloques: Bloque[]): Set<string> {
  const anclas = new Set<string>();
  for (const bloque of bloques) {
    if (bloque.tipo === "titulo") anclas.add(bloque.ancla);
    if (bloque.tipo === "lista") for (const e of bloque.elementos) for (const a of anclasDe(e.hijos)) anclas.add(a);
  }
  return anclas;
}

// El manual real (`docs/manual/`), tal como viaja dentro del programa.
describe("el manual de docs/manual", () => {
  const procesos = [...manual.procesos.values()];

  it("trae las tres apps y cada proceso empieza con su título", () => {
    for (const app of ["escritorio", "movil", "panel"] as const) {
      expect(procesos.filter((p) => p.app === app).length).toBeGreaterThan(5);
    }
    for (const proceso of procesos) {
      expect(proceso.bloques[0], proceso.ruta).toMatchObject({ tipo: "titulo", nivel: 1 });
    }
  });

  it("cada proceso figura en el índice del README (nada queda en Otros)", () => {
    for (const app of ["escritorio", "movil", "panel"] as const) {
      const grupos = manual.indice[app];
      expect(grupos.map((g) => g.titulo)).not.toContain("Otros");
      const enIndice = grupos.flatMap((g) => g.procesos.map((p) => p.ruta));
      expect(new Set(enIndice).size).toBe(procesos.filter((p) => p.app === app).length);
    }
  });

  it("cada enlace lleva a un proceso que existe, y su ancla a una sección que existe", () => {
    const rotos: string[] = [];
    for (const proceso of procesos) {
      for (const destino of enlaces(proceso.bloques)) {
        const resuelto = resolverEnlace(proceso.ruta, destino);
        const objetivo = resuelto && manual.procesos.get(resuelto.ruta);
        if (!objetivo) rotos.push(`${proceso.ruta} → ${destino}`);
        else if (resuelto.ancla && !anclasDe(objetivo.bloques).has(resuelto.ancla))
          rotos.push(`${proceso.ruta} → ${destino} (no existe la sección)`);
      }
    }
    expect(rotos).toEqual([]);
  });

  it("los mensajes de pantalla se encuentran pegándolos tal cual", () => {
    const [primero] = buscarEnManual(
      procesos.filter((p) => p.app === "escritorio"),
      "Este encargado ya tiene un gafete provisional prestado",
    );
    expect(primero.proceso.ruta).toBe("escritorio/entregar-gafete-kof.md");
    expect(primero.seccion).toBe("Mensajes que pueden aparecer");
  });
});
