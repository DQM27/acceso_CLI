
CREATE VIRTUAL TABLE contratistas_fts USING fts5(
    cedula, nombre,
    content='contratistas', content_rowid='id',
    tokenize='trigram case_sensitive 0 remove_diacritics 1'
);
CREATE VIRTUAL TABLE empresas_fts USING fts5(
    nombre,
    content='empresas', content_rowid='id',
    tokenize='trigram case_sensitive 0 remove_diacritics 1'
);
CREATE VIRTUAL TABLE usuarios_fts USING fts5(
    cedula, nombre,
    content='usuarios', content_rowid='id',
    tokenize='trigram case_sensitive 0 remove_diacritics 1'
);

CREATE TRIGGER contratistas_fts_ai AFTER INSERT ON contratistas BEGIN
    INSERT INTO contratistas_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;
CREATE TRIGGER contratistas_fts_ad AFTER DELETE ON contratistas BEGIN
    INSERT INTO contratistas_fts(contratistas_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
END;
CREATE TRIGGER contratistas_fts_au AFTER UPDATE ON contratistas BEGIN
    INSERT INTO contratistas_fts(contratistas_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
    INSERT INTO contratistas_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;

CREATE TRIGGER empresas_fts_ai AFTER INSERT ON empresas BEGIN
    INSERT INTO empresas_fts(rowid, nombre) VALUES (new.id, new.nombre);
END;
CREATE TRIGGER empresas_fts_ad AFTER DELETE ON empresas BEGIN
    INSERT INTO empresas_fts(empresas_fts, rowid, nombre)
    VALUES ('delete', old.id, old.nombre);
END;
CREATE TRIGGER empresas_fts_au AFTER UPDATE ON empresas BEGIN
    INSERT INTO empresas_fts(empresas_fts, rowid, nombre)
    VALUES ('delete', old.id, old.nombre);
    INSERT INTO empresas_fts(rowid, nombre) VALUES (new.id, new.nombre);
END;

CREATE TRIGGER usuarios_fts_ai AFTER INSERT ON usuarios BEGIN
    INSERT INTO usuarios_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;
CREATE TRIGGER usuarios_fts_ad AFTER DELETE ON usuarios BEGIN
    INSERT INTO usuarios_fts(usuarios_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
END;
CREATE TRIGGER usuarios_fts_au AFTER UPDATE ON usuarios BEGIN
    INSERT INTO usuarios_fts(usuarios_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
    INSERT INTO usuarios_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;

INSERT INTO contratistas_fts(contratistas_fts) VALUES ('rebuild');
INSERT INTO empresas_fts(empresas_fts) VALUES ('rebuild');
INSERT INTO usuarios_fts(usuarios_fts) VALUES ('rebuild');
