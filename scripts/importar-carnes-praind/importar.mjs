#!/usr/bin/env node
// Importa una carpeta de carnés PRAIND (fase 5 de
// docs/features-futuras/plan-fotos-praind.md). Ver README.md al lado.
//
//   node scripts/importar-carnes-praind/importar.mjs --carpeta <dir> [--correo <admin>] [--aplicar]
//
// Sin --aplicar sólo arma el informe: no crea ni cambia nada.
//
// Todo pasa por los mismos caminos que el panel y con la sesión del
// administrador (código por correo): las empresas con `panel_crear_empresa`,
// los contratistas con las Edge Functions `admin-crear-contratista` y
// `admin-editar-contratista` (las reglas del núcleo), y la imagen con el
// Worker `praind-brisas`. Nada salta la RLS.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { stdin, stdout } from "node:process";
import readline from "node:readline/promises";
import { fileURLToPath } from "node:url";

const AQUI = path.dirname(fileURLToPath(import.meta.url));
const RAIZ = path.resolve(AQUI, "../..");
const PROYECTO_STAGING = "pmrytjktlyiuikxuuxpr";

// ---------------------------------------------------------------- argumentos

function argumentos(lista) {
  const salida = { aplicar: false };
  for (let i = 0; i < lista.length; i++) {
    const a = lista[i];
    if (a === "--aplicar") salida.aplicar = true;
    else if (a === "--sin-ocr") salida.sinOcr = true;
    else if (a.startsWith("--")) salida[a.slice(2)] = lista[++i];
  }
  if (!salida.carpeta) {
    console.error("Uso: node importar.mjs --carpeta <dir> [--correo <admin>] [--aplicar] [--sin-ocr]");
    process.exit(2);
  }
  salida.carpeta = path.resolve(salida.carpeta);
  salida.salida = path.resolve(salida.salida ?? path.join(path.dirname(salida.carpeta), "importacion-praind"));
  return salida;
}

function configuracion() {
  const env = {};
  for (const archivo of [".env", ".env.local"]) {
    const ruta = path.join(RAIZ, "web", archivo);
    if (!existsSync(ruta)) continue;
    for (const linea of readFileSync(ruta, "utf8").split(/\r?\n/)) {
      const m = linea.match(/^\s*([A-Z_]+)\s*=\s*(.*)\s*$/);
      if (m) env[m[1]] = m[2];
    }
  }
  const url = process.env.SUPABASE_URL ?? env.VITE_SUPABASE_URL;
  const clave = process.env.SUPABASE_KEY ?? env.VITE_SUPABASE_PUBLISHABLE_KEY;
  const worker =
    process.env.PRAIND_URL ??
    (url.includes(PROYECTO_STAGING) ? "https://praind-staging.megabrisas.com" : "https://praind.megabrisas.com");
  return { url: url.replace(/\/+$/, ""), clave, worker };
}

// --------------------------------------------------------------- normalizar

/** Igual que `Cedula::normalizar` (reglas/src/cedula.rs). */
export function normalizarCedula(texto) {
  if (!texto) return null;
  let limpia = String(texto).replace(/[\s.\-]/g, "").toUpperCase();
  if (!limpia || !/^[A-Z0-9]+$/.test(limpia)) return null;
  if (limpia.length === 10 && limpia.startsWith("0") && /^\d+$/.test(limpia)) limpia = limpia.slice(1);
  return limpia;
}

export const plegar = (texto) =>
  String(texto ?? "")
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();

const palabras = (texto) => plegar(texto).split(" ").filter(Boolean);

function distancia(a, b) {
  const fila = Array.from({ length: b.length + 1 }, (_, j) => j);
  for (let i = 1; i <= a.length; i++) {
    let previo = fila[0];
    fila[0] = i;
    for (let j = 1; j <= b.length; j++) {
      const tmp = fila[j];
      fila[j] = Math.min(fila[j] + 1, fila[j - 1] + 1, previo + (a[i - 1] === b[j - 1] ? 0 : 1));
      previo = tmp;
    }
  }
  return fila[b.length];
}

const palabraParecida = (a, b) => a === b || (Math.min(a.length, b.length) >= 4 && distancia(a, b) <= 1);

/** Todas las palabras del nombre más corto están (o casi, por un error de
 * tipeo) en el otro, y son al menos tres. */
