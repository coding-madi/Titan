-- Add migration script here


-- First, define the ENUM type for the 'status' column
CREATE TYPE log_status AS ENUM ('active', 'inactive', 'flush_pending');

-- Now, create the active_log table
CREATE TABLE active_log (
    ID BIGSERIAL PRIMARY KEY,
    log_name VARCHAR(255) NOT NULL, -- Assuming log_name should not be null
    log_path VARCHAR(255),
    "group" VARCHAR(255),            -- "group" is a reserved keyword, so it's quoted
    status log_status DEFAULT 'active', -- Use the custom ENUM type with a default value
    rotated_time TIMESTAMP WITH TIME ZONE -- Store timestamp with timezone information
);

