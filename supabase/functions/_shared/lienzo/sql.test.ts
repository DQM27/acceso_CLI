import { assert, assertEquals, assertStringIncludes } from "jsr:@std/assert@1";
import { revisarSql } from "./sql.ts";

function aceptada(sql: string) {
  const r = revisarSql(sql);
  if (!r.ok) throw new Error(`Rechazó una consulta válida: ${r.error}\n${sql}`);
  return r;
}

function rechazada(sql: string, peligroso: boolean) {
  const r = revisarSql(sql);
  if (r.ok) throw new Error(`Aceptó: ${sql}`);
  assertEquals(r.peligroso, peligroso, `peligroso para: ${sql} (${r.error})`);
  return r;
}

Deno.test("acepta consultas de lectura sobre ia con lo que hace falta para analizar", () => {
  const consultas = [
    "select nombre, cedula from ia.contratistas where estado_praind = 'VENCIDA' order by nombre",
    "select (hora_entrada at time zone 'America/Costa_Rica')::date as dia, count(*) as n from ia.ingresos " +
    "where hora_entrada >= now() - interval '30 days' group by 1 order by 1",
    "with x as (select empresa_nombre, count(*) as c from ia.ingresos group by 1) select * from x where c > 3",
    "select p.nombre, count(*) filter (where p.placa is not null) as con_placa from ia.ingresos_proveedor p group by 1",
    "select nombre, row_number() over (partition by empresa_nombre order by nombre) as n from ia.contratistas",
    "select c.nombre from ia.contratistas c left join ia.ingresos i on i.contratista_id = c.id " +
    "and i.hora_entrada > now() - interval '60 days' where i.id is null and c.estado_praind = 'POR_VENCER'",
    "select extract(dow from hora_entrada) as dia_semana, count(*) as n from ia.ingresos group by 1",
    "select nombre from ia.contratistas union all select nombre from ia.ingresos_proveedor",
    // NOT EXISTS: el parser lo lee como función; antes daba una alerta falsa.
    "select c.nombre from ia.contratistas c where c.estado_praind = 'POR_VENCER' and not exists " +
    "(select 1 from ia.ingresos i where i.contratista_id = c.id and i.hora_entrada >= now() - interval '60 days')",
    "select coalesce(empresa_nombre, 'Sin empresa') as empresa, case when count(*) > 5 then 'muchos' else 'pocos' end as c " +
    "from ia.contratistas group by 1",
  ];
  for (const sql of consultas) aceptada(sql);
});

Deno.test("devuelve el SQL regenerado y las vistas que lee", () => {
  const r = aceptada("select   nombre from IA.Contratistas c join ia.ingresos i on i.contratista_id = c.id");
  assertEquals(r.vistas, ["contratistas", "ingresos"]);
  assertStringIncludes(r.sql.toLowerCase(), "ia");
  // El SQL regenerado se vuelve a revisar igual (no cambia el significado).
  aceptada(r.sql);
});

Deno.test("GROUP BY / ORDER BY por número de columna no se convierten en constantes", () => {
  const r = aceptada(
    "select (hora_entrada at time zone 'America/Costa_Rica')::date as dia, count(*) as n from ia.ingresos " +
      "group by 1 order by 2 desc, 1",
  );
  const sql = r.sql.replace(/\s+/g, " ");
  assert(!/GROUP BY \(1\)/i.test(sql) && !/ORDER BY \(2\)/i.test(sql), sql);
  assertStringIncludes(sql, "GROUP BY ((hora_entrada AT TIME ZONE");
  assertStringIncludes(sql, "ORDER BY n DESC , dia");
  // Una columna sin alias se ordena por su expresión.
  const sinAlias = aceptada("select empresa_nombre, count(*) from ia.ingresos group by 1 order by 1").sql;
  assert(!/\(1\)/.test(sinAlias), sinAlias);
});

Deno.test("rechaza tablas fuera de ia y funciones prohibidas como peligrosas", () => {
  rechazada("select token from public.tokens_push", true);
  rechazada("select * from contratistas", true);
  rechazada("select * from pg_catalog.pg_roles", true);
  rechazada("select set_config('role', 'service_role', true) as r, nombre from ia.contratistas", true);
  rechazada("select current_setting('request.jwt.claims') as c from ia.sitios", true);
  rechazada("select pg_sleep(10) as s from ia.sitios", true);
  // Lo que va adentro de un EXISTS también se revisa.
  rechazada("select nombre from ia.sitios where exists (select 1 from public.tokens_push)", true);
  rechazada("select pg_catalog.lower(nombre) as n from ia.sitios", true);
  rechazada("select nombre from ia.sitios where id in (select id from public.sitios)", true);
  rechazada("select * from ia.sitios, lateral set_config('role', 'service_role', true) as x", true);
});

Deno.test("rechaza escrituras y varias sentencias", () => {
  rechazada("delete from ia.sitios", true);
  rechazada("update ia.sitios set nombre = 'x'", true);
  rechazada("insert into ia.sitios (nombre) values ('x')", true);
  rechazada("drop table public.sitios", true);
  rechazada("with x as (delete from public.sitios returning 1) select * from x", true);
  rechazada("select 1 as a from ia.sitios; delete from public.sitios", true);
});

Deno.test("lo que no se entiende o no lee nada no es peligroso, sólo inválido", () => {
  rechazada("select nombre frmo ia.contratistas", false);
  rechazada("select 1 as uno", false);
  assert(!revisarSql("").ok);
});