export function mismaPersona(a, b) {
  const [corto, largo] = [palabras(a), palabras(b)].sort((x, y) => x.length - y.length);
  if (corto.length < 3) return false;
  const libres = [...largo];
  for (const p of corto) {
    const i = libres.findIndex((q) => palabraParecida(p, q));
    if (i < 0) return false;
    libres.splice(i, 1);
  }
  return true;
}

/** ¿El nombre del carné es sólo una versión más corta del guardado (le
 * falta un apellido)? Entonces se queda el guardado, que está más completo. */
const contenido = (corto, largo) => {
  const libres = palabras(largo);
  return palabras(corto).every((p) => {
    const i = libres.indexOf(p);
    if (i < 0) return false;
    libres.splice(i, 1);
    return true;
  });
};

const SUFIJOS = new Set(["sa", "srl", "inc", "ltda", "s", "a", "de", "cv", "rl"]);
/** Clave para comparar empresas: sin tildes, mayúsculas, puntuación ni la
 * forma jurídica ("S.A.", "SRL", ...). */
export function claveEmpresa(nombre) {
  return plegar(nombre)
    .replace(/[.,]/g, "")
    .replace(/[^a-z0-9 ]/g, " ")
    .split(/\s+/)
    .filter((p) => p && !SUFIJOS.has(p))
    .join(" ");
}

const fechaIso = (dma) => {
  const m = dma?.match(/^(\d{1,2})\/(\d{1,2})\/(\d{4})$/);
  return m ? `${m[3]}-${m[2].padStart(2, "0")}-${m[1].padStart(2, "0")}` : null;
};

export function hoyCostaRica(ahora = new Date()) {
  return new Date(ahora.getTime() - 6 * 3600 * 1000).toISOString().slice(0, 10);
}

// ------------------------------------------------------------------- leer

export function extraerCampos(lineas) {
  const t = lineas.join("\n");
  const campo = (re) => t.match(re)?.[1]?.trim() ?? null;
  const fechas = [...t.matchAll(/(\d{1,2})\s*\/\s*(\d{1,2})\s*\/\s*(\d{4})/g)].map((m) => `${m[1]}/${m[2]}/${m[3]}`);
  return {
    nombre: campo(/Nombre\s*:\s*(.+)/i),
    cedula: normalizarCedula(campo(/c.dula\s*:?\s*([0-9A-Za-z\- ]+)/i)),
    empresa: campo(/Empresa\s*:\s*(.+)/i),
    induccion: fechaIso(campo(/Fecha de inducci.n\s*:\s*(\d{1,2}\/\d{1,2}\/\d{4})/i) ?? fechas[0]),
    vencimiento: fechaIso(
      campo(/vencimiento de\s*\n?\s*inducci.n\s*:\s*(\d{1,2}\/\d{1,2}\/\d{4})/i) ?? (fechas.length >= 2 ? fechas.at(-1) : null),
    ),
  };
}

