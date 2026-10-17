import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(() => Promise.resolve()) }));

import { invoke } from "@tauri-apps/api/core";
import {
  activoDesdeContratista,
  activoDesdeCorreo,
  activoDesdeKof,
  activoDesdeProveedor,
  cerrarActivo,
  listarActivosUnificados,
} from "./activosUnificados";
import type { FilaActiva } from "./activos";
import type { FilaCorreoActiva } from "./correo";
import type { FilaGafeteProvisionalActiva } from "./gafetesProvisionales";
import type { FilaProveedorActiva } from "./proveedores";

const invocar = vi.mocked(invoke);

const contratistaLocal: FilaActiva = {
  origen: "local",
  registro_id: 7,
  contratista_id: 3,
  cedula: "1-0847-0293",
  contratista_nombre: "Marlon Quesada",
  empresa_nombre: "Constructora del Valle",
  tipo_ingreso: "InHouse",
  medio_ingreso: "Vehiculo",
  fecha_hora_ingreso: "2027-03-08T12:00:00Z",
  gafete_numero: 10,
  placa: "ABC123",
  usuario_ingreso_nombre: "root",
  resultado_registrado: "Permitido",
  resultado_acceso: "Permitido",
  nivel_permanencia: "Aviso",
};

const contratistaRemoto: FilaActiva = {
  origen: "remoto",
  uuid_remoto: "uuid-c",
  registro_id: null,
  contratista_id: null,
  cedula: null,
  contratista_nombre: "Persona Remota",
  empresa_nombre: null,
  tipo_ingreso: null,
  medio_ingreso: null,
  fecha_hora_ingreso: "2027-03-08T08:00:00Z",
  gafete_numero: null,
  placa: null,
  usuario_ingreso_nombre: "Op PC",
  resultado_registrado: null,
  resultado_acceso: null,
  nivel_permanencia: "Normal",
};

const proveedorLocal: FilaProveedorActiva = {
  origen: "local",
  id: 7,
  cedula: "2-0500-0600",
  nombre: "Ana Proveedora",
  empresa_nombre: "Distribuidora Sol",
  placa: null,
  gafete_numero: 21,
  fecha_hora_ingreso: "2027-03-08T13:00:00Z",
};

const proveedorRemoto: FilaProveedorActiva = {
  origen: "remoto",
  uuid_remoto: "uuid-p",
  id: null,
  cedula: "2-0500-0601",
  nombre: "Luis Proveedor",
  empresa_nombre: "Distribuidora Sol",
  placa: "XYZ789",
  gafete_numero: 22,
  fecha_hora_ingreso: "2027-03-08T13:30:00Z",
};

const correoLocal: FilaCorreoActiva = {
  origen: "local",
  id: 7,
  cedula: "3-0111-0222",
  nombre: "Visita Correo",
  motivo: "Entrevista RH, autorizó Ana Mora por correo",
  placa: null,
  gafete_numero: 31,
  fecha_hora_ingreso: "2027-03-08T14:00:00Z",
  usuario_ingreso_nombre: "Guarda 1",
};

const correoRemoto: FilaCorreoActiva = { ...correoLocal, origen: "remoto", uuid_remoto: "uuid-m", id: null };

const kofLocal: FilaGafeteProvisionalActiva = {
  origen: "local",
  id: 7,
  encargado_nombre: "Encargado KOF",
  encargado_codigo_empleado: "55012",
  gafete_numero: 41,
  fecha_hora_entrega: "2027-03-08T15:00:00Z",
  usuario_entrega_nombre: "Guarda 2",
};

const kofRemoto: FilaGafeteProvisionalActiva = {
  origen: "remoto",
  uuid_remoto: "uuid-k",
  id: null,
  encargado_nombre: "Encargado Remoto",
  encargado_codigo_empleado: "55013",
  gafete_numero: 42,
  fecha_hora_entrega: "2027-03-08T15:30:00Z",
  usuario_entrega_nombre: "Guarda 3",
};

