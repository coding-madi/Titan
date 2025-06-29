-- Add migration script here
CREATE TABLE schema (
                        id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                        flight_name  TEXT NOT NULL,
                        schema       BYTEA NOT NULL,
                        created_at   TIMESTAMPTZ DEFAULT now(),
                        updated_at   TIMESTAMPTZ DEFAULT now()
);
