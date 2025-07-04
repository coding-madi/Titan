-- Add migration script here
CREATE TABLE schema (
                        id           INTEGER PRIMARY KEY,
                        flight_name  TEXT NOT NULL,
                        schema       BLOB NOT NULL,
                        created_at   TEXT DEFAULT CURRENT_TIMESTAMP,
                        updated_at   TEXT DEFAULT CURRENT_TIMESTAMP
);
