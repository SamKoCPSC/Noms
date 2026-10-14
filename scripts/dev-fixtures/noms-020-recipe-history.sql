-- Approved NOMS-020 local acceptance data only; NEVER production.
-- Verify com.noms.dev-environment=true on the exact Postgres container first.
-- Supply an existing mock account: psql -v fixture_owner=<UUID> -f ...
-- Insert-only, transactional and idempotent. No existing content is modified.
\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('noms.fixture_owner', :'fixture_owner', true);
DO $fixture$
DECLARE
    fixture_namespace CONSTANT text := 'NOMS020 CP8 recipe history 2026-09-30';
    fixture_owner uuid := current_setting('noms.fixture_owner')::uuid;
    fixture_username text;
    root_id CONSTANT uuid := md5(fixture_namespace || ' root')::uuid;
    fork_id CONSTANT uuid := md5(fixture_namespace || ' fork')::uuid;
    fixture_recipe_id uuid;
    recipe_title text;
    existing_owner uuid;
    existing_title text;
    existing_visibility text;
    fixture_snapshot jsonb;
    recipe_kind text;
    ordinal integer;
BEGIN
    SELECT username INTO fixture_username FROM users WHERE id = fixture_owner;
    IF NOT FOUND THEN RAISE EXCEPTION 'An existing local mock owner is required'; END IF;
    FOREACH recipe_kind IN ARRAY ARRAY['root', 'fork'] LOOP
        fixture_recipe_id := CASE WHEN recipe_kind = 'root' THEN root_id ELSE fork_id END;
        recipe_title := 'CP8 History ' || recipe_kind || ' – Local test only';
        SELECT user_id, title, visibility INTO existing_owner, existing_title, existing_visibility
        FROM recipes WHERE id = fixture_recipe_id;
        IF FOUND THEN
            IF existing_owner <> fixture_owner OR existing_title <> recipe_title
                OR existing_visibility <> 'private' THEN
                RAISE EXCEPTION 'History fixture collision or changed fixture';
            END IF;
        ELSE
            INSERT INTO recipes (id, user_id, title, description, commentary,
                ingredients, instructions, visibility)
            VALUES (fixture_recipe_id, fixture_owner, recipe_title,
                CASE WHEN recipe_kind = 'root' THEN 'Synthetic history version 30'
                    ELSE 'Synthetic history fork' END,
                'Private synthetic acceptance data. Not a real recipe.',
                '[{"amount":"1","unit":"cup","name":"water"}]'::jsonb,
                '[{"text":"Synthetic test method.","sub_steps":[]}]'::jsonb, 'private');
        END IF;
        SELECT jsonb_build_object('title', title, 'description', description,
            'commentary', commentary, 'prep_time_minutes', prep_time_minutes,
            'cook_time_minutes', cook_time_minutes, 'servings', servings,
            'ingredients', ingredients, 'instructions', instructions,
            'equipment', equipment, 'images', images, 'tags', '[]'::jsonb)
        INTO fixture_snapshot FROM recipes WHERE id = fixture_recipe_id;
        FOR ordinal IN 1..(CASE WHEN recipe_kind = 'root' THEN 30 ELSE 1 END) LOOP
            INSERT INTO recipe_versions (id, recipe_id, version_number, snapshot, notes)
            VALUES (md5(fixture_namespace || recipe_kind || ' version ' || ordinal)::uuid,
                fixture_recipe_id, ordinal,
                CASE WHEN recipe_kind = 'root' THEN jsonb_set(fixture_snapshot, '{description}',
                    to_jsonb('Synthetic history version ' || ordinal)) ELSE fixture_snapshot END,
                'Synthetic local history ' || recipe_kind || ' v' || ordinal)
            ON CONFLICT (recipe_id, version_number) DO NOTHING;
        END LOOP;
    END LOOP;
    INSERT INTO fork_relationships (id, original_recipe_id, forked_recipe_id, forked_by,
        original_recipe_key, forked_recipe_key, forked_version_number, original_title,
        forked_title, original_owner_id, original_owner_username, original_visibility,
        forked_owner_id, forked_owner_username, forked_visibility, message)
    VALUES (md5(fixture_namespace || ' relationship')::uuid, root_id, fork_id, fixture_owner,
        root_id, fork_id, 30, 'CP8 History root – Local test only',
        'CP8 History fork – Local test only', fixture_owner, fixture_username, 'private',
        fixture_owner, fixture_username, 'private', 'Synthetic local lineage branch')
    ON CONFLICT (id) DO NOTHING;
    IF (SELECT count(*) FROM recipe_versions WHERE recipe_id = root_id) <> 30
        OR (SELECT count(*) FROM recipe_versions WHERE recipe_id = fork_id) <> 1 THEN
        RAISE EXCEPTION 'Synthetic history shape validation failed';
    END IF;
END
$fixture$;
COMMIT;