describe("activoDesdeContratista", () => {
  it("lleva a la fila común el tipo, el medio con placa y el nivel de permanencia", () => {
    expect(activoDesdeContratista(contratistaLocal)).toMatchObject({
      clase: "contratista",
      cedula: "1-0847-0293",
      nombre: "Marlon Quesada",
      detalle: "Constructora del Valle",
      tipo: "IN HOUSE",
      medio: "ABC123",
      gafete_numero: 10,
      usuario_ingreso_nombre: "root",
      nivel_permanencia: "Aviso",
    });
  });

  it("una fila remota vieja, sin tipo ni medio, queda con guion y sin gafete", () => {
    expect(activoDesdeContratista(contratistaRemoto)).toMatchObject({
      cedula: null,
      detalle: null,
      tipo: "—",
      medio: "—",
      gafete_numero: null,
    });
  });
});

describe("activoDesdeProveedor", () => {
  it("es un PROVEEDOR con su empresa, sin nivel de permanencia ni usuario que dio el ingreso", () => {
    expect(activoDesdeProveedor(proveedorLocal)).toMatchObject({
      clase: "proveedor",
      tipo: "PROVEEDOR",
      detalle: "Distribuidora Sol",
      usuario_ingreso_nombre: "—",
      nivel_permanencia: null,
    });
  });

  it("con placa entró en vehículo (se ve la placa); sin placa, caminando", () => {
    expect(activoDesdeProveedor(proveedorRemoto).medio).toBe("XYZ789");
    expect(activoDesdeProveedor(proveedorLocal).medio).toBe("CAMINANDO");
  });
});

describe("activoDesdeCorreo", () => {
  it("es una visita por correo cuyo detalle es el motivo, distinta del tipo 'Por correo' de un contratista", () => {
    expect(activoDesdeCorreo(correoLocal)).toMatchObject({
      clase: "correo",
      tipo: "VISITA POR CORREO",
      detalle: "Entrevista RH, autorizó Ana Mora por correo",
      usuario_ingreso_nombre: "Guarda 1",
      nivel_permanencia: null,
    });
  });
});

describe("activoDesdeKof", () => {
  it("el préstamo KOF no tiene cédula: lleva el código de empleado como detalle y la entrega como hora", () => {
    expect(activoDesdeKof(kofLocal)).toMatchObject({
      clase: "kof",
      cedula: null,
      nombre: "Encargado KOF",
      detalle: "Cód. 55012",
      tipo: "KOF",
      medio: "—",
      gafete_numero: 41,
      fecha_hora_ingreso: "2027-03-08T15:00:00Z",
      usuario_ingreso_nombre: "Guarda 2",
      nivel_permanencia: null,
    });
  });
});

describe("clave", () => {
  // Cada clase numera sus registros por su cuenta: el id local 7 existe en las
  // cuatro. Si la clave no incluyera la clase, AG Grid las confundiría.
  it("no se repite entre clases aunque el id local sea el mismo", () => {
    const claves = [
      activoDesdeContratista(contratistaLocal).clave,
      activoDesdeProveedor(proveedorLocal).clave,
      activoDesdeCorreo(correoLocal).clave,
      activoDesdeKof(kofLocal).clave,
    ];
    expect(new Set(claves).size).toBe(4);
  });

  it("una fila local y una remota de la misma clase tampoco se confunden", () => {
    expect(activoDesdeProveedor(proveedorLocal).clave).not.toBe(activoDesdeProveedor(proveedorRemoto).clave);
  });
});