export function leerCarpeta(opciones) {
  mkdirSync(opciones.salida, { recursive: true });
  const ocr = path.join(opciones.salida, "ocr.json");
  if (!opciones.sinOcr || !existsSync(ocr)) {
    console.log("Leyendo los carnés con el OCR de Windows (unos segundos por cada 10)…");
    execFileSync(
      "powershell.exe",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        path.join(AQUI, "leer-carnes.ps1"),
        "-Carpeta",
        opciones.carpeta,
        "-Salida",
        ocr,
        "-Convertidos",
        path.join(opciones.salida, "convertidos"),
      ],
      { stdio: "inherit" },
    );
  }
  const rutaCorrecciones = path.join(opciones.salida, "correcciones.json");
  const correcciones = existsSync(rutaCorrecciones) ? JSON.parse(readFileSync(rutaCorrecciones, "utf8")) : {};

  const leidos = JSON.parse(readFileSync(ocr, "utf8").replace(/^﻿/, ""));
  const carnes = [];
  const descartados = [];
  for (const x of leidos) {
    const arreglo = correcciones[x.archivo] ?? {};
    if (arreglo.omitir) {
      descartados.push({ archivo: x.archivo, motivo: arreglo.motivo ?? "omitido en correcciones.json" });
      continue;
    }
    if (x.error) {
      descartados.push({ archivo: x.archivo, motivo: `no se pudo leer: ${x.error}` });
      continue;
    }
    const campos = { ...extraerCampos(x.lineas), ...arreglo };
    if (arreglo.cedula) campos.cedula = normalizarCedula(arreglo.cedula);
    const faltan = ["nombre", "cedula", "empresa", "vencimiento"].filter((k) => !campos[k]);
    if (faltan.length) {
      descartados.push({ archivo: x.archivo, motivo: `no se leyó: ${faltan.join(", ")} (corregir en correcciones.json)` });
      continue;
    }
    const bytes = readFileSync(x.imagen);
    carnes.push({
      ...campos,
      archivo: x.archivo,
      imagen: x.imagen,
      modificado: x.modificado,
      bytes: bytes.length,
      sha: createHash("sha256").update(bytes).digest("hex"),
    });
  }

  // Una persona puede tener varios carnés (el sitio da un enlace nuevo en
  // cada inducción): queda el que vence más tarde; a igual vencimiento, la
  // inducción más nueva y después el archivo más nuevo.
  const porCedula = new Map();
  for (const c of carnes) {
    const lista = porCedula.get(c.cedula) ?? [];
    lista.push(c);
    porCedula.set(c.cedula, lista);
  }
  const personas = [...porCedula.values()].map((lista) => {
    lista.sort(
      (a, b) =>
        b.vencimiento.localeCompare(a.vencimiento) ||
        (b.induccion ?? "").localeCompare(a.induccion ?? "") ||
        b.modificado.localeCompare(a.modificado),
    );
    return { carne: lista[0], otros: lista.slice(1) };
  });
  return { personas, descartados, totalArchivos: leidos.length };
}

// --------------------------------------------------------------- la nube

class Nube {
  constructor(config) {
    this.config = config;
    this.sesion = null;
  }

  async entrar(correo) {
    const rl = readline.createInterface({ input: stdin, output: stdout });
    try {
      correo ??= (await rl.question("Correo del administrador del panel: ")).trim();
      await this.auth("/otp", { email: correo, create_user: false });
      const codigo = (await rl.question(`Te mandé un código a ${correo}. Código: `)).trim();
      this.guardarSesion(await this.auth("/verify", { type: "email", email: correo, token: codigo }));
    } finally {
      rl.close();
    }
  }

  guardarSesion(s) {
    this.sesion = { token: s.access_token, refresco: s.refresh_token, vence: Date.now() + (s.expires_in - 120) * 1000 };
  }

  async auth(ruta, cuerpo) {
    const r = await fetch(`${this.config.url}/auth/v1${ruta}`, {
      method: "POST",
      headers: { apikey: this.config.clave, "Content-Type": "application/json" },
      body: JSON.stringify(cuerpo),
    });
    const datos = await r.json().catch(() => ({}));
    if (!r.ok) throw new Error(`No se pudo entrar: ${datos.msg ?? datos.error_description ?? datos.message ?? r.status}`);
    return datos;
  }

  async token() {
    if (Date.now() > this.sesion.vence) {
      this.guardarSesion(
        await this.auth("/token?grant_type=refresh_token", { refresh_token: this.sesion.refresco }),
      );
    }
    return this.sesion.token;
  }

  async pedir(url, init = {}) {
    const r = await fetch(url, {
      ...init,
      headers: { apikey: this.config.clave, Authorization: `Bearer ${await this.token()}`, ...init.headers },
    });
    const texto = await r.text();
    let datos = null;
    try {
      datos = texto ? JSON.parse(texto) : null;
    } catch {
      datos = texto;
    }
    if (!r.ok) throw new Error(datos?.detail ?? datos?.message ?? datos?.error ?? `respondió ${r.status}`);
    return datos;
  }

  async todas(tabla, columnas) {
    const filas = [];
    for (let desde = 0; ; desde += 1000) {
      const lote = await this.pedir(
        `${this.config.url}/rest/v1/${tabla}?select=${columnas}&order=id&offset=${desde}&limit=1000`,
      );
      filas.push(...lote);
      if (lote.length < 1000) return filas;
    }
  }

