import { assertEquals } from "jsr:@std/assert@1";
import type { ContratistaValido } from "../_shared/reglas.ts";
import type { EmpresaEncontrada } from "../_shared/contratistas.ts";
import { type ContratistaGuardado, procesarEdicion, type Puertos } from "./edicion.ts";

const HOY = "2026-10-05";
const EMPRESA: EmpresaEncontrada = { id: "11111111-1111-4111-8111-111111111111", nombre: "BAC" };
const ID = "22222222-2222-4222-8222-222222222222";

function guardado(cambios: Partial<ContratistaGuardado> = {}): ContratistaGuardado {
  return {
    id: ID,
    identificacion: "112340567",
    tipo_ingreso: "SWAT",
    es_personal_ruta: false,
    fecha_vencimiento_praind: null,
    ...cambios,
  };
}

function puertosDePrueba(opciones: {
  actual?: ContratistaGuardado | null;
  adentro?: boolean;
  duplicada?: boolean;
  admin?: boolean;
} = {}) {
  const actualizados: ContratistaValido[] = [];
  const puertos: Puertos = {
    autorizar: () => Promise.resolve(opciones.admin === false ? null : "admin@example.com"),
    buscarContratista: () => Promise.resolve(opciones.actual === undefined ? guardado() : opciones.actual),
    buscarEmpresa: (id) => Promise.resolve(id === EMPRESA.id ? EMPRESA : null),
    estaAdentro: () => Promise.resolve(opciones.adentro ?? false),
    actualizar: (_id, contratista) => {
      if (opciones.duplicada) return Promise.resolve({ ok: false, motivo: "cedula_duplicada" });
      actualizados.push(contratista);
      return Promise.resolve({ ok: true, fila: { identificacion: contratista.cedula, nombre: contratista.nombre } });
    },
  };
  return { puertos, actualizados };
}

const edicion = {
  id: ID,
  cedula: "112340567",
  nombre: " ana  pérez ",
  empresa_id: EMPRESA.id,
  tipo_ingreso: "SWAT",
  fecha_vencimiento_praind: null,
  es_personal_ruta: false,
  con_acceso: true,
};

Deno.test("guarda los datos normalizados por las reglas del núcleo", async () => {
  const { puertos, actualizados } = puertosDePrueba();
  const respuesta = await procesarEdicion(edicion, HOY, puertos);
  assertEquals(respuesta.estado, 200);
  assertEquals(actualizados[0].nombre, "ANA PÉREZ");
});

Deno.test("cambia la cédula si la persona está afuera", async () => {
  const { puertos, actualizados } = puertosDePrueba();
  const respuesta = await procesarEdicion({ ...edicion, cedula: "2-0000-0002" }, HOY, puertos);
  assertEquals(respuesta.estado, 200);
  assertEquals(actualizados[0].cedula, "200000002");
});

Deno.test("no cambia la cédula de quien está adentro, con el mensaje de las apps", async () => {
  const { puertos, actualizados } = puertosDePrueba({ adentro: true });
  const respuesta = await procesarEdicion({ ...edicion, cedula: "200000002" }, HOY, puertos);
  assertEquals(respuesta.estado, 409);
  assertEquals(
    respuesta.cuerpo.detail,
    "No se puede cambiar la cédula mientras está adentro — registre primero la salida",
  );
  assertEquals(actualizados.length, 0);
});

Deno.test("adentro igual se edita lo demás, y la misma cédula escrita distinto no es un cambio", async () => {
  const { puertos } = puertosDePrueba({ adentro: true });
  assertEquals((await procesarEdicion({ ...edicion, nombre: "Ana María" }, HOY, puertos)).estado, 200);
  assertEquals((await procesarEdicion({ ...edicion, cedula: "01-1234-0567" }, HOY, puertos)).estado, 200);
});

Deno.test("con el PRAIND ya vencido se corrige el nombre o se quita el acceso sin tocar la fecha", async () => {
  const actual = guardado({ tipo_ingreso: "PRAIND", fecha_vencimiento_praind: "2026-10-01" });
  const { puertos } = puertosDePrueba({ actual });
  const datos = { ...edicion, tipo_ingreso: "PRAIND", fecha_vencimiento_praind: "2026-10-01" };
  assertEquals((await procesarEdicion({ ...datos, nombre: "Otro Nombre" }, HOY, puertos)).estado, 200);
  assertEquals((await procesarEdicion({ ...datos, con_acceso: false }, HOY, puertos)).estado, 200);
  // Pero si cambia la fecha, tiene que quedar vigente.
  const otraFecha = await procesarEdicion({ ...datos, fecha_vencimiento_praind: "2026-10-02" }, HOY, puertos);
  assertEquals(otraFecha.cuerpo.error, "praind_vencido");
});

Deno.test("un contratista viejo POR CORREO se corrige sin cambiarle el tipo, pero no se elige de nuevo", async () => {
  const { puertos } = puertosDePrueba({ actual: guardado({ tipo_ingreso: "POR_CORREO" }) });
  assertEquals((await procesarEdicion({ ...edicion, tipo_ingreso: "POR_CORREO" }, HOY, puertos)).estado, 200);
  const { puertos: deSwat } = puertosDePrueba();
  const aCorreo = await procesarEdicion({ ...edicion, tipo_ingreso: "POR_CORREO" }, HOY, deSwat);
  assertEquals(aCorreo.cuerpo.error, "tipo_ingreso_retirado");
});

Deno.test("401 sin permiso, 404 si no existe, 409 cédula repetida, 404 empresa inexistente", async () => {
  assertEquals((await procesarEdicion(edicion, HOY, puertosDePrueba({ admin: false }).puertos)).estado, 401);
  const noExiste = await procesarEdicion(edicion, HOY, puertosDePrueba({ actual: null }).puertos);
  assertEquals(noExiste.estado, 404);
  assertEquals(noExiste.cuerpo.detail, "El contratista ya no existe");
  const duplicada = await procesarEdicion(edicion, HOY, puertosDePrueba({ duplicada: true }).puertos);
  assertEquals(duplicada.estado, 409);
  assertEquals(duplicada.cuerpo.detail, "Ya existe un contratista con esa cédula");
  const sinEmpresa = await procesarEdicion(
    { ...edicion, empresa_id: "33333333-3333-4333-8333-333333333333" },
    HOY,
    puertosDePrueba().puertos,
  );
  assertEquals(sinEmpresa.cuerpo.detail, "La empresa seleccionada ya no existe");
});
