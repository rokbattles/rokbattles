CREATE TABLE prepared_mail (
    mail_id TEXT PRIMARY KEY NOT NULL,
    bytes BLOB NOT NULL CHECK(length(bytes) BETWEEN 1 AND 26214400)
) STRICT;
