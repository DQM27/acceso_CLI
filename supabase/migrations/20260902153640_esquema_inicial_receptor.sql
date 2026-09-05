-- Sitios (sedes): Brisas, Cartago, Belén, etc.
create table sitios (
  id uuid primary key default gen_random_uuid(),
  nombre text not null unique,
  direccion text,
  created_at timestamptz not null default now()
);

-- Dispositivos: una fila por PC o celular, cada uno atado a un sitio.
-- El secreto real nunca se guarda en texto plano, solo su hash (sha-256 hex).
create table dispositivos (
  id uuid primary key default gen_random_uuid(),
  sitio_id uuid not null references sitios(id),
  tipo text not null check (tipo in ('pc', 'mobile')),
  etiqueta text not null,
  secret_hash text not null unique,
  created_at timestamptz not null default now(),
  revoked_at timestamptz
);
create index dispositivos_sitio_idx on dispositivos(sitio_id);

-- Espejo: contratistas. El id lo genera el dispositivo que lo crea (UUID),
-- así el upsert hacia la nube es idempotente si se reintenta un envío.
create table contratistas (
  id uuid primary key,
  sitio_id uuid not null references sitios(id),
  dispositivo_origen_id uuid not null references dispositivos(id),
  nombre text not null,
  identificacion text,
  empresa text,
  activo boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create index contratistas_sitio_idx on contratistas(sitio_id);

-- Cola: ingresos/salidas. Igual, id generado por el dispositivo de origen.
-- "version" habilita control de concurrencia optimista: cerrar (registrar
-- salida) solo tiene efecto si version coincide con la que vio el cliente;
-- si no, ya lo cerró el otro dispositivo del mismo sitio primero.
create table ingresos (
  id uuid primary key,
  sitio_id uuid not null references sitios(id),
  dispositivo_entrada_id uuid not null references dispositivos(id),
  contratista_id uuid not null references contratistas(id),
  contratista_nombre text not null,
  hora_entrada timestamptz not null,
  hora_salida timestamptz,
  dispositivo_salida_id uuid references dispositivos(id),
  version bigint not null default 1,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create index ingresos_sitio_idx on ingresos(sitio_id);
-- Para la vista "quién está adentro ahora" (salida pendiente) por sitio.
create index ingresos_activos_idx on ingresos(sitio_id) where hora_salida is null;

alter table sitios enable row level security;
alter table dispositivos enable row level security;
alter table contratistas enable row level security;
alter table ingresos enable row level security;

-- Sin políticas todavía: por ahora solo el service_role (usado por nosotros
-- vía MCP/administración) puede tocar estas tablas. Falta la función que
-- verifique el secreto de cada dispositivo y emita un JWT con el sitio_id
-- como claim -- ahí es donde se van a escribir las políticas reales de
-- "cada dispositivo solo ve su propio sitio".
