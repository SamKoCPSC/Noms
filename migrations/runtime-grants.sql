\set ON_ERROR_STOP on

-- The schema-owning deployment role applies DDL and extensions. The runtime
-- role receives only the data-plane privileges used by the application.
-- Refuse to pretend that GRANT/REVOKE is an isolation boundary when the role
-- has administrative attributes, inherits another role, or already owns an
-- application object (ownership carries implicit privileges).
SELECT format(
    $guard$
    DO $runtime_role_guard$
    DECLARE
        checked_role pg_roles%%ROWTYPE;
    BEGIN
        SELECT * INTO STRICT checked_role
        FROM pg_roles
        WHERE rolname = %L;

        IF checked_role.rolsuper
            OR checked_role.rolcreaterole
            OR checked_role.rolcreatedb
            OR checked_role.rolreplication
            OR checked_role.rolbypassrls
        THEN
            RAISE EXCEPTION 'runtime database role has administrative attributes';
        END IF;

        IF EXISTS (
            SELECT 1 FROM pg_auth_members WHERE member = checked_role.oid
        ) THEN
            RAISE EXCEPTION 'runtime database role must not inherit or assume another role';
        END IF;

        IF EXISTS (
            SELECT 1
            FROM pg_namespace
            WHERE nspname = 'public' AND nspowner = checked_role.oid
        ) OR EXISTS (
            SELECT 1
            FROM pg_class AS relation
            JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
            WHERE namespace.nspname = 'public' AND relation.relowner = checked_role.oid
        ) OR EXISTS (
            SELECT 1
            FROM pg_proc AS routine
            JOIN pg_namespace AS namespace ON namespace.oid = routine.pronamespace
            WHERE namespace.nspname = 'public' AND routine.proowner = checked_role.oid
        ) OR EXISTS (
            SELECT 1
            FROM pg_type AS data_type
            JOIN pg_namespace AS namespace ON namespace.oid = data_type.typnamespace
            WHERE namespace.nspname = 'public' AND data_type.typowner = checked_role.oid
        ) THEN
            RAISE EXCEPTION 'runtime database role must not own application objects';
        END IF;
    END
    $runtime_role_guard$;
    $guard$,
    :'runtime_role'
) \gexec

REVOKE TEMPORARY ON DATABASE :"runtime_database" FROM PUBLIC;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
REVOKE ALL ON SCHEMA public FROM :"runtime_role";
GRANT CONNECT ON DATABASE :"runtime_database" TO :"runtime_role";
GRANT USAGE ON SCHEMA public TO :"runtime_role";

REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA public FROM :"runtime_role";
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO :"runtime_role";

REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public FROM :"runtime_role";
GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO :"runtime_role";

ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO :"runtime_role";
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO :"runtime_role";
