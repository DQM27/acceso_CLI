-- Hora aproximada de llegada, puramente informativa (decisión explícita
-- del usuario): "esta visita llega a las 10:00" no significa negar el
-- paso a las 11:00 -- ninguna política ni la RPC la usan para autorizar
-- nada, sólo se guarda para mostrarla. `time`, no `text`: sí existe un
-- tipo nativo acá (a diferencia de SQLite del lado local).
alter table public.citas add column hora_estimada time;
