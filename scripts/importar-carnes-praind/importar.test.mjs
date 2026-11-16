// node --test scripts/importar-carnes-praind/importar.test.mjs
import assert from "node:assert/strict";
import { test } from "node:test";
import { armarPlan, claveEmpresa, extraerCampos, mismaPersona, normalizarCedula } from "./importar.mjs";

test("lee los campos del carné tal como los devuelve el OCR", () => {
  const campos = extraerCampos([
    "CARNET DE INDUCCIÓN AL SITE",
    "Nombre: Jordi Lopez Mendoza",
    "No. de cédula: 702730233",
    "Empresa: Multiservicios De Aldama De San Rafael",
    "Fecha de inducción: 06/06/2026",
    "Fecha de vencimiento de",
    "inducción: 05/06/2028",
  ]);
  assert.deepEqual(campos, {
    nombre: "Jordi Lopez Mendoza",
    cedula: "702730233",
    empresa: "Multiservicios De Aldama De San Rafael",
    induccion: "2026-06-06",
    vencimiento: "2028-06-05",
  });
});

test("la cédula se normaliza igual que en el núcleo", () => {
  assert.equal(normalizarCedula("1-1111-1111"), "111111111");
  assert.equal(normalizarCedula("0111111111"), "111111111");
  assert.equal(normalizarCedula(""), null);
});

test("las empresas se comparan sin forma jurídica, tildes ni mayúsculas", () => {
  assert.equal(claveEmpresa("Multiservicios De Aldama De San Rafael"), claveEmpresa("MULTISERVICIOS de aldama de san rafael"));
  assert.equal(claveEmpresa("Expenic Ing S.A"), claveEmpresa("EXPENIC ING SA"));
  assert.equal(claveEmpresa("Integracom de Centroamérica S A"), "integracom centroamerica");
});

test("misma persona: le falta un apellido o tiene un error de tipeo", () => {
  assert.ok(mismaPersona("CESAR AUGUSTO MOLINA MARTINEZ", "Cesar Augusto Molina Marftinez"));
  assert.ok(mismaPersona("ANA MARIA SOLANO", "Ana María Solano Vargas"));
  assert.ok(!mismaPersona("ANA SOLANO", "Ana Solano Vargas"));
  assert.ok(!mismaPersona("ANA MARIA SOLANO", "ANA LUCIA SOLANO"));
});

const carne = (extra) => ({
  nombre: "Ana Maria Solano",
  cedula: "111111111",
  empresa: "Expenic Ing S.A",
  vencimiento: "2028-01-01",
  sha: "abc",
  archivo: "a.png",
  ...extra,
});
const guardado = (extra) => ({
  id: "c1",
  nombre: "ANA MARIA SOLANO",
  identificacion: "1-1111-1111",
  empresa_id: "e1",
  empresa_nombre: "EXPENIC ING",
  tipo_ingreso: "PRAIND",
  fecha_vencimiento_praind: "2027-01-01",
  es_personal_ruta: false,
  activo: true,
  praind_foto: null,
  ...extra,
});
const empresas = [{ id: "e1", nombre: "EXPENIC ING S.A.", activa: true }];
const plan = (c, contratistas) =>
  armarPlan({ personas: [{ carne: carne(c), otros: [] }] }, empresas, contratistas, "2026-11-16").acciones[0];

test("existente: corre la fecha si el carné vence después y carga la foto", () => {
  const a = plan({}, [guardado()]);
  assert.equal(a.tipo, "actualizar");
  assert.equal(a.fecha, "2028-01-01");
  assert.ok(a.subeFoto);
  assert.ok(!a.cambiaEmpresa);
});

test("existente con la misma foto y fecha: no hace nada", () => {
  const a = plan({}, [guardado({ fecha_vencimiento_praind: "2028-01-01", praind_foto: "contratistas/c1/abc.png" })]);
  assert.equal(a.tipo, "nada");
});

test("el carné vencido no pisa la fecha pero sí carga la foto", () => {
  const a = plan({ vencimiento: "2026-01-01" }, [guardado({ fecha_vencimiento_praind: null })]);
  assert.equal(a.fecha, null);
  assert.ok(a.subeFoto);
});

test("nombre mal escrito: queda el del carné; si sólo le falta un apellido, queda el guardado", () => {
  assert.equal(plan({ nombre: "Ana Maria Solano" }, [guardado({ nombre: "ANA MARIA SOLANNO" })]).nombre, "ANA MARIA SOLANO");
  assert.equal(plan({ nombre: "Ana Solano" }, [guardado({ nombre: "ANA MARIA SOLANO" })]).nombre, "ANA MARIA SOLANO");
});

test("nuevo: se crea si el PRAIND está vigente; vencido no se puede registrar", () => {
  assert.equal(plan({}, []).tipo, "crear");
  assert.equal(plan({ vencimiento: "2026-01-01" }, []).tipo, "omitir");
});
