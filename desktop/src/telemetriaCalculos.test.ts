import { describe, expect, it } from "vitest";
import {
  AgregadorBloqueos,
  AgregadorLlamadas,
  AgregadorRealtime,
  latenciaDesde,
  nombreCanonicoOperacion,
  percentil,
  tipoDeError,
  tipoDeRed,
  TIPOS_ERROR,
  clasificarErrorComando,
} from "./telemetriaCalculos";

// Los mismos ejemplos que `TelemetriaCalculosTest.kt` (móvil): si una de las
// dos plataformas cambia la regla, la otra tiene que cambiar con ella.
describe("nombreCanonicoOperacion", () => {
  it("pasa camelCase a snake_case", () => {
    expect(nombreCanonicoOperacion("buscarContratistas")).toBe("buscar_contratistas");
    expect(nombreCanonicoOperacion("vincularDispositivoInicial")).toBe("vincular_dispositivo_inicial");
    expect(nombreCanonicoOperacion("sincronizarConNube")).toBe("sincronizar_con_nube");
  });

  it("quita los sufijos de versiones viejas", () => {
    expect(nombreCanonicoOperacion("registrarIngresoVerificado")).toBe("registrar_ingreso");
    expect(nombreCanonicoOperacion("registrarIngresoConSecreto")).toBe("registrar_ingreso");
    expect(nombreCanonicoOperacion("prepararIngresoConSecreto")).toBe("preparar_ingreso");
    expect(nombreCanonicoOperacion("sincronizarConNubeConSecreto")).toBe("sincronizar_con_nube");
    expect(nombreCanonicoOperacion("autenticarConSecreto")).toBe("autenticar");
  });

  it("un nombre ya canónico no cambia", () => {
    expect(nombreCanonicoOperacion("registrar_ingreso")).toBe("registrar_ingreso");
    expect(nombreCanonicoOperacion("sincronizar_con_nube")).toBe("sincronizar_con_nube");
    expect(nombreCanonicoOperacion("login")).toBe("login");
  });

  it("es idempotente y no deja el nombre vacío", () => {
    for (const nombre of ["registrarIngresoVerificado", "sincronizarConNubeConSecreto", "login"]) {
      const una = nombreCanonicoOperacion(nombre);
      expect(nombreCanonicoOperacion(una)).toBe(una);
    }
    expect(nombreCanonicoOperacion("_verificado")).toBe("_verificado");
  });

  it("maneja siglas seguidas", () => {
    expect(nombreCanonicoOperacion("leerPDF417")).toBe("leer_pdf417");
    expect(nombreCanonicoOperacion("abrirXMLHttp")).toBe("abrir_xml_http");
  });
});

describe("tipoDeRed", () => {
  it("el escritorio sólo distingue si hay red o no", () => {
    expect(tipoDeRed(true)).toBe("con_red");
    expect(tipoDeRed(false)).toBe("sin_red");
  });
});

describe("percentil", () => {
  it("rango más cercano, igual que en el teléfono", () => {
    expect(percentil([], 0.5)).toBeNull();
    expect(percentil([10, 20, 30, 40], 0.5)).toBe(20);
    expect(percentil([10, 20, 30, 40], 0.9)).toBe(40);
  });
});

describe("AgregadorLlamadas", () => {
  it("resume por comando y se vacía", () => {
    const llamadas = new AgregadorLlamadas();
    llamadas.registrar("sincronizar_con_nube", 500, true);
    llamadas.registrar("sincronizar_con_nube", 900, false);
    llamadas.registrar("listar_ingresos_activos", 3.26, true);

    const filas = llamadas.vaciar();
    expect(filas).toContainEqual({
      nombre: "sincronizar_con_nube",
      llamadas: 2,
      errores: 1,
      errores_por_tipo: { otro: 1 },
      p50_ms: 500,
      p90_ms: 900,
      max_ms: 900,
      total_ms: 1400,
    });
    expect(filas.find((f) => f.nombre === "listar_ingresos_activos")?.p50_ms).toBe(3.3);
    expect(llamadas.vaciar()).toEqual([]);
  });
});