  rpc(nombre, args) {
    return this.pedir(`${this.config.url}/rest/v1/rpc/${nombre}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(args),
    });
  }

  funcion(nombre, cuerpo) {
    return this.pedir(`${this.config.url}/functions/v1/${nombre}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(cuerpo),
    });
  }

  subirCarne(contratistaId, bytes, nombreEnOrigen) {
    const q = new URLSearchParams({ nombre_en_origen: nombreEnOrigen });
    return this.pedir(`${this.config.worker}/cargar/${contratistaId}?${q}`, {
      method: "PUT",
      headers: { "Content-Type": "application/octet-stream" },
      body: bytes,
    });
  }
}

// ------------------------------------------------------------------- plan

export function armarPlan({ personas }, empresas, contratistas, hoy) {
  // Empresas: cada nombre del carné se asocia a una existente (misma clave,
  // o una empieza con la otra y no hay dudas) o se crea con la forma más
  // usada en los carnés.
  const variantes = new Map();
  for (const { carne } of personas) {
    const k = claveEmpresa(carne.empresa);
    const m = variantes.get(k) ?? new Map();
    m.set(carne.empresa, (m.get(carne.empresa) ?? 0) + 1);
    variantes.set(k, m);
  }
  const existentes = empresas.map((e) => ({ ...e, clave: claveEmpresa(e.nombre) }));
  const empresaDe = new Map(); // clave del carné -> { id?, nombre, nueva }
  for (const [k, m] of variantes) {
    const nombre = [...m.entries()].sort((a, b) => b[1] - a[1])[0][0];
    let encontrada = existentes.filter((e) => e.clave === k);
    if (encontrada.length !== 1) {
      const parecidas = existentes.filter(
        (e) => e.clave && (e.clave.startsWith(`${k} `) || k.startsWith(`${e.clave} `)),
      );
      if (!encontrada.length && parecidas.length === 1) encontrada = parecidas;
    }
    const activa = encontrada.find((e) => e.activa) ?? encontrada[0];
    empresaDe.set(k, activa ? { id: activa.id, nombre: activa.nombre, nueva: false, delCarne: nombre } : { nombre, nueva: true, delCarne: nombre });
  }

  const porCedula = new Map(contratistas.map((c) => [normalizarCedula(c.identificacion), c]));
  const acciones = [];
  for (const persona of personas) {
    const { carne } = persona;
    const empresa = empresaDe.get(claveEmpresa(carne.empresa));
    let actual = porCedula.get(carne.cedula);
    let como = "cédula";
    if (!actual) {
      const candidatos = contratistas.filter((c) => mismaPersona(c.nombre, carne.nombre));
      if (candidatos.length === 1) {
        actual = candidatos[0];
        como = "nombre";
      }
    }
    const vencido = carne.vencimiento < hoy;
    const accion = { persona, empresa, como, cambios: [], vencido };

    if (!actual) {
      accion.tipo = vencido ? "omitir" : "crear";
      if (vencido) accion.motivo = `PRAIND vencido el ${carne.vencimiento}: la nube no deja registrar a alguien nuevo con el PRAIND vencido`;
      acciones.push(accion);
      continue;
    }

    accion.actual = actual;
    accion.tipo = "actualizar";
    const nombre = { nuevo: actual.nombre };
    if (plegar(actual.nombre) !== plegar(carne.nombre) && !contenido(carne.nombre, actual.nombre)) {
      nombre.nuevo = carne.nombre.toUpperCase();
      accion.cambios.push(`nombre: ${actual.nombre} → ${nombre.nuevo}`);
    }
    accion.nombre = nombre.nuevo;

    accion.empresaId = actual.empresa_id;
    if (actual.empresa_id !== empresa.id) {
      const actualClave = claveEmpresa(actual.empresa_nombre ?? "");
      const mismaPrimera = actualClave && actualClave.split(" ")[0] === claveEmpresa(carne.empresa).split(" ")[0];
      if (!mismaPrimera) {
        accion.empresaId = empresa.id ?? null; // null: se crea antes
        accion.cambiaEmpresa = true;
        accion.cambios.push(`empresa: ${actual.empresa_nombre ?? "—"} → ${empresa.nombre}${empresa.nueva ? " (nueva)" : ""}`);
      }
    }

    accion.fecha = actual.fecha_vencimiento_praind;
    if (!vencido && (!actual.fecha_vencimiento_praind || carne.vencimiento > actual.fecha_vencimiento_praind)) {
      accion.fecha = carne.vencimiento;
      accion.cambios.push(`vence PRAIND: ${actual.fecha_vencimiento_praind ?? "—"} → ${carne.vencimiento}`);
    }

    accion.subeFoto = !actual.praind_foto?.includes(carne.sha);
    if (accion.subeFoto) accion.cambios.push(actual.praind_foto ? "carné: reemplaza el anterior" : "carné: lo carga");
    if (!accion.cambios.length) accion.tipo = "nada";
    acciones.push(accion);
  }
  return { acciones, empresaDe };
}

