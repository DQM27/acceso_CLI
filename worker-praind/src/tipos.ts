export interface Env {
  FOTOS: R2Bucket;
  SUPABASE_URL: string;
  SUPABASE_PUBLISHABLE_KEY: string;
  /** Lista separada por comas. */
  ORIGENES_PERMITIDOS: string;
}

/** Lo que el Worker necesita del exterior; las pruebas lo reemplazan. */
export interface Dependencias {
  fetch: (entrada: string, init?: RequestInit) => Promise<Response>;
}

export interface Permisos {
  puede_ver: boolean;
  puede_cargar: boolean;
  es_admin: boolean;
}

export type TipoImagen = "image/png" | "image/jpeg";
