-- Add migration script here
-- ========= log_groups =========
CREATE TABLE log_groups (
                            id              INTEGER PRIMARY KEY AUTOINCREMENT,
                            service_name    TEXT NOT NULL,
                            container_name  TEXT,          -- NULL  ⇒  fall back to log_name grouping
                            log_name        TEXT,          -- NULL  when container_name IS NOT NULL
                            description     TEXT,
                            created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
                            updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
                            CHECK (
                                (container_name IS NOT NULL AND log_name IS NULL) OR
                                (container_name IS NULL AND log_name IS NOT NULL)
                                )
);

/* Uniqueness rules */
CREATE UNIQUE INDEX idx_log_groups_service_container
    ON log_groups (service_name, container_name)
    WHERE container_name IS NOT NULL;

CREATE UNIQUE INDEX idx_log_groups_service_logname
    ON log_groups (service_name, log_name)
    WHERE container_name IS NULL;

-- ——— auto‑update updated_at
CREATE TRIGGER trg_log_groups_set_updated_at
    AFTER UPDATE ON log_groups
    FOR EACH ROW
BEGIN
    UPDATE log_groups
    SET updated_at = CURRENT_TIMESTAMP
    WHERE id = OLD.id;
END;