// ---------------------------------------------------------------- informe

function informe(plan, lectura, hoy, aplicado) {
  const { acciones, empresaDe } = plan;
  const de = (t) => acciones.filter((a) => a.tipo === t);
  const l = [];
  l.push(`# Importación de carnés PRAIND (${hoy})`, "");
  l.push(aplicado ? "**Aplicado.** Ver resultado.json para los errores." : "**Simulación:** no se cambió nada. Para aplicarlo, volver a correr con --aplicar.", "");
  l.push(`- Archivos leídos: ${lectura.totalArchivos}; descartados: ${lectura.descartados.length}`);
  l.push(`- Personas distintas: ${acciones.length}`);
  l.push(`- Contratistas nuevos: ${de("crear").length}`);
  l.push(`- Contratistas que se actualizan: ${de("actualizar").length}`);
  l.push(`- Ya al día: ${de("nada").length}`);
  l.push(`- No se pueden registrar (PRAIND vencido y no están en el sistema): ${de("omitir").length}`);
  const nuevas = [...empresaDe.values()].filter((e) => e.nueva);
  l.push(`- Empresas nuevas: ${nuevas.length}`, "");

  l.push("## Empresas", "");
  for (const e of [...empresaDe.values()].sort((a, b) => a.delCarne.localeCompare(b.delCarne))) {
    l.push(e.nueva ? `- **NUEVA** ${e.nombre}` : `- ${e.delCarne} → ${e.nombre}`);
  }
  const fila = (a) => {
    const c = a.persona.carne;
    return `- ${c.nombre.toUpperCase()} · ${c.cedula} · ${c.empresa} · vence ${c.vencimiento}${a.vencido ? " (VENCIDO)" : ""}`;
  };
  l.push("", "## Nuevos", "", ...de("crear").map(fila));
  l.push("", "## Se actualizan", "");
  for (const a of de("actualizar")) {
    l.push(`- ${a.actual.nombre} · ${a.actual.identificacion}${a.como === "nombre" ? ` (encontrado por NOMBRE; el carné dice cédula ${a.persona.carne.cedula})` : ""}`);
    for (const c of a.cambios) l.push(`  - ${c}`);
  }
  l.push("", "## No se pueden registrar", "", ...de("omitir").map(fila));
  l.push("", "## Repetidos (queda el que vence más tarde)", "");
  for (const a of acciones.filter((x) => x.persona.otros.length)) {
    l.push(`- ${a.persona.carne.nombre}: queda ${a.persona.carne.archivo}; se ignoran ${a.persona.otros.map((o) => `${o.archivo} (vence ${o.vencimiento})`).join(", ")}`);
  }
  l.push("", "## Archivos descartados", "", ...lectura.descartados.map((d) => `- ${d.archivo}: ${d.motivo}`));
  return `${l.join("\n")}\n`;
}

// ------------------------------------------------------------------ aplicar

