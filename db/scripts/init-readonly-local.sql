-- Create a read-only role
CREATE ROLE readonly;

-- Create a user that inherits the read-only role
CREATE USER readonly_user WITH PASSWORD 'password' IN ROLE readonly;

-- Grant connect permission to the database
GRANT CONNECT ON DATABASE icon_api TO readonly;

-- Grant usage on schema
GRANT USAGE ON SCHEMA public TO readonly;

-- Grant SELECT permissions only
GRANT SELECT ON ALL TABLES IN SCHEMA public TO readonly;

-- Make sure future tables also get SELECT permissions
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO readonly;

-- Optionally set session variables to prevent writes even if permissions change
ALTER ROLE readonly SET default_transaction_read_only = ON;
