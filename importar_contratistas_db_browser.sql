-- Carga masiva de contratistas para Control Acceso / schema v6
-- Generado a partir de los datos suministrados.
-- NOTA: MEDIO_INGRESO no se carga aquí porque pertenece a registro_ingresos.
-- es_personal_ruta se fija en 0 porque la fuente no contiene ese dato.
-- Los SWAT se cargan con fecha_vencimiento_praind = NULL.
-- MELVIN GARRO CHINCHILLA queda con fecha NULL por dato fuente inválido: 20/03/202/.
-- DIDIER LEONARDO BONILLA VEGA: 10/012028 se normalizó a 2028-01-10.

PRAGMA foreign_keys = ON;

-- Empresas
INSERT INTO empresas (nombre) VALUES ('ALDAMA') ON CONFLICT(nombre) DO NOTHING;
INSERT INTO empresas (nombre) VALUES ('BAC') ON CONFLICT(nombre) DO NOTHING;
INSERT INTO empresas (nombre) VALUES ('BLUE SATELLITE') ON CONFLICT(nombre) DO NOTHING;
INSERT INTO empresas (nombre) VALUES ('BROOMDAY') ON CONFLICT(nombre) DO NOTHING;
INSERT INTO empresas (nombre) VALUES ('ECOS') ON CONFLICT(nombre) DO NOTHING;
INSERT INTO empresas (nombre) VALUES ('EXPENIC') ON CONFLICT(nombre) DO NOTHING;
INSERT INTO empresas (nombre) VALUES ('MEDICO') ON CONFLICT(nombre) DO NOTHING;
INSERT INTO empresas (nombre) VALUES ('MULTIPRO S.A.') ON CONFLICT(nombre) DO NOTHING;
INSERT INTO empresas (nombre) VALUES ('SODEXO') ON CONFLICT(nombre) DO NOTHING;
INSERT INTO empresas (nombre) VALUES ('VALERIA') ON CONFLICT(nombre) DO NOTHING;

