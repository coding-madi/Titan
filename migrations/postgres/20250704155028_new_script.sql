CREATE TABLE schema (
                        id           BIGSERIAL PRIMARY KEY,
                        flight_name  TEXT NOT NULL,
                        schema       BYTEA NOT NULL,
                        created_at   TIMESTAMPTZ DEFAULT NOW(),
                        updated_at   TIMESTAMPTZ DEFAULT NOW()
);
