-- NOMS-020 final Delete acceptance fixture. Local test data only; NEVER production.
-- Verify com.noms.dev-environment=true on the exact Postgres container first.
-- Supply an existing mock owner: psql -v fixture_owner=<UUID> -f ...
-- Creating this fixture does NOT authorize pressing the final Delete action.
-- Get action-time confirmation for this exact recipe before destructive browser checks.
-- Insert-only and transactional; no original account, recipe or draft is changed.
\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('noms.fixture_owner', :'fixture_owner', true);
DO $fixture$
DECLARE
    fixture_namespace CONSTANT text := 'NOMS020 CP8 disposable recipe 2026-10-01';
    fixture_owner uuid := current_setting('noms.fixture_owner')::uuid;
    fixture_id CONSTANT uuid := md5(fixture_namespace || ' recipe')::uuid;
    fixture_title CONSTANT text := 'CP8 Disposable recipe – Local acceptance only';
    existing_title text;
    existing_owner uuid;
    existing_visibility text;
    fixture_snapshot jsonb;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM users WHERE id = fixture_owner) THEN
        RAISE EXCEPTION 'An existing local mock account is required';
    END IF;
    SELECT title, user_id, visibility INTO existing_title, existing_owner, existing_visibility
    FROM recipes WHERE id = fixture_id;
    IF FOUND THEN
        IF existing_title IS DISTINCT FROM fixture_title OR existing_owner IS DISTINCT FROM fixture_owner
            OR existing_visibility IS DISTINCT FROM 'private' THEN
            RAISE EXCEPTION 'Disposable recipe ID collision or changed fixture';
        END IF;
    ELSE
        INSERT INTO recipes (id, user_id, title, description, commentary, ingredients, instructions, visibility)
        VALUES (fixture_id, fixture_owner, fixture_title,
            'Expendable synthetic recipe for final Delete acceptance. No real content.',
            'Delete only after explicit action-time confirmation.',
            '[{"amount":"1","unit":"cup","name":"water"}]'::jsonb,
            '[{"text":"Synthetic test step.","sub_steps":[]}]'::jsonb, 'private');
    END IF;
    -- This fixture must not be linked into other acceptance work before deletion.
    IF EXISTS (SELECT 1 FROM collection_recipes WHERE recipe_id = fixture_id)
        OR EXISTS (SELECT 1 FROM recipe_drafts WHERE recipe_id = fixture_id OR forked_from_recipe_id = fixture_id)
        OR EXISTS (SELECT 1 FROM fork_relationships WHERE original_recipe_id = fixture_id OR forked_recipe_id = fixture_id) THEN
        RAISE EXCEPTION 'Disposable recipe has dependencies; do not use it for Delete acceptance';
    END IF;
    SELECT jsonb_build_object('title', title, 'description', description, 'commentary', commentary,
        'ingredients', ingredients, 'instructions', instructions, 'equipment', equipment,
        'images', images, 'tags', '[]'::jsonb) INTO fixture_snapshot
    FROM recipes WHERE id = fixture_id;
    INSERT INTO recipe_versions (id, recipe_id, version_number, snapshot, notes)
    VALUES (md5(fixture_namespace || ' version 1')::uuid, fixture_id, 1,
        fixture_snapshot, 'Expendable synthetic Delete acceptance version')
    ON CONFLICT (recipe_id, version_number) DO NOTHING;
END
$fixture$;
COMMIT;
