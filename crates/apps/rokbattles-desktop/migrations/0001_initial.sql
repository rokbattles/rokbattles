CREATE TABLE acknowledged (
    mail_id TEXT PRIMARY KEY NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TABLE sources (
    path TEXT PRIMARY KEY NOT NULL,
    mail_id TEXT,
    pending INTEGER NOT NULL CHECK(pending IN (0, 1)),
    size INTEGER CHECK(size >= 0),
    modified_ms INTEGER CHECK(modified_ms >= 0),
    attempts INTEGER NOT NULL CHECK(attempts BETWEEN 0 AND 10),
    not_before_ms INTEGER CHECK(not_before_ms >= 0),
    CHECK((size IS NULL) = (modified_ms IS NULL)),
    CHECK(pending = 1 OR (attempts = 0 AND not_before_ms IS NULL))
) STRICT;

CREATE INDEX sources_mail_id ON sources(mail_id);

CREATE TABLE upload_control (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    api_backoff_until_ms INTEGER CHECK(api_backoff_until_ms >= 0)
) STRICT;

INSERT INTO upload_control VALUES (1, NULL);