-- Contratistas
INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119430546', 'BRYAN JOSUE BLANCO DURAN',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-10-13', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '109260892', 'RANDALL FRANCISCO GEUVARA CORRALES',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-08-04', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117790003', 'VERNOR FRANCISCO MARTINEZ VARGAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-11-18', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '109910272', 'CESAR MARIN SOLIS',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2028-06-24', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '102070108', 'JOSE RAMON VEGA SALAS',
    (SELECT id FROM empresas WHERE nombre = 'MULTIPRO S.A.'),
    'PRAIND', '2027-05-25', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '186201060708', 'VANESSA CAROLINA MUÑOZ DE CHACON',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2026-12-09', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '113850770', 'MARIELA CORDERO SALAZAR',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-07-29', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '11572047', 'CESAR AUGUSTO MOLINA MARFTINEZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-07-15', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '111620770', 'HECTOR ALEXIS ORTEGA COTO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-07-15', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117680676', 'CRISTIAN BERMUDEZ CRUZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-05', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '118030610', 'JOHN VALLEJOS ESPINOZA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-10-13', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '602980672', 'SIDER ROJAS BRENES',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-01-07', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119210969', 'ABRAHAM JESUS ARRONIS NAVARRO',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2026-09-01', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '206020599', 'MAIKOL DANILO ESPINOZA VEGA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-04-23', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155848226125', 'CARLOS EMILIO SEQUEIRA CARMONA',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2028-04-06', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '118410371', 'NORLING FARIÑA ROCHA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-18', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '110360938', 'HUBERTO GUSTAVO CASCANTE PAVON',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2026-11-08', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '603500432', 'KARON DE LOS ANGELES UMAÑA UREÑA',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'IN_HOUSE', '2027-08-28', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114310235', 'ABIGAIL YAMURY FALLAS ARIAS',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-06-26', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119850113', 'CLAUDIO QUIROS MENA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-12-29', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '502810059', 'ANALIS CERDAS RODRIGUEZ',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2027-03-25', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117620526', 'ANTHONY JIMENEZ MATAMOROS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-06-02', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155854184808', 'DIMA CRUZ ARCEDA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-02-03', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '702620765', 'BRANDON RODRIGUEZ AZOFEIFA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-04-16', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '111370744', 'ADRIAN MONTOYA DURAN',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-04-30', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '108270883', 'JORGE GUSTAVO DIAZ VALVELVERDE',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-06-10', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117700656', 'ALEXANDER VARELA REYES',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-03-18', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117290998', 'VALERIA MARIN VALLDEPERAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-07-15', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114650317', 'JORGE ANDRES ROJAS ROSALES',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2026-08-16', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119201112330', 'YERLANDIS RODRIGUEZ ALMAGUEL',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-04-23', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '118990414', 'DONOBAN CORRALES ABARCA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-04-30', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '186203182013', 'CARLOS ARDILES MARTINEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-07-17', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114340439', 'KESTOR MORALES CONEJO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-07-06', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114110535', 'JORGE ZELAYA ACUÑA',
    (SELECT id FROM empresas WHERE nombre = 'BLUE SATELLITE'),
    'PRAIND', '2028-06-06', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114260728', 'ANDREI MADRIGAL ARAYA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-07-08', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119690338', 'JORDAN JOSE LOPEZ URROZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-07-11', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '113460789', 'FRANCHESCA HIDALGO UMAÑA',
    (SELECT id FROM empresas WHERE nombre = 'BLUE SATELLITE'),
    'PRAIND', '2028-06-06', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '703200149', 'JAROL CASTRO RODRIGUEZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-04-23', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114490818', 'NILLS QUIROS CALDERON',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-07-13', 0, 0
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114120636', 'SERGIO GUSTAVO BOLAÑOS CAMPOS',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-07-13', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155834532920', 'MARCO ANTHONY JIMENEZ REYES',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-07-08', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '120170247', 'DIRK JARED JARQUIN GARCIA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-07-04', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '110540105', 'MELANIA CORRALES GONZALEZ',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-07-09', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119201769925', 'RAGNAR ALFREDO GONZALEZ DIAZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-02-07', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '122200911520', 'ANDERSON LEONARDO ALVARADO ACOSTA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-01-29', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '113920151', 'LUIS GUILLERMO LOPEZ MONTERO',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-07-13', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '115430228', 'JOSE FRANCISCO ROJAS ROSALES',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-01-27', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119510836', 'JOSE ANDRES GARCIA MENDEZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2026-12-05', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '110370953', 'NIDIA PIZARRO NAVARRETE',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2026-09-10', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '402270686', 'EUNICE NOVO ARAYA',
    (SELECT id FROM empresas WHERE nombre = 'MULTIPRO S.A.'),
    'PRAIND', '2027-07-02', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155820504408', 'MARIA DE LOS ANGELES BERMUDEZ FLORES',
    (SELECT id FROM empresas WHERE nombre = 'BLUE SATELLITE'),
    'PRAIND', '2028-06-25', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '502540381', 'JOSE ADOLFO OBANDO ORTEGA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-07-30', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155826014917', 'NICOLAS PADILLA ALEMAN',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-01-17', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114900941', 'DANEL GERARDO BADILLA SALAZAR',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-06-01', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '603690520', 'OMAR PAZOS LEON',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-08-18', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114570860', 'VERONICA ROJAS ROSALES',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-04-29', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '112770967', 'JENCY FERNNDEZ MADRIGAL',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-04-29', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '115890975', 'MARIA EUGENIA CALDERON GUTIERREZ',
    (SELECT id FROM empresas WHERE nombre = 'BLUE SATELLITE'),
    'PRAIND', '2028-06-06', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '604360240', 'JOHAN OBANDO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-06-19', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155819825726', 'CINTHYA JUNIET CASTILLO',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2027-01-20', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '305660648', 'ANDREY CASTRO DE LEON',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2026-09-26', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155840233117', 'JEFRI DE JESUS ESPINOZA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-11-18', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '116140386', 'STEVEN ANDRES SOLANO SOLIS',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-07-17', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155842037109', 'JUAN CARLOS PEREZ SAENZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-06-30', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '702730233', 'JORDI LOPEZ MENDOZA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-06-05', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117110677', 'KEVIN ANDRES CALDERON CECILIANO',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2028-06-27', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114810856', 'RAFAEL ANGEL RIVERA NAVAS',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2028-07-15', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '109720564', 'RANDALL ENRIQUE GAMBOA FALLAS',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-06-03', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117610439', 'KENDALL ELIAN ASTUA MARIN',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-07-14', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '113070910', 'CARLOS MAURICIO SANCHEZ SEGURA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-01-19', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119201492017', 'REYDEL CONSUEGRA PEREZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-07-07', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155850846308', 'DERWIN MARIANO MELENDEZ PADILLA',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2028-04-18', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '118110062', 'JOSE PABLO MORAN BONILLA',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2028-04-06', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '701140265', 'CARLOS DARRILLO MURILLO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'IN_HOUSE', '2028-01-03', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '113800149', 'HERNALDO REYES RUIZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-03-19', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '115460879', 'CRISTOPHER CRUZ CHARPENTIER',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-04-30', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '116620806', 'BRAYAN STARLING RAMIREZ MUÑOZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-01-26', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117680696', 'SERGIO ALBERTO GRANADOS JIMENEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-11-18', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '186203088831', 'EDUARDO JOSE ARDILLES MARTINEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-07-04', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '115390820', 'MARIO ALBERTO MORA SALAZAR',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-12-02', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '112870185', 'JOSUE JIMENEZ SOLIS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-01-31', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '118000874', 'LUIS CASTRO BONILLA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-06-05', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '118040484', 'KEVIN CASANOVA ARCE',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-04-30', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '116060899', 'JURGEN CASTILLO CHINCHILLA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-06-05', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '118990260', 'JOET GERARDO GUZMAN SOTO',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-07-04', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117760966', 'ANTHONY CRUZ VARGAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-06-05', 0, 0
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119830742', 'ANGEL NORMAN SOLANNO FALLAS',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2028-06-27', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119480831', 'HINES BURGOS JEFFERSON HERNAN',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-12-03', 0, 0
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '60484023', 'ERICKSON QUESADA DIAZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-03-27', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '116850690', 'JOSE FABIAN BARRANTES BERMUDEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-05-30', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '111390970', 'MELVIN GARRO CHINCHILLA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', NULL, 0, 0
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '118870755', 'CRISTHIAN DIAZ THOMAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-07-15', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117550683', 'PABLO ALVARADO MORALES',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-02-07', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117910150', 'BRANDON SUAREZ LARA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-06-05', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '116790981', 'STEVEN MURILLO MORALES',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-03-04', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119480832', 'MINOR JOAF HINES BURGOS',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-04-30', 0, 0
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '115630855', 'DAYANA MICHELLE FRANCIS NUÑEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2026-09-09', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117570692', 'JEREMY MENA MEDRANO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-04-30', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '112930925', 'JOSE BONILLA AGUIRRE',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-05-14', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117250991', 'KENDALL ANTONIO MATUTE CORDERO',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-04-17', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '604520552', 'JASON STEVEN SANCHEZ ZUÑIGA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-06-23', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117820324', 'AARON MORUA SALAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-03-27', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '107630413', 'GERARDP MENA VILLALOBOS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2026-11-08', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '401870197', 'JEAN CARLO CARVAJAL CERDAS',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-08-18', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '304190910', 'DIDIER LEONARDO BONILLA VEGA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-01-10', 0, 0
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '119600101', 'IAN JOEL VARGAS JIMENEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-11-10', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '116750545', 'ZAEL RODRIGEZ BERMUDEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2026-09-23', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '402590992', 'DANIEL DAVID UGALDE HURTADO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-11-01', 0, 0
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '801450433', 'YARIS OCAMPO SEQUEIRA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2026-12-08', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '115090543', 'CARLOS MORA BONILLA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-07-16', 0, 0
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '112940563', 'HAZEL SMITH CASTILLO',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2026-08-21', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '115840236', 'ZAIDA MADRIGAL MONTOYA',
    (SELECT id FROM empresas WHERE nombre = 'BAC'),
    'SWAT', NULL, 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '118110402', 'DEIVER CALEB GUILLEN NAVARRO',
    (SELECT id FROM empresas WHERE nombre = 'BAC'),
    'SWAT', NULL, 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117900735', 'BERTA LIDIA ACEVEDO PALMA',
    (SELECT id FROM empresas WHERE nombre = 'BAC'),
    'SWAT', NULL, 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '112390934', 'ALEXANDRA MUÑOS SABORIO',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'IN_HOUSE', '2027-10-10', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '109020921', 'YESSENIA CARDENAS DURAN',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'IN_HOUSE', '2027-10-10', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '108150226', 'ERICK JOSE ZAMORA ROMERO',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'IN_HOUSE', '2027-10-10', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155811493532', 'NEFTALY JOSE PALACIOS URBINA',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'IN_HOUSE', '2027-10-10', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '155846079625', 'CARLOS ELDER ARROIGA TREMINIO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'IN_HOUSE', '2027-10-10', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '114610625', 'MARIANA MORA SEQUEIRA',
    (SELECT id FROM empresas WHERE nombre = 'ECOS'),
    'IN_HOUSE', '2027-10-10', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '304230717', 'HARRY GERARDO ARAYA FONSECA',
    (SELECT id FROM empresas WHERE nombre = 'MEDICO'),
    'IN_HOUSE', '2027-10-10', 0, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;

-- Verificaciones mínimas
SELECT COUNT(*) AS total_contratistas FROM contratistas;
SELECT COUNT(*) AS total_empresas FROM empresas;
PRAGMA foreign_key_check;
