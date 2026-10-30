-- Empresas de proveedor fusionadas desde el panel (incidente 2026-10-30:
-- "FUDDY FACTORY" y "FOODIE FACTORY" eran la misma empresa). La fusión vive
-- en la nube; el equipo la recibe con el catálogo:
--   fusionada_en  `uuid` de la empresa en la que se fusionó esta. Nulo si es
--                 una empresa vigente. Las fusionadas no se ofrecen en los
--                 selectores ni en la búsqueda, y escribir su nombre al dar de
--                 alta lleva a la que quedó.
ALTER TABLE empresas_proveedor ADD COLUMN fusionada_en TEXT;