describe("cerrarActivo", () => {
  beforeEach(() => invocar.mockClear());

  it("cada clase local se cierra por su propio comando", async () => {
    await cerrarActivo(activoDesdeContratista(contratistaLocal));
    await cerrarActivo(activoDesdeProveedor(proveedorLocal));
    await cerrarActivo(activoDesdeCorreo(correoLocal));
    await cerrarActivo(activoDesdeKof(kofLocal));

    expect(invocar.mock.calls.map(([comando]) => comando)).toEqual([
      "registrar_salida",
      "registrar_salida_proveedor",
      "registrar_salida_correo",
      "registrar_devolucion_gafete_provisional",
    ]);
    // Los cuatro con el id local 7, no el de otra clase.
    expect(invocar.mock.calls.every(([, args]) => (args as { id: number }).id === 7)).toBe(true);
  });

  it("lo abierto por el otro dispositivo se cierra contra la nube, con su uuid", async () => {
    await cerrarActivo(activoDesdeContratista({ ...contratistaRemoto }));
    await cerrarActivo(activoDesdeProveedor(proveedorRemoto));
    await cerrarActivo(activoDesdeCorreo(correoRemoto));
    await cerrarActivo(activoDesdeKof(kofRemoto));

    expect(invocar.mock.calls).toEqual([
      ["cerrar_ingreso_remoto", { uuid: "uuid-c" }],
      ["cerrar_ingreso_proveedor_remoto", { uuid: "uuid-p" }],
      ["cerrar_ingreso_correo_remoto", { uuid: "uuid-m" }],
      ["cerrar_prestamo_gafete_provisional_remoto", { uuid: "uuid-k" }],
    ]);
  });
});

describe("listarActivosUnificados", () => {
  beforeEach(() => invocar.mockReset());

  it("junta las cuatro clases, local y remoto, y suma el total", async () => {
    const respuestas: Record<string, unknown> = {
      listar_ingresos_activos: {
        items: [
          {
            registro_id: 7,
            contratista_id: 3,
            cedula: "1-0847-0293",
            contratista_nombre: "Marlon Quesada",
            empresa_nombre: "Constructora del Valle",
            tipo_ingreso: "Praind",
            medio_ingreso: "Caminando",
            fecha_hora_ingreso: "2027-03-08T12:00:00Z",
            gafete_numero: null,
            placa: null,
            usuario_ingreso_nombre: "root",
            resultado_registrado: "Permitido",
            resultado_acceso: "Permitido",
            nivel_permanencia: "Normal",
          },
        ],
        // El núcleo puede recortar la lista: el total manda en el contador.
        total: 5,
      },
      listar_ingresos_remotos: [],
      listar_proveedores_activos: [
        {
          id: 7,
          cedula: "2-0500-0600",
          nombre: "Ana Proveedora",
          empresa_nombre: "Distribuidora Sol",
          placa: null,
          gafete_numero: 21,
          fecha_hora_ingreso: "2027-03-08T13:00:00Z",
        },
      ],
      listar_ingresos_proveedor_remotos: [],
      listar_correos_activos: [],
      listar_ingresos_correo_remotos: [
        {
          uuid: "uuid-m",
          cedula: "3-0111-0222",
          nombre: "Visita Correo",
          motivo: "Entrevista",
          placa: null,
          gafete_numero: 31,
          hora_entrada: "2027-03-08T14:00:00Z",
          usuario_entrada_nombre: "Guarda 1",
        },
      ],
      listar_gafetes_provisionales_activos: [],
      listar_prestamos_gafete_provisional_remotos: [
        {
          uuid: "uuid-k",
          encargado_nombre: "Encargado Remoto",
          encargado_codigo_empleado: "55013",
          gafete_numero: 42,
          hora_entrega: "2027-03-08T15:30:00Z",
          usuario_entrega_nombre: "Guarda 3",
        },
      ],
    };
    invocar.mockImplementation(((comando: string) => Promise.resolve(respuestas[comando] ?? [])) as typeof invoke);

    const { activos, total } = await listarActivosUnificados();

    expect(activos.map((a) => a.clase)).toEqual(["contratista", "proveedor", "correo", "kof"]);
    expect(total).toBe(5 + 1 + 1 + 1);
    expect(new Set(activos.map((a) => a.clave)).size).toBe(4);
  });

  it("si una de las listas falla, falla todo en vez de mostrar la pantalla incompleta", async () => {
    invocar.mockImplementation(((comando: string) =>
      comando === "listar_correos_activos"
        ? Promise.reject(new Error("fallo de base"))
        : Promise.resolve(comando === "listar_ingresos_activos" ? { items: [], total: 0 } : [])) as typeof invoke);

    await expect(listarActivosUnificados()).rejects.toThrow("fallo de base");
  });
});
