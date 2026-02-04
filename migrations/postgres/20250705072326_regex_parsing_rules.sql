-- ========= regex_parsing_rules =========
CREATE TABLE regex_parsing_rules (
                                     id                 SERIAL PRIMARY KEY,
                                     log_group_id       INTEGER NOT NULL
                                         REFERENCES log_groups(id)
                                             ON DELETE CASCADE,
                                     column_name        VARCHAR(255) NOT NULL,
                                     regex_pattern      TEXT         NOT NULL,  -- include named capture groups
                                     struct_field_name  VARCHAR(255),
                                     active             BOOLEAN      NOT NULL DEFAULT TRUE,
                                     created_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
                                     updated_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TRIGGER trg_regex_rules_set_updated_at
    BEFORE UPDATE ON regex_parsing_rules
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();