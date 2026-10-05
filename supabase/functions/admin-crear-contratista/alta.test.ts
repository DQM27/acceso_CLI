import { assertEquals } from "jsr:@std/assert@1";
import type { ContratistaValido } from "../_shared/reglas.ts";
import { hoyCostaRica } from "../_shared/reglas.ts";
import { type EmpresaEncontrada, procesarAlta, type Puertos } from "./alta.ts";

const HOY = "2026-10-04";
const EMPRESA: EmpresaEncontrada = { id: "11111111-1111-4111-8111-111111111111", nombre: "BAC" };

function puertosDePrueba(opciones: { duplicada?: boolean; admin?: boolean } = {}) {
  const guardados: ContratistaValido[] = [];
  const puertos: Puertos = {
    autorizar: () => Promise.resolve(opciones.admin === false ? null : "admin@example.com"),
    buscarEmpresa: (id) => Promise.resolve(id === EMPRESA.id ? EMPRESA : null),
    insertar: (contratista, empresa) => {
      if (opciones.duplicada) return Promise.resolve({ ok: false, motivo: "cedula_duplicada" });
      guardados.push(contratista);
      return Promise.resolve({
        ok: true,
        fila: { identificacion: contratista.cedula, nombre: contratista.nombre, empresa_nombre: empresa.nombre },
      });
    },
  };
  return { puertos, guardados };
}

const valido = {
  cedula: "1-1234-0567",
  nombre: " ana  pérez ",
  empresa_id: EMPRESA.id,
  tipo_ingreso: "PRAIND",
  fecha_vencimiento_praind: "2030-01-31",
  con_acceso: true,
};

Deno.test("guarda los datos normalizados por las reglas del núcleo", async () => {
  const { puertos, guardados } = puertosDePrueba();
  const respuesta = await procesarAlta(valido, HOY, puertos);
  assertEquals(respuesta.estado, 200);
  assertEquals(guardados, [{
    cedula: "112340567",
    nombre: "ANA PÉREZ",
    tipo_ingreso: "PRAIND",
    fecha_vencimiento_praind: "2030-01-31",
    es_personal_ruta: false,
    tiene_acceso: true,
  }]);
});

Deno.test("POR CORREO se rechaza: el hueco que tenía la función SQL", async () => {
  const { puertos, guardados } = puertosDePrueba();
  const respuesta = await procesarAlta({ ...valido, tipo_ingreso: "POR_CORREO" }, HOY, puertos);
  assertEquals(respuesta.estado, 422);
  assertEquals(respuesta.cuerpo.error, "tipo_ingreso_retirado");
  assertEquals(guardados.length, 0);
});

Deno.test("rechaza con el mismo mensaje que las apps", async () => {
  const { puertos } = puertosDePrueba();
  const vencido = await procesarAlta({ ...valido, fecha_vencimiento_praind: "2026-10-03" }, HOY, puertos);
  assertEquals(vencido.estado, 422);
  assertEquals(vencido.cuerpo.detail, "El PRAIND está vencido — ingrese una fecha vigente");

  const cedula = await procesarAlta({ ...valido, cedula: "12" }, HOY, puertos);
  assertEquals(cedula.cuerpo.detail, "La cédula debe tener sólo números, entre 9 y 13 dígitos");
});

Deno.test("sin acceso no se pide PRAIND", async () => {
  const { puertos } = puertosDePrueba();
  const respuesta = await procesarAlta(
    { ...valido, con_acceso: false, fecha_vencimiento_praind: null },
    HOY,
    puertos,
  );
  assertEquals(respuesta.estado, 200);
});

Deno.test("empresa inexistente y cédula repetida", async () => {
  const sinEmpresa = await procesarAlta(
    { ...valido, empresa_id: "22222222-2222-4222-8222-222222222222" },
    HOY,
    puertosDePrueba().puertos,
  );
  assertEquals(sinEmpresa.estado, 404);
  assertEquals(sinEmpresa.cuerpo.detail, "La empresa seleccionada ya no existe");

  const duplicada = await procesarAlta(valido, HOY, puertosDePrueba({ duplicada: true }).puertos);
  assertEquals(duplicada.estado, 409);
  assertEquals(duplicada.cuerpo.detail, "Ya existe un contratista con esa cédula");
});

Deno.test("quien no es administrador recibe 401 y no se guarda nada, aunque los datos sean válidos", async () => {
  const { puertos, guardados } = puertosDePrueba({ admin: false });
  const respuesta = await procesarAlta(valido, HOY, puertos);
  assertEquals(respuesta.estado, 401);
  assertEquals(guardados.length, 0);
  // Ni le cuenta qué regla falla: primero la autorización.
  assertEquals((await procesarAlta({ ...valido, tipo_ingreso: "POR_CORREO" }, HOY, puertos)).estado, 401);
});

Deno.test("la autorización y la empresa se consultan a la vez", async () => {
  const orden: string[] = [];
  const demora = (ms: number) => new Promise((listo) => setTimeout(listo, ms));
  const puertos: Puertos = {
    autorizar: async () => {
      orden.push("autorizar:inicio");
      await demora(20);
      orden.push("autorizar:fin");
      return "admin@example.com";
    },
    buscarEmpresa: async () => {
      orden.push("empresa:inicio");
      await demora(5);
      orden.push("empresa:fin");
      return EMPRESA;
    },
    insertar: () => Promise.resolve({ ok: true, fila: {} }),
  };
  assertEquals((await procesarAlta(valido, HOY, puertos)).estado, 200);
  // La empresa arranca antes de que termine la autorización.
  assertEquals(orden.slice(0, 2), ["autorizar:inicio", "empresa:inicio"]);
});

Deno.test("personal de ruta: se guarda si el tipo lo admite y pide PRAIND aunque sea SWAT", async () => {
  const { puertos, guardados } = puertosDePrueba();
  const conRuta = await procesarAlta({ ...valido, es_personal_ruta: true }, HOY, puertos);
  assertEquals(conRuta.estado, 200);
  assertEquals(guardados[0].es_personal_ruta, true);

  const swatConRuta = await procesarAlta(
    { ...valido, tipo_ingreso: "SWAT", es_personal_ruta: true, fecha_vencimiento_praind: null },
    HOY,
    puertos,
  );
  assertEquals(swatConRuta.estado, 422);
  assertEquals(swatConRuta.cuerpo.error, "personal_ruta_no_admitido");
});

Deno.test("cuerpo vacío o malformado", async () => {
  const { puertos } = puertosDePrueba();
  assertEquals((await procesarAlta(null, HOY, puertos)).estado, 400);
  assertEquals((await procesarAlta({}, HOY, puertos)).estado, 422);
});

Deno.test("hoy en Costa Rica, no en UTC", () => {
  // 2026-10-05 02:00 UTC = 2026-10-04 20:00 en Costa Rica (UTC-6).
  assertEquals(hoyCostaRica(new Date("2026-10-05T02:00:00Z")), "2026-10-04");
});
