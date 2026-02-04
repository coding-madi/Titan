-- ========= log_groups =========
CREATE TABLE log_groups (
                            id              SERIAL PRIMARY KEY,
                            service_name    VARCHAR(255) NOT NULL,
                            container_name  VARCHAR(255),   -- NULL  ⇒  fall back to log_name grouping
                            log_name        VARCHAR(255),   -- NULL  when container_name IS NOT NULL
                            description     TEXT,
                            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
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
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at := NOW();
RETURN NEW;
END; $$ LANGUAGE plpgsql;

CREATE TRIGGER trg_log_groups_set_updated_at
    BEFORE UPDATE ON log_groups
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();