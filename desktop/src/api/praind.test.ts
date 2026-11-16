import { describe, expect, it } from "vitest";
import { comoBytes } from "./praind";

describe("comoBytes", () => {
  const png = [137, 80, 78, 71];

  it("acepta el ArrayBuffer del canal binario", () => {
    expect([...comoBytes(new Uint8Array(png).buffer)]).toEqual(png);
  });

  it("acepta la lista de números del canal de mensajes (si no, la imagen sale rota)", () => {
    expect([...comoBytes(png)]).toEqual(png);
  });

  it("acepta una vista sobre parte de un buffer", () => {
    const grande = new Uint8Array([0, ...png, 0]);
    expect([...comoBytes(grande.subarray(1, 5))]).toEqual(png);
  });

  it("rechaza otra cosa", () => {
    expect(() => comoBytes("137,80")).toThrow("formato inesperado");
  });
});
