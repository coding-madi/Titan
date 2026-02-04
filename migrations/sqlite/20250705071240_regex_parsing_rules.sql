-- Add migration script here
CREATE TABLE regex_parsing_rules (
                                     id                 INTEGER PRIMARY KEY AUTOINCREMENT,
                                     log_group_id       INTEGER NOT NULL
                                         REFERENCES log_groups(id)
                                             ON DELETE CASCADE,
                                     column_name        TEXT    NOT NULL,
                                     regex_pattern      TEXT    NOT NULL,     -- include named capture groups
                                     struct_field_name  TEXT,                 -- NULL ⇒ no struct wrapper
                                     active             INTEGER NOT NULL DEFAULT 1,   -- 1 = true, 0 = false
                                     created_at         DATETIME DEFAULT CURRENT_TIMESTAMP,
                                     updated_at         DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER trg_regex_rules_set_updated_at
    AFTER UPDATE ON regex_parsing_rules
    FOR EACH ROW
BEGIN
    UPDATE regex_parsing_rules
    SET updated_at = CURRENT_TIMESTAMP
    WHERE id = OLD.id;
END;
