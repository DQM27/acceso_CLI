-- Tipo de dispositivo nuevo, de solo lectura -- para visores web (como
-- historial-brisas.html) que solo necesitan listar movimientos, nunca
-- registrar/cerrar ingresos. Antes de esto cualquier dispositivo con
-- secreto valido podia escribir, sin importar para que se lo pensó usar.

alter table public.dispositivos drop constraint if exists dispositivos_tipo_check;
alter table public.dispositivos add constraint dispositivos_tipo_check
  check (tipo = any (array['pc', 'mobile', 'visor']));

update public.dispositivos
set tipo = 'visor'
where etiqueta = 'Visor web historial (prueba)';

-- INSERT/UPDATE en ingresos: excluye explicitamente a 'visor' -- las
-- politicas de SELECT no cambian, un visor sigue leyendo todo lo de su
-- sitio igual que cualquier otro dispositivo.
drop policy if exists "crear ingresos del propio sitio" on public.ingresos;
create policy "crear ingresos del propio sitio"
on public.ingresos
for insert
to authenticated
with check (
  sitio_id = ((auth.jwt() ->> 'sitio_id')::uuid)
  and (auth.jwt() ->> 'tipo') <> 'visor'
);

drop policy if exists "actualizar ingresos del propio sitio" on public.ingresos;
create policy "actualizar ingresos del propio sitio"
on public.ingresos
for update
to authenticated
using (sitio_id = ((auth.jwt() ->> 'sitio_id')::uuid))
with check (
  sitio_id = ((auth.jwt() ->> 'sitio_id')::uuid)
  and (auth.jwt() ->> 'tipo') <> 'visor'
);
