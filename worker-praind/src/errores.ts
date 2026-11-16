/** Error con respuesta HTTP propia. `mensaje` es el texto que muestran las apps. */
export class ErrorHttp extends Error {
  constructor(
    readonly estado: number,
    readonly codigo: string,
    readonly mensaje: string,
  ) {
    super(`${codigo}: ${mensaje}`);
  }
}

/** Error devuelto por una RPC de Supabase (PostgREST). */
export class ErrorRpc extends Error {
  constructor(
    readonly estado: number,
    readonly codigoPg: string | undefined,
    readonly mensaje: string,
  ) {
    super(`rpc ${estado} ${codigoPg ?? ""}`);
  }

  /** Traduce el error de Postgres a la respuesta que ve la app. */
  aHttp(): ErrorHttp {
    switch (this.codigoPg) {
      case "42501":
        return new ErrorHttp(403, "sin_permiso", this.mensaje);
      case "22023":
        return new ErrorHttp(400, "dato_invalido", this.mensaje);
      case "P0002":
        return new ErrorHttp(404, "no_existe", this.mensaje);
    }
    if (this.estado === 401 || this.codigoPg?.startsWith("PGRST3")) {
      return new ErrorHttp(401, "sesion_invalida", "La sesión no es válida o venció.");
    }
    return new ErrorHttp(502, "error_nube", "No se pudo consultar la nube. Intentá de nuevo.");
  }
}