describe("AgregadorRealtime", () => {
  it("sin actividad no informa nada", () => {
    expect(new AgregadorRealtime().vaciar()).toBeNull();
  });

  it("cuenta conexión, avisos, ecos, aplicados y latencias con los campos del teléfono", () => {
    const realtime = new AgregadorRealtime();
    realtime.desfaseReloj(-120);
    realtime.conectando();
    realtime.suscrito(730);
    realtime.aviso("ingresos", 2048, false, 180);
    realtime.aviso("ingresos", 1024, true, 220);
    realtime.aplicado(true, 16.4);
    realtime.terminado("renovacion", 30 * 60_000);

    const resumen = realtime.vaciar();
    expect(resumen).toMatchObject({
      intentos_conexion: 1,
      ms_hasta_suscribir_p50: 730,
      fin_de_conexion: { renovacion: 1 },
      minutos_conectado_max: 30,
      avisos_por_tabla: { ingresos: 1 },
      avisos: 1,
      ecos_propios: 1,
      kb_recibidos: 3,
      aplicados: 1,
      no_aplicados: 0,
      latencia_ms_min: 180,
      latencia_ms_max: 220,
      latencia_corregida: true,
      desfase_reloj_ms: -120,
    });
    expect(realtime.vaciar()).toBeNull();
  });
});

describe("AgregadorBloqueos", () => {
  it("resume las tareas largas", () => {
    const bloqueos = new AgregadorBloqueos();
    expect(bloqueos.vaciar()).toBeNull();
    bloqueos.registrar(60);
    bloqueos.registrar(250);
    expect(bloqueos.vaciar()).toEqual({
      tareas_largas: 2,
      ms_bloqueado: 310,
      ms_p50: 60,
      ms_max: 250,
      mayores_a_200ms: 1,
    });
  });
});

describe("latenciaDesde", () => {
  it("corrige con el desfase de esta PC contra el servidor", () => {
    const servidor = "2026-09-29T03:35:00.000Z";
    const ahora = Date.parse(servidor) + 30_400;
    // La PC va 30 s adelantada: la latencia real son 400 ms.
    expect(latenciaDesde(servidor, ahora, 30_000)).toBe(400);
    expect(latenciaDesde(servidor, ahora, null)).toBe(30_400);
    expect(latenciaDesde("no es fecha", ahora, 0)).toBeNull();
  });
});

describe("tipoDeError", () => {
  it("sólo el tipo, nunca el mensaje", () => {
    expect(tipoDeError(new TypeError("cédula 1-2345-6789"))).toBe("TypeError");
    expect(tipoDeError("texto con datos")).toBe("string");
    expect(tipoDeError(undefined)).toBe("undefined");
  });
});

