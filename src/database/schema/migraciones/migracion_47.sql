
CREATE UNIQUE INDEX idx_empresas_nombre_plegado ON empresas (PLEGAR(nombre));
CREATE UNIQUE INDEX idx_empresas_proveedor_nombre_plegado ON empresas_proveedor (PLEGAR(nombre));