async function aplicar(nube, plan, hoy, rutaResultado) {
  const resultado = [];
  const anotar = (r) => {
    resultado.push(r);
    writeFileSync(rutaResultado, JSON.stringify(resultado, null, 1));
  };

  for (const e of plan.empresaDe.values()) {
    if (!e.nueva) continue;
    try {
      const creada = await nube.rpc("panel_crear_empresa", { p_nombre: e.nombre });
      e.id = creada.id;
      e.nueva = false;
      console.log(`Empresa creada: ${creada.nombre}`);
      anotar({ empresa: e.nombre, ok: true });
    } catch (error) {
      console.log(`✗ Empresa ${e.nombre}: ${error.message}`);
      anotar({ empresa: e.nombre, ok: false, error: error.message });
    }
  }

  const trabajo = plan.acciones.filter((a) => a.tipo === "crear" || a.tipo === "actualizar");
  let n = 0;
  for (const a of trabajo) {
    n++;
    const c = a.persona.carne;
    const etiqueta = `[${n}/${trabajo.length}] ${c.nombre.toUpperCase()} (${c.cedula})`;
    try {
      let id;
      if (a.tipo === "crear") {
        if (!a.empresa.id) throw new Error(`no se pudo crear la empresa ${a.empresa.nombre}`);
        const fila = await nube.funcion("admin-crear-contratista", {
          cedula: c.cedula,
          nombre: c.nombre,
          empresa_id: a.empresa.id,
          tipo_ingreso: "PRAIND",
          fecha_vencimiento_praind: c.vencimiento,
          con_acceso: true,
          es_personal_ruta: false,
        });
        id = fila.id;
      } else {
        id = a.actual.id;
        const datosCambian = a.cambios.some((x) => !x.startsWith("carné:"));
        if (datosCambian) {
          const empresaId = a.cambiaEmpresa ? a.empresa.id : a.actual.empresa_id;
          if (!empresaId) throw new Error(`no se pudo crear la empresa ${a.empresa.nombre}`);
          await nube.funcion("admin-editar-contratista", {
            id,
            cedula: a.actual.identificacion,
            nombre: a.nombre,
            empresa_id: empresaId,
            tipo_ingreso: a.actual.tipo_ingreso ?? "PRAIND",
            fecha_vencimiento_praind: a.fecha,
            con_acceso: a.actual.activo,
            es_personal_ruta: a.actual.es_personal_ruta ?? false,
          });
        }
      }
      if (a.tipo === "crear" || a.subeFoto) {
        await nube.subirCarne(id, readFileSync(c.imagen), c.nombre);
      }
      console.log(`✓ ${etiqueta} ${a.tipo === "crear" ? "registrado" : a.cambios.join("; ")}`);
      anotar({ cedula: c.cedula, nombre: c.nombre, tipo: a.tipo, ok: true });
    } catch (error) {
      console.log(`✗ ${etiqueta}: ${error.message}`);
      anotar({ cedula: c.cedula, nombre: c.nombre, tipo: a.tipo, ok: false, error: error.message });
    }
  }
  return resultado;
}

// -------------------------------------------------------------------- main

async function main() {
  const opciones = argumentos(process.argv.slice(2));
  const config = configuracion();
  const hoy = hoyCostaRica();
  console.log(`Nube: ${config.url}\nCarnés: ${config.worker}\nCarpeta: ${opciones.carpeta}\nInformes en: ${opciones.salida}\n`);

  const lectura = leerCarpeta(opciones);
  console.log(`${lectura.personas.length} personas en ${lectura.totalArchivos} archivos (${lectura.descartados.length} descartados).\n`);

  const nube = new Nube(config);
  await nube.entrar(opciones.correo);
  const permisos = await nube.rpc("praind_permisos", {});
  if (!permisos?.es_admin) throw new Error("Esa cuenta no es administradora del panel.");

  const [empresas, contratistas] = await Promise.all([
    nube.todas("empresas", "id,nombre,activa"),
    nube.todas(
      "contratistas",
      "id,nombre,identificacion,empresa_id,empresa_nombre,tipo_ingreso,fecha_vencimiento_praind,es_personal_ruta,activo,praind_foto",
    ),
  ]);
  console.log(`En la nube: ${empresas.length} empresas, ${contratistas.length} contratistas.`);

  const plan = armarPlan(lectura, empresas, contratistas, hoy);
  const rutaInforme = path.join(opciones.salida, opciones.aplicar ? "informe-aplicado.md" : "informe-simulacion.md");
  writeFileSync(rutaInforme, informe(plan, lectura, hoy, false));
  console.log(`Informe: ${rutaInforme}`);

  if (!opciones.aplicar) {
    console.log("\nSimulación: no se cambió nada. Revisá el informe y, si está bien, corré lo mismo con --aplicar.");
    return;
  }
  const resultado = await aplicar(nube, plan, hoy, path.join(opciones.salida, "resultado.json"));
  const fallas = resultado.filter((r) => !r.ok);
  writeFileSync(rutaInforme, informe(plan, lectura, hoy, true));
  console.log(`\nListo: ${resultado.length - fallas.length} bien, ${fallas.length} con error (ver resultado.json).`);
  console.log("Se puede volver a correr: lo que ya quedó bien no se repite.");
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`\nError: ${error.message}`);
    process.exit(1);
  });
}