// Los mensajes son los reales del núcleo (`src/services/error.rs`): si alguien
// los reescribe, esta prueba avisa antes de que los errores se vayan a "otro".
describe("clasificarErrorComando", () => {
  const casos: Array<[string, string]> = [
    ["No hay una sesión activa", "no_autenticado"],
    ["La sesión que registra el movimiento no existe o está inactiva", "no_autenticado"],
    ["La sesión actual no está autorizada para realizar esta operación", "sin_autorizacion"],
    ["Credenciales inválidas", "credenciales"],
    ["Usuario inactivo", "credenciales"],
    ["La contraseña actual es incorrecta", "credenciales"],
    ["El reloj del equipo está atrasado respecto al último movimiento registrado", "reloj"],
    ["Acceso denegado: SinAcceso", "rechazado"],
    ["El PRAIND está vencido", "rechazado"],
    ["No hay ninguna visita agendada para esta cédula", "rechazado"],
    ["El gafete ya está asignado", "conflicto"],
    ["El gafete no está disponible: Perdido", "conflicto"],
    ["El contratista ya tiene un ingreso activo", "conflicto"],
    ["El contratista ya tiene un ingreso activo en el otro dispositivo del sitio", "conflicto"],
    ["La cédula del contratista ya existe", "conflicto"],
    ["Este vehículo ya tiene una salida de ruta activa", "conflicto"],
    ["Contratista no encontrado", "no_encontrado"],
    ["El gafete no está registrado en el catálogo", "no_encontrado"],
    ["El registro de ingreso no está activo", "no_encontrado"],
    ["La cédula es obligatoria", "validacion"],
    ["La cédula debe tener sólo números, entre 9 y 13 dígitos", "validacion"],
    ["La placa es obligatoria cuando el ingreso es en vehículo", "validacion"],
    ["La salida no puede ser anterior al ingreso", "validacion"],
    ["error sending request for url (https://ejemplo.supabase.co/rest/v1/x)", "sin_red"],
    ["No se pudo conectar con la nube", "sin_red"],
    ["database is locked", "base_de_datos"],
    ["Error interno", "interno"],
    ["algo que nadie previó", "otro"],
  ];

  it.each(casos)("«%s» es %s", (mensaje, esperado) => {
    expect(clasificarErrorComando(mensaje)).toBe(esperado);
  });

  it("también lee el mensaje de un Error", () => {
    expect(clasificarErrorComando(new Error("El gafete ya está asignado"))).toBe("conflicto");
  });

  it("sin mensaje que lo diga, la red decide: sin conexión es sin_red, con conexión es otro", () => {
    expect(clasificarErrorComando(undefined, false)).toBe("sin_red");
    expect(clasificarErrorComando(undefined, true)).toBe("otro");
  });

  it("un mensaje claro gana sobre la red: una validación sin conexión sigue siendo validación", () => {
    expect(clasificarErrorComando("La cédula es obligatoria", false)).toBe("validacion");
  });

  it("una cancelación es cancelada", () => {
    expect(clasificarErrorComando(new DOMException("abortado", "AbortError"))).toBe("cancelada");
  });

  // Privacidad: aunque el mensaje traiga una cédula o un nombre, de acá sólo
  // sale un valor de la lista fija.
  it("siempre devuelve un valor de la lista fija y nunca parte del mensaje", () => {
    const mensaje = "Marlon Quesada, cédula 1-0847-0293, no tiene permiso";
    const tipo = clasificarErrorComando(mensaje);
    expect(TIPOS_ERROR).toContain(tipo);
    expect(tipo).not.toMatch(/Marlon|1-0847/);
  });
});

describe("AgregadorLlamadas: errores por tipo", () => {
  it("una operación con errores trae cuántos de cada tipo", () => {
    const llamadas = new AgregadorLlamadas();
    llamadas.registrar("registrar_ingreso", 100, true);
    llamadas.registrar("registrar_ingreso", 120, false, "conflicto");
    llamadas.registrar("registrar_ingreso", 90, false, "conflicto");
    llamadas.registrar("registrar_ingreso", 80, false, "sin_red");

    const [fila] = llamadas.vaciar();

    expect(fila).toMatchObject({
      llamadas: 4,
      errores: 3,
      errores_por_tipo: { conflicto: 2, sin_red: 1 },
    });
  });

  it("un error sin tipo cuenta como otro", () => {
    const llamadas = new AgregadorLlamadas();
    llamadas.registrar("login", 10, false);
    expect(llamadas.vaciar()[0]).toMatchObject({ errores: 1, errores_por_tipo: { otro: 1 } });
  });

  it("sin errores, la fila no trae el campo (el formato de siempre)", () => {
    const llamadas = new AgregadorLlamadas();
    llamadas.registrar("login", 10, true);
    expect(llamadas.vaciar()[0]).not.toHaveProperty("errores_por_tipo");
  });

  it("los tipos no se arrastran de un volcado al siguiente", () => {
    const llamadas = new AgregadorLlamadas();
    llamadas.registrar("login", 10, false, "credenciales");
    llamadas.vaciar();
    llamadas.registrar("login", 10, false, "sin_red");
    expect(llamadas.vaciar()[0]).toMatchObject({ errores_por_tipo: { sin_red: 1 } });
  });
});
