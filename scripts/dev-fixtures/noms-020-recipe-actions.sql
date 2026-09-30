-- NOMS-020 recipe-action acceptance. Local test data only; NEVER production.
-- Verify com.noms.dev-environment=true on the exact Postgres container first.
-- Supply an existing local mock account: psql -v fixture_owner=<user UUID> -f ...
-- Insert-only and transactional; no account or existing recipe is changed.
\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('noms.fixture_owner', :'fixture_owner', true);
DO $fixture$
DECLARE
    fixture_namespace CONSTANT text := 'NOMS020 CP8 recipe actions 2026-09-30';
    fixture_owner uuid := current_setting('noms.fixture_owner')::uuid;
    fixture_recipe CONSTANT uuid := md5(fixture_namespace || ' recipe')::uuid;
    fixture_title CONSTANT text := 'CP8 Recipe actions – Local test only';
    existing_owner uuid;
    existing_title text;
    existing_visibility text;
    fixture_snapshot jsonb;
    ordinal integer;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM users WHERE id = fixture_owner) THEN
        RAISE EXCEPTION 'An existing local mock account is required';
    END IF;
    SELECT user_id, title, visibility INTO existing_owner, existing_title, existing_visibility
    FROM recipes WHERE id = fixture_recipe;
    IF FOUND THEN
        IF existing_owner <> fixture_owner OR existing_title <> fixture_title
            OR existing_visibility <> 'private' THEN
            RAISE EXCEPTION 'Recipe action fixture ID collision or changed fixture';
        END IF;
    ELSE
        INSERT INTO recipes (id, user_id, title, description, commentary,
            ingredients, instructions, visibility)
        VALUES (fixture_recipe, fixture_owner, fixture_title,
            'Synthetic local recipe-action acceptance. Not a real recipe.',
            'Private synthetic fixture; do not publish.',
            '[{"amount":"2","unit":"cups","name":"water"}]'::jsonb,
            '[{"text":"Synthetic second-version method.","sub_steps":[]}]'::jsonb,
            'private');
    END IF;
    SELECT jsonb_build_object('title', title, 'description', description,
        'commentary', commentary, 'prep_time_minutes', prep_time_minutes,
        'cook_time_minutes', cook_time_minutes, 'servings', servings,
        'ingredients', ingredients, 'instructions', instructions,
        'equipment', equipment, 'images', images, 'tags', '[]'::jsonb)
    INTO fixture_snapshot FROM recipes WHERE id = fixture_recipe;
    FOR ordinal IN 1..2 LOOP
        INSERT INTO recipe_versions (id, recipe_id, version_number, snapshot, notes)
        VALUES (md5(fixture_namespace || ' version ' || ordinal)::uuid,
            fixture_recipe, ordinal,
            CASE WHEN ordinal = 1 THEN jsonb_set(fixture_snapshot, '{instructions}',
                '[{"text":"Synthetic first-version method.","sub_steps":[]}]'::jsonb)
                ELSE fixture_snapshot END,
            'Synthetic local recipe-action version ' || ordinal)
        ON CONFLICT (recipe_id, version_number) DO NOTHING;
    END LOOP;
END
$fixture$;
COMMIT;
