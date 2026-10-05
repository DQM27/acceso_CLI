import { describe, expect, it } from "vitest";
import { validarGafeteOpcional } from "./VisitaCheckInModal.logica";

// "¿Ya está adentro?" lo decide el núcleo al verificar la cédula
// (`CitaService::verificar_check_in`, tests en src/services/cita_service.rs).
describe("validarGafeteOpcional", () => {
  it("vacío es válido, con numero null", () => {
    expect(validarGafeteOpcional("")).toEqual({ valido: true, numero: null });
    expect(validarGafeteOpcional("   ")).toEqual({ valido: true, numero: null });
  });

  it("no numérico es inválido", () => {
    expect(validarGafeteOpcional("abc")).toEqual({
      valido: false,
      mensaje: "Ingrese un número de gafete válido",
    });
  });

  it("numérico válido", () => {
    expect(validarGafeteOpcional("5")).toEqual({ valido: true, numero: 5 });
  });
});
