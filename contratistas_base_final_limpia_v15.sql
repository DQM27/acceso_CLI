-- Base limpia de contratistas para Control de Acceso / schema v15
-- Generada a partir de la depuración del histórico y del catálogo normalizado.
--
-- REGLAS FINALES:
--  * Solo se cargan PRAIND, IN_HOUSE y SWAT.
--  * Todo personal de BAC se clasifica como SWAT.
--  * PRAIND/IN_HOUSE sin fecha conocida usan 2027-05-30 y es_personal_ruta = 1.
--  * SWAT usa fecha_vencimiento_praind = NULL y es_personal_ruta = 0.
--  * No contiene historial, filas históricas, cédulas fuente ni banderas de auditoría.
--  * No inserta registros en registro_ingresos.

PRAGMA foreign_keys = ON;

-- ============================================================
-- EMPRESAS UTILIZADAS POR LOS CONTRATISTAS FINALES
-- ============================================================
INSERT INTO empresas (nombre, activo) VALUES ('ALDAMA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('ARGUEDAS', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('ASOSI S.A', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('AVANTIA FIRE Y SECURITY', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('BAC', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('BELCA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('BLUE SATELLITE', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('BROOMDAY', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('CANFER', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('CAPACITA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('CAPACITACIÓN DE BRIGADA.', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('CIASA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('COM EL ORBE', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('COSEI', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('DYG', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('EBS', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('ECOS', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('ENTREVISTA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('EXPENIC', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('INTEGRACOM', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('K-9 INTERNACIONAL', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('KOF', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('LIMPIEZAS SRL', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('LLANTAS EXPRES', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('M BRISAS', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('MEDICO', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('MULTIPRO S.A.', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('NAVEGACION', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('ORBE', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('PROVISA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('RAICES MERCADEO', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('RENTOKILL', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('SCR', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('SIGMA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('SODEXO', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('STERICLEAN DE CENTRO AMERICA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('SUPRA CONTINENTAL INC', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('TALLER GERSON', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('TECMAS GUATEMALA S.A', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('TRACTOMOTRIZ', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('TRAUMA STORE', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('TRUCKSLOGIC', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('VALERIA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('WARDIAN', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;

-- ============================================================
-- CONTRATISTAS
-- Campos exclusivos del modelo contratistas.
-- ============================================================
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
    '118900608', 'ABRAHAM VILLALOBOS ZUÑIGA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-03-24', 0, 1
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
    '111370944', 'ADRIAN MONTOYA DURAN',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '800990581', 'ALBERTO GAMALIEL BLANDINO JIMENEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-04-30', 0, 1
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
    '604640781', 'ALEXANDER BERROCAL MUÑOS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-02-06', 0, 1
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
    '108270350', 'ALEXANDER GUILLEN MORALES',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-03-17', 0, 1
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
    '155854130221', 'ALEXANDER JOSE SOLANO ABARCA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-04-24', 0, 1
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
    '119670344', 'ALONSO ROJAS FIGUEROA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118420984', 'ANDY VARELA OBANDO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '117110824', 'ANTHONY JOSE BARRIOS CALDERON',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '117022863', 'ARLYN GEOVANNY VARGAS CHAVARRIA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '114600279', 'AUDRY ALVAREZ MONTOYA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119110917', 'BRANDON AGUIRRE CASTILLO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-04-15', 0, 1
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
    '119470695', 'BRANDON BADILLA VALVERDE',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116790185', 'BRANDON GAGO ESPINOZA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '304980645', 'BRANDON TENORIO SOTO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '117570322', 'BRYAN ENRIQUE LOPEZ PEREZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119430546', 'BRYAN JOSUE BLANCO DUARTE',
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
    '118870677', 'BRYAN VALLEJOS MONTOYA',
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
    '112490341', 'CARLOS DURAN MATA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110510377', 'CARLOS LUIS ACOSTA VEGA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '111380581', 'CARLOS LUIS ALFARO QUIJANO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-10-25', 0, 1
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
    '108550705', 'CARLOS LUIS CALVO CALDERON',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2026-07-03', 0, 1
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
    '304230563', 'CARLOS MONTERO NAVARRO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '701140265', 'CARLOS MURILLO CARRILLO',
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
    '109930402', 'CHRISTIAN ESTEBAN ARCE MADRIZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '113360800', 'CHRISTOPHER FRANCISCO CALVO CHAVEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '114900941', 'DANIEL GERARDO BADILLA SALAZAR',
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
    '401950656', 'DANIEL OVIEDO CAMPOS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '304100490', 'DANIEL STEVEN TRISTAN CHAVES',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '119100039', 'DARREL VAZQUEZ CHAVARRIA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-03-24', 0, 1
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
    '119550314', 'DAVID URROZ CARMONA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115630855', 'DAYANNA MICHELE FRANSIS NUÑEZ',
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
    '114860366', 'DENNIS FRANSISCO CORDERO ARIAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119470638', 'DERECK JIMENEZ FLORES',
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
    '503940035', 'DIDIER DIAZ ANGULO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-01-10', 0, 1
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
    '304190910', 'DIDIER LEANDRO BONILLA VEGA',
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
    '111320077', 'DIEGO ESTEBAN SERRNO MORA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119440463', 'DYLAN JAFFETH VILLAREVIA MONTOYA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118190373', 'EDINSON JOVAN ARROYO CERDAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '800880680', 'ELVIN MARTINEZ ILIAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '155846621002', 'ELYIN MIRANDA JARQUIN',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-12-22', 0, 1
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
    '602090384', 'EMILIO MIGUEL BEITA VILLANUEVA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '155857547616', 'ENGEELS CARMONA BALDELOMAR',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2025-05-15', 0, 1
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
    '604840723', 'ERICKSON QUESADA DIAZ',
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
    '118300409', 'ESTEBAN FABIAN HIDALGO LEANDRO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '111700474', 'FRANCISCO JAVIER JIMENEZ VILLAREAL',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110630564', 'FRANCISCO OCONITRILLO BRIONES',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110850475', 'FRANCISCO ZELEDON BARQUERO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '800880356', 'GEISSER FABRICIO MARTINEZ ILIAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '107630413', 'GERARDO MENA VILLALOBOS',
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
    '108090283', 'GERMAN EDUARDO MENDEZ SANCHO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '118910646', 'GERSON MARTINEZ VARGAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '702790853', 'HEINER ALEXANDER RAMIREZ ROJAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119480831', 'HINES BURGOS JEFFERSON HERNAN',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-12-13', 0, 0
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
    '110360938', 'HUBERT GUSTAVO CASCANTE POVEDA',
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
    '155842499120', 'IGNACIO FERNANDEZ LOPEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '503840422', 'IRWIN MEDRANO PARRA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118460474', 'JAFFET ANTONIO ALVAREZ MENA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115850733', 'JAIRO JOSHUE PEREZ RODRIGUEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '604520552', 'JASON STEVEN SHANCHEZ ZUÑIGA',
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
    '118340722', 'JEFFERSON CHINCHILLA RIVERA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '172400408127', 'JESUS ANGEL MARIÑO CAMPOS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118030610', 'JHON ALBERTO VALLEJOS ESPINOZA',
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
    '117080048', 'JOEL JOSHUE FONSECA RODRIGUEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119340114', 'JOHAN ALVARADO MARIN',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '603540621', 'JONATHAN GUIDO CALDERON',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-01-09', 0, 1
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
    '119790972', 'JORDAN STEVEN SOLANO LEON',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '108270883', 'JORGE GUSTAVO DIAZ VALVERDE',
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
    '116860016', 'JORGE NAVARRO ACUÑA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '304190955', 'JOSE ALBERTO ARAYA QUIROS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '155806629035', 'JOSE ALFONSO NUÑEZ MOJICA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '114270777', 'JOSE MAURICIO FALLAS CAMPOS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '114710900', 'JOSHUA JIMENEZ MUÑOZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119720915', 'JOSUE MORA COTO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-03-25', 0, 1
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
    '801330926', 'JUAN ARTURO OSPINA',
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
    '114730747', 'JUAN CARLOS QUIROS NAVARRO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '402570399', 'JUAN PABLO PACHECO QUESADA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '155855920522', 'KEVIN SILVA RAMOS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119140808', 'LESTER MENDIETA VELASQUEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118630265', 'LUIS ALONSO RODRIGUEZ BUSTAMANTE',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-01-21', 0, 1
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
    '118000894', 'LUIS CASTRO BONILLA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '109730805', 'LUIS DIEGO CHAVARRIA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2026-09-06', 0, 1
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
    '116850156', 'MAIKOL VILLALOBOS QUESADA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '117560778', 'MARCO VINICIO BRENES JIMENEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-12-01', 0, 1
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
    '111390970', 'MELVIN GARRO CHINCHILLA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-03-20', 0, 0
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
    '109150938', 'MELVIN SALAZAR ROJAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118900622', 'MICHAEL CANO HERNANDEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '109540495', 'MIGUEL ANTONIO CORDERO DINARTE',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116680955', 'MOISES CALDERON',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118410371', 'NORLING FARIÑA ROCHA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-01-22', 0, 1
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
    'PRAIND', '2028-01-22', 0, 1
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
    '155830283022', 'OSCAR DANILO LOPEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116260044', 'OSCAR EDUARDO MEDINA MEDINA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119201595525', 'OSMANY ERMELO CRUZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '701980160', 'RAFAEL CARRANZA NUÑEZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '114380170', 'RALPHI FRANCISO CAMPOS VILLALOBOS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '113740690', 'RANDALL VILLALOBOS MARIN',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '117170133', 'RICARDO MORALES JARA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '117060403', 'RODOLFO MARTIN ARIAS OVARES',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110370219', 'ROODY BONILLA RUIZ',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '119420912', 'ROY DANIEL SALAZAR ARAICA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119640948', 'SEBASTIAN CANALES LEON',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '117780268', 'STEVEN BURGOS OVIEDO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2028-02-27', 0, 1
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
    '114180549', 'STEVEN PICADO MORA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2026-07-08', 0, 1
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
    '120040055', 'TAYRON ZAPATA VAZQUEZ',
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
    '186202763226', 'VICTOR HUGO LEAL GUARAMATA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110160446', 'WARDNER EDUARDO MARIN UMAÑA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119570645', 'WENDELL MYRIE CASTRO',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119220894', 'WESLEY AZOFEIFA FALLAS',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '155847424811', 'WILMER ARGENIS MENDOZA GARCIA',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '119110948', 'YIANCARLO LUIS VIQUEZ PONCE',
    (SELECT id FROM empresas WHERE nombre = 'ALDAMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116750545', 'ZAEL RODRIGUEZ BERMUDEZ',
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
    '155852429423', 'CARLOS EDUARDO HERRERA MUÑOSZ',
    (SELECT id FROM empresas WHERE nombre = 'ARGUEDAS'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '155831094832', 'ENMANUEL VELZQUEZ RIVERA',
    (SELECT id FROM empresas WHERE nombre = 'ARGUEDAS'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '113920875', 'SAIMON FRANCISCO QUIROS RODRIGUEZ',
    (SELECT id FROM empresas WHERE nombre = 'ASOSI S.A'),
    'PRAIND', '2027-05-30', 1, 1
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
    '602610052', 'DIXSON FLORES CABEZAS',
    (SELECT id FROM empresas WHERE nombre = 'AVANTIA FIRE Y SECURITY'),
    'PRAIND', '2027-05-30', 1, 1
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
    '603270958', 'ADRIAN RAMIRES ESPINOZA',
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
    '119080011', 'ALLAN DARIEL RIVERA PADILLA',
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
    '116030489', 'ANTHONNY JOSE MURILLO RAMIREZ',
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
    '304790086', 'BRANDON ANDRES PEREIRA RUIZ',
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
    '117550942', 'BRYAN ROSALES SOTO',
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
    '114280412', 'EDWIN GONZALES ALFARO',
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
    '114200822', 'ERICKA SEGURA BARBOZA',
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
    '114610524', 'FRANCIS ANTONIO BONILLA PIÑAR',
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
    '701310460', 'FRANCISCO CAMACHO MOSCOSO',
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
    '155825658225', 'JADE MERCEDES RAMIREZ RIVAS',
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
    '119480080', 'JARED CHAVARRÍA CÓRDOBA',
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
    '114840127', 'JEAN CARLOS MOYA JARA',
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
    '115280647', 'KARLA GUIDO TAMES',
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
    '115840762', 'KENNETH RODRIGUEZ ESPINOZA',
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
    '304610803', 'MARCELO ANANIAS CORDERO ROBLES',
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
    '116580807', 'MARYAN BRAVO SOLIS',
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
    '116300719', 'ROGER JAVIER ZUÑIGA AGÜERO',
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
    '155808369419', 'SCARLETH JUNIETH SANCHEZ CASTRO',
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
    '117180927', 'ULISES GONZALEZ RODRIGUES',
    (SELECT id FROM empresas WHERE nombre = 'BELCA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '113090340', 'ALEXANDRA VEGA MATARRITA',
    (SELECT id FROM empresas WHERE nombre = 'BLUE SATELLITE'),
    'PRAIND', '2027-05-30', 1, 1
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
    '502400341', 'AURELIA VILLAFUERTE OBANDO',
    (SELECT id FROM empresas WHERE nombre = 'BLUE SATELLITE'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118680569', 'ISAAC DAVID CHAVARRIA MORERA',
    (SELECT id FROM empresas WHERE nombre = 'BLUE SATELLITE'),
    'PRAIND', '2027-05-30', 1, 1
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
    '109900603', 'LUIS FERNANDO LEITON CHAVES',
    (SELECT id FROM empresas WHERE nombre = 'BLUE SATELLITE'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115070809', 'MARIA ESTER ROY ROMERO',
    (SELECT id FROM empresas WHERE nombre = 'BLUE SATELLITE'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116000043', 'YESENIA KARINA DIAS QUIROZ',
    (SELECT id FROM empresas WHERE nombre = 'BLUE SATELLITE'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112870644', 'ALEJANDRA SOLIS CHACON',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119630804', 'DANIEL ORTEGA MARTINEZ',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-12-04', 0, 1
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
    '118790372', 'DILAN MONGE VARGAS',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112290539', 'GEISON JOSE ALVARADO HUERTAS',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119070716', 'JAKOL MONTIEL ESTRADA',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-05-30', 1, 1
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
    '111790137', 'JOHAN MAURICIO CASTILLO SANCHEZ',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115830349', 'JOSE ADRIAN ARIAS ACOSTA',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110520854', 'JULIO MORA ZUÑIGA',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-05-30', 1, 1
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
    '603500432', 'KAREN DE LOS ANGELES UMAÑA UREÑA',
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
    '113450310', 'LILIANA DE LOS ANGELES SOLANO LEON',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110540105', 'MELANIA CORRALES GONZALES',
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
    '604450079', 'WALTER GONZALEZ LARA',
    (SELECT id FROM empresas WHERE nombre = 'BROOMDAY'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112850787', 'DANNY MENA CASCANTE',
    (SELECT id FROM empresas WHERE nombre = 'CANFER'),
    'PRAIND', '2027-05-30', 1, 1
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
    '501830565', 'GILBERTO VEGA CORONADO',
    (SELECT id FROM empresas WHERE nombre = 'CANFER'),
    'PRAIND', '2027-05-30', 1, 1
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
    '114550721', 'FRANCISCO RODRIGUEZ ANGULO',
    (SELECT id FROM empresas WHERE nombre = 'CAPACITA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115620118', 'JEAN CARLOS CHAVEZ CORELLA',
    (SELECT id FROM empresas WHERE nombre = 'CAPACITA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110270928', 'JUAN PABLO PALOMO CAMACHO',
    (SELECT id FROM empresas WHERE nombre = 'CAPACITA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '109130148', 'LEONARDO MONGE NAVARRO',
    (SELECT id FROM empresas WHERE nombre = 'CAPACITA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '117990114', 'DAVID LASANTAS TORRES',
    (SELECT id FROM empresas WHERE nombre = 'CAPACITACIÓN DE BRIGADA.'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115950723', 'DIDIER BLANCO BLANDON',
    (SELECT id FROM empresas WHERE nombre = 'CIASA'),
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
    '117080836', 'GERALD SALAZAE SANABRIA',
    (SELECT id FROM empresas WHERE nombre = 'CIASA'),
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
    '112010396', 'BYRON ARNOLDO AZOFEIFA VARGAS',
    (SELECT id FROM empresas WHERE nombre = 'COM EL ORBE'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119160990', 'ADRIAN JOSUE ULLOA MENDEZ',
    (SELECT id FROM empresas WHERE nombre = 'COSEI'),
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
    '503030459', 'CARLOS MARQUEZ PEÑA',
    (SELECT id FROM empresas WHERE nombre = 'DYG'),
    'PRAIND', '2027-01-19', 0, 1
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
    '205720180', 'DAVID CUBERO PORRAS',
    (SELECT id FROM empresas WHERE nombre = 'DYG'),
    'PRAIND', '2026-10-13', 0, 1
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
    '206640519', 'FABRICIO CISNEROS GONZALES',
    (SELECT id FROM empresas WHERE nombre = 'DYG'),
    'PRAIND', '2027-02-25', 0, 1
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
    '204640409', 'OSCAR CUBERO PORRAS',
    (SELECT id FROM empresas WHERE nombre = 'DYG'),
    'PRAIND', '2027-03-21', 0, 1
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
    '205060878', 'WILIAM CUBERO PORRAS',
    (SELECT id FROM empresas WHERE nombre = 'DYG'),
    'PRAIND', '2026-04-06', 0, 1
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
    '801140208', 'CARLOS MEJIA PUELLO',
    (SELECT id FROM empresas WHERE nombre = 'EBS'),
    'PRAIND', '2027-05-30', 1, 1
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
    '702910253', 'BRIAN STEVEN GUZMAN JIMENEZ',
    (SELECT id FROM empresas WHERE nombre = 'ECOS'),
    'PRAIND', '2027-05-30', 1, 1
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
    '604220983', 'JOSAFAT EMMANUEL GUTIERREZ MADRIGAL',
    (SELECT id FROM empresas WHERE nombre = 'ECOS'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119390071', 'ALBERTH ZAMORA BONILLA',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '111050849', 'ALLAN SEGURA VALVERDE',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116310082', 'CRISTHIAN CUBERO CAMACHO',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115570607', 'CRITHIAN BRICEÑO MOLINA',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116600255', 'DAVID SALAZAR PEREZ',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '207710306', 'EMANUEL VIQUEZ ROBLETO',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116180125', 'ERICK HERRERA SEGURA',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115590668', 'ERICK MARTINEZ MARTINEZ',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '401980744', 'FREDDY GAMBOA SOLIS',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '113430340', 'JEFFRY GONZALEZ GRACIA',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116630935', 'JOSUE RODRIGUEZ CODRINGTON',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115340937', 'JULIO ACUÑA MORA',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118320666', 'KEVIN LEON ELIZONDO',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116510074', 'LUIS GAMBOA NAVARRO',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112740725', 'LUIS GUNTER ACUÑA ALVARADO',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116590053', 'MICKEL MORA GOMEZ',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '111150609', 'OBED CECILIANO VENEGAS',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116050388', 'SAHIR ARCE MORA',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116140396', 'STEVEN MONTOYA MONGE',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '113380695', 'WAGNER OBANDO CASTRO',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119130457', 'YEIKEL HERNANDEZ MORA',
    (SELECT id FROM empresas WHERE nombre = 'ENTREVISTA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119201835528', 'ADRIAN PEREZ DEBORA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '117080403', 'ALEJANDRO ALBERTO TAYLOR SALAZAR',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '186201982207', 'ALEXIS RODRIGUEZ LEE',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119830742', 'ANGEL NORMAN SOLANO FALLAS',
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
    '115620944', 'ANTHONY HERNANDEZ FONSECA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119340443', 'BRANDON CANTILLO GONZALES',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118470188', 'BRANDON PORRAS LEITON',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2026-12-19', 0, 1
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
    '114430343', 'BRAYAN ANDRES SEGURA GONZALEZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112740330', 'BRYAN CANO RETANA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '113580761', 'BRYAN LEANDRO ARIAS PORTILLO',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-03-06', 0, 1
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
    '112220257', 'BRYAN PICADO SEQUEIRA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-03-06', 0, 1
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
    '114870907', 'CARLOS JOSE AGUILAR GARCIA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119430125', 'DIEGO JOSETH BLANCO SANTAMARIA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119201345533', 'ENMANUEL CUESTA CATANARES',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2026-08-07', 0, 1
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
    '116600427', 'ESTEBAN RODRIGUEZ ROJAS',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '207540370', 'FABIO VASQUEZ CALDERON',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118840214', 'GARY GUZMAN BEJARANO',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119201405528', 'HENDRY ACUÑA ALVAREZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115810917', 'JEAN PAUL VALLADARES ORDOÑEZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '605050123', 'JEFERSSON CHAVES DUARTE',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112770967', 'JENCY FERNANDEZ MADRIGAL',
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
    '119470780', 'JHONNY LEANDRO CALDERON GUZMAN',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2026-06-27', 0, 1
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
    '112190070', 'JOSE MARIO RETANA VARGAS',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115200965', 'JUAN DANIEL PEREZ RAMIREZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '209850050', 'JULIAN NAVARRO BARRERA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118970852', 'JUNIOR SANDINO SALGADO',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '155845788922', 'KELVIN ISAAC RIVERA GONZALEZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '801530626', 'KERVIN WILLIAM ESCOBAR GONZALEZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2026-12-06', 0, 1
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
    '208380617', 'MARIA CELESTE HELIAS GONZALES',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119201447933', 'MICHAEL PALOMINO PADRON',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '114490818', 'NILLS QUIROS CADERON',
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
    '107450351', 'RAFAEL BOLAÑOS JIMENEZ',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '109260892', 'RANDALL FRANCISCO GUEVARA CORRALES',
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
    '155831126606', 'RUDY GUDAMUZ ZAMORA',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '116140386', 'STEVEN ANDES SOLANO SOLIS',
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
    '118370706', 'STIFF FABIAN ESPINOZA MORALES',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '114670909', 'WILLIAM ADRIAN MORA VENEGAS',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118560927', 'YAROD JOSE CRUZ SOLANO',
    (SELECT id FROM empresas WHERE nombre = 'EXPENIC'),
    'PRAIND', '2026-07-05', 0, 1
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
    '119470869', 'BYRON GABRIEL BONILLA OCON',
    (SELECT id FROM empresas WHERE nombre = 'INTEGRACOM'),
    'PRAIND', '2027-05-30', 1, 1
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
    '701300680', 'FRANSICO GARCIA MARQUEZ',
    (SELECT id FROM empresas WHERE nombre = 'INTEGRACOM'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119130105', 'JORDY MADRIZ BONILLA',
    (SELECT id FROM empresas WHERE nombre = 'INTEGRACOM'),
    'PRAIND', '2027-05-30', 1, 1
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
    '113460347', 'RICARDO BONILLA ARROYO',
    (SELECT id FROM empresas WHERE nombre = 'INTEGRACOM'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119020414', 'ANTHONY PANIAGUA MANZANAREZ',
    (SELECT id FROM empresas WHERE nombre = 'K-9 INTERNACIONAL'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116340105', 'FRABRIZIO FONSECA',
    (SELECT id FROM empresas WHERE nombre = 'K-9 INTERNACIONAL'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '503640334', 'PEDRO MOLINA MORALES',
    (SELECT id FROM empresas WHERE nombre = 'K-9 INTERNACIONAL'),
    'PRAIND', '2027-05-30', 1, 1
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
    '111880052', 'BAYRON JESUS ALVARADO BRENES',
    (SELECT id FROM empresas WHERE nombre = 'KOF'),
    'PRAIND', '2027-05-30', 1, 1
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
    '114760188', 'WALTER BLANCO PEÑA',
    (SELECT id FROM empresas WHERE nombre = 'LIMPIEZAS SRL'),
    'PRAIND', '2026-05-24', 0, 1
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
    '111750279', 'HAROLD ARTURO VALVERDE DELGADO',
    (SELECT id FROM empresas WHERE nombre = 'LLANTAS EXPRES'),
    'PRAIND', '2027-05-30', 1, 1
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
    '106900627', 'MAURICIO MONTANARI CORRALES',
    (SELECT id FROM empresas WHERE nombre = 'M BRISAS'),
    'PRAIND', '2027-05-30', 1, 1
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
    '304530380', 'FABIOLA CODERO EZPINOZA',
    (SELECT id FROM empresas WHERE nombre = 'MEDICO'),
    'IN_HOUSE', '2027-05-30', 1, 1
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

INSERT INTO contratistas (
    cedula, nombre, empresa_id, tipo_ingreso,
    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
) VALUES (
    '117590806', 'IVONNE MENDEZ GONZALES',
    (SELECT id FROM empresas WHERE nombre = 'MEDICO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '111080215', 'MELISSA CRUZ RIVAS',
    (SELECT id FROM empresas WHERE nombre = 'MEDICO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112070108', 'JOSE RAMON VEGA SALAS',
    (SELECT id FROM empresas WHERE nombre = 'MULTIPRO S.A.'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115360578', 'DIEGO ALBERTO ARCE MORA',
    (SELECT id FROM empresas WHERE nombre = 'NAVEGACION'),
    'PRAIND', '2027-05-30', 1, 1
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
    '109360055', 'CRISTIAN PERALTA LOPEZ',
    (SELECT id FROM empresas WHERE nombre = 'ORBE'),
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
    '116150109', 'MARCO MESÉN SANDI',
    (SELECT id FROM empresas WHERE nombre = 'ORBE'),
    'PRAIND', '2026-01-23', 0, 1
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
    '111590618', 'JOSE SALGADO MADRIZ',
    (SELECT id FROM empresas WHERE nombre = 'PROVISA'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '106860669', 'ALEXANDER CARMONA CALDERON',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112160947', 'ALLAN TORRES BONILLA',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '115990086', 'BERNALD FRANSISCO MONTERO TENORIO',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118290636', 'CRISTOFER SANDI ALFARO',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '114600268', 'JAVIER ALBERTO CHAVARRIA VARGAS',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110950040', 'JIMMY CARMONA JARA',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116960087', 'JUAN CASTILLO TENORIO',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112183947', 'JULIO TOMAS BONILLA',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118510218', 'KEYLOR RODRIGUEZ VARGAS',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '117190192', 'LUIS ALEJANDRO CARMONA SANCHEZ',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '116800686', 'RONEY VARGAS TENORIO',
    (SELECT id FROM empresas WHERE nombre = 'RAICES MERCADEO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '155811367627', 'CARLOS ANDINO ICABALCETA',
    (SELECT id FROM empresas WHERE nombre = 'RENTOKILL'),
    'PRAIND', '2027-05-30', 1, 1
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
    '501980365', 'ENRIQUE SEQUEIRA JAEN',
    (SELECT id FROM empresas WHERE nombre = 'RENTOKILL'),
    'PRAIND', '2027-05-30', 1, 1
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
    '105910657', 'LUIS GUILLERMO AGUILAR ALVAREZ',
    (SELECT id FROM empresas WHERE nombre = 'RENTOKILL'),
    'PRAIND', '2027-05-30', 1, 1
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
    '303040685', 'WILBERTH MATAMOROS CODERO',
    (SELECT id FROM empresas WHERE nombre = 'RENTOKILL'),
    'PRAIND', '2027-05-30', 1, 1
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
    '108850126', 'MANUEL MOLINA',
    (SELECT id FROM empresas WHERE nombre = 'SCR'),
    'PRAIND', '2027-05-30', 1, 1
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
    '109440975', 'JIMMY MADRIZ RAMIREZ',
    (SELECT id FROM empresas WHERE nombre = 'SIGMA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112390934', 'ALEXANDRA MUÑOZ SABORIO',
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
    '502810059', 'ANALIS DE LOS ANGELES CERDAS RODRIGUEZ',
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
    '115310864', 'FRANCINNY MARIA ROJAS MENA',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2026-08-26', 0, 1
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
    '115310854', 'FRANCINNY ROJAS MENA',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '305450949', 'HAZEL JOHANA ANGULO AGUILAR',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2027-05-30', 1, 1
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
    'PRAIND', '2028-01-22', 0, 1
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
    '105730076', 'JORGE ALBERTO VARGAS HIDALGO',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '204660078', 'LUIS ANTONIO VEGA CASTRO',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '401860114', 'MICHAEL CAMPOS CHAVES',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '701460646', 'MINDY MURILLO GUZMAN',
    (SELECT id FROM empresas WHERE nombre = 'SODEXO'),
    'PRAIND', '2027-05-30', 1, 1
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
    '109020921', 'YESENIA CARDENAS DURAN',
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
    '207380161', 'EDUARDO JOSHUE VILLALOBOS',
    (SELECT id FROM empresas WHERE nombre = 'STERICLEAN DE CENTRO AMERICA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '108950250', 'JOHNNY MORALES',
    (SELECT id FROM empresas WHERE nombre = 'STERICLEAN DE CENTRO AMERICA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '207300845', 'MAIKOL ANTONIO SANCHEZ MARIN',
    (SELECT id FROM empresas WHERE nombre = 'SUPRA CONTINENTAL INC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '205620481', 'MAURICIO CASTRO GUZMAN',
    (SELECT id FROM empresas WHERE nombre = 'TALLER GERSON'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '118650861', 'RUBEN CASTILLO ACUÑA',
    (SELECT id FROM empresas WHERE nombre = 'TECMAS GUATEMALA S.A'),
    'PRAIND', '2027-05-30', 1, 1
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
    '111350727', 'CARLOS NUÑEZ ZARATE',
    (SELECT id FROM empresas WHERE nombre = 'TRACTOMOTRIZ'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119010327', 'ELMER CANALES ROMERO',
    (SELECT id FROM empresas WHERE nombre = 'TRACTOMOTRIZ'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119720353', 'JOSE FERNANDEZ FALLAS',
    (SELECT id FROM empresas WHERE nombre = 'TRACTOMOTRIZ'),
    'PRAIND', '2027-05-30', 1, 1
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
    '119800380', 'JHOEL BERNARDO CARVAJAL',
    (SELECT id FROM empresas WHERE nombre = 'TRAUMA STORE'),
    'PRAIND', '2027-05-30', 1, 1
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
    '207130049', 'LUIS ARNOLDO RAMIREZ CHAVES',
    (SELECT id FROM empresas WHERE nombre = 'TRUCKSLOGIC'),
    'PRAIND', '2027-05-30', 1, 1
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
    '401750220', 'FELIX CASTILLO CHAVEZ',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '205890473', 'JUAN JOSE GUILLEN RIVERA',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '112890308', 'RIGOBERTO SOTO MARTINEZ',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110630696', 'WILBERT JOSE MESEN ESQUIVEL',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '109080479', 'WILLIAM RUTHERFORD SCOTT',
    (SELECT id FROM empresas WHERE nombre = 'VALERIA'),
    'PRAIND', '2027-05-30', 1, 1
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
    '110770657', 'DAVID GARCIA ARGUEDAS',
    (SELECT id FROM empresas WHERE nombre = 'WARDIAN'),
    'PRAIND', '2027-05-30', 1, 1
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
    '120010272', 'JEAN CARLOS ICABALCETA PARRALES',
    (SELECT id FROM empresas WHERE nombre = 'WARDIAN'),
    'PRAIND', '2027-05-30', 1, 1
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
    '113600853', 'JORGE GODINEZ CORDERO',
    (SELECT id FROM empresas WHERE nombre = 'WARDIAN'),
    'PRAIND', '2027-05-30', 1, 1
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
    '118430732', 'KENNETH JESUS CHAVES ARAYA',
    (SELECT id FROM empresas WHERE nombre = 'WARDIAN'),
    'IN_HOUSE', '2027-05-30', 1, 1
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
    '112370775', 'LUIS ARAYA VALVERDE',
    (SELECT id FROM empresas WHERE nombre = 'WARDIAN'),
    'PRAIND', '2027-05-30', 1, 1
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
    '305200649', 'SAMANDALEON BARAHONA',
    (SELECT id FROM empresas WHERE nombre = 'WARDIAN'),
    'PRAIND', '2027-05-30', 1, 1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    empresa_id = excluded.empresa_id,
    tipo_ingreso = excluded.tipo_ingreso,
    fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
    es_personal_ruta = excluded.es_personal_ruta,
    tiene_acceso = excluded.tiene_acceso;
