-- NOMS-020 populated responsive acceptance. Local test data only; NEVER production.
-- Verify com.noms.dev-environment=true on the exact Postgres container first.
-- Requires the recipe-actions, recipe-history and public-profile fixtures.
-- Supply their existing mock owner: psql -v fixture_owner=<UUID> -f ...
-- Insert-only and transactional; existing recipes, profiles and drafts are untouched.
\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('noms.fixture_owner', :'fixture_owner', true);
DO $fixture$
DECLARE
    fixture_owner uuid := current_setting('noms.fixture_owner')::uuid;
    root_id CONSTANT uuid := md5('NOMS020 CP8 responsive acceptance 2026-10-01 root')::uuid;
    child_id CONSTANT uuid := md5('NOMS020 CP8 responsive acceptance 2026-10-01 child')::uuid;
    profile_namespace CONSTANT text := 'NOMS020 CP8 public profile 2026-09-30';
    actions_id CONSTANT uuid := md5('NOMS020 CP8 recipe actions 2026-09-30 recipe')::uuid;
    history_id CONSTANT uuid := md5('NOMS020 CP8 recipe history 2026-09-30 root')::uuid;
    ordinal integer;
    existing_name text;
    existing_owner uuid;
    existing_visibility text;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM users WHERE id = fixture_owner) THEN
        RAISE EXCEPTION 'An existing local mock owner is required';
    END IF;
    -- Refuse attaching unrelated data if a prerequisite fixture was replaced.
    -- Compare text outside SQL predicates: ParadeDB indexes multiple title fields.
    SELECT title, user_id, visibility INTO existing_name, existing_owner, existing_visibility
    FROM recipes WHERE id = actions_id;
    IF existing_name IS DISTINCT FROM 'CP8 Recipe actions – Local test only'
        OR existing_owner IS DISTINCT FROM fixture_owner OR existing_visibility IS DISTINCT FROM 'private' THEN
        RAISE EXCEPTION 'Expected recipe-action fixture missing or changed';
    END IF;
    SELECT title, user_id, visibility INTO existing_name, existing_owner, existing_visibility
    FROM recipes WHERE id = history_id;
    IF existing_name IS DISTINCT FROM 'CP8 History root – Local test only'
        OR existing_owner IS DISTINCT FROM fixture_owner OR existing_visibility IS DISTINCT FROM 'private' THEN
        RAISE EXCEPTION 'Expected recipe-history fixture missing or changed';
    END IF;
    SELECT name, owner_id, visibility INTO existing_name, existing_owner, existing_visibility
    FROM collections WHERE id = md5(profile_namespace || ' collection 1')::uuid;
    IF existing_name IS DISTINCT FROM 'CP8Profile Collection 01 – Local test only'
        OR existing_owner IS DISTINCT FROM md5(profile_namespace || ' owner')::uuid
        OR existing_visibility IS DISTINCT FROM 'public'
        OR (SELECT count(*) FROM recipes WHERE user_id = md5(profile_namespace || ' owner')::uuid
            AND visibility = 'public') <> 25 THEN
        RAISE EXCEPTION 'Expected public-profile fixture missing or changed';
    END IF;
    IF EXISTS (SELECT 1 FROM collections WHERE id = root_id) THEN
        SELECT name INTO existing_name FROM collections WHERE id = root_id;
        IF existing_name <> 'CP8 Responsive library' OR NOT EXISTS
            (SELECT 1 FROM collections WHERE id = root_id AND owner_id = fixture_owner
                AND parent_collection_id IS NULL AND visibility = 'private')
        THEN RAISE EXCEPTION 'Responsive root collision'; END IF;
    ELSE
        INSERT INTO collections (id, owner_id, name, description, visibility)
        VALUES (root_id, fixture_owner, 'CP8 Responsive library',
            'Synthetic local acceptance collection. Not real content.', 'private');
    END IF;
    IF EXISTS (SELECT 1 FROM collections WHERE id = child_id) THEN
        SELECT name INTO existing_name FROM collections WHERE id = child_id;
        IF existing_name <> 'CP8 Nested collection' OR NOT EXISTS
            (SELECT 1 FROM collections WHERE id = child_id AND owner_id = fixture_owner
                AND parent_collection_id = root_id AND visibility = 'private')
        THEN RAISE EXCEPTION 'Responsive child collision'; END IF;
    ELSE
        INSERT INTO collections (id, owner_id, parent_collection_id, name, description, visibility)
        VALUES (child_id, fixture_owner, root_id, 'CP8 Nested collection',
            'Synthetic nested and multi-membership acceptance.', 'private');
    END IF;
    INSERT INTO collection_recipes (collection_id, recipe_id, sort_order) VALUES
        (root_id, actions_id, 0), (child_id, actions_id, 0), (child_id, history_id, 1)
    ON CONFLICT DO NOTHING;
    FOR ordinal IN 1..25 LOOP
        SELECT title, user_id, visibility INTO existing_name, existing_owner, existing_visibility
        FROM recipes WHERE id = md5(profile_namespace || ' recipe ' || ordinal)::uuid;
        IF existing_name IS DISTINCT FROM 'CP8Profile Recipe ' || lpad(ordinal::text, 2, '0') || ' – Local test only'
            OR existing_owner IS DISTINCT FROM md5(profile_namespace || ' owner')::uuid
            OR existing_visibility IS DISTINCT FROM 'public' THEN
            RAISE EXCEPTION 'Public recipe fixture missing or changed';
        END IF;
        INSERT INTO collection_recipes (collection_id, recipe_id, sort_order)
        VALUES (md5(profile_namespace || ' collection 1')::uuid,
            md5(profile_namespace || ' recipe ' || ordinal)::uuid, ordinal - 1)
        ON CONFLICT DO NOTHING;
    END LOOP;
END
$fixture$;
COMMIT;
