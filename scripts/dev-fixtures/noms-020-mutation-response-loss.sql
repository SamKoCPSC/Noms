-- NOMS-020 accepted-response-loss checks. Local test data only; NEVER production.
-- Verify com.noms.dev-environment=true on the exact Postgres container first.
-- Supply an existing local mock account: psql -v fixture_owner=<user UUID> -f ...
-- Insert-only, transactional, and independent of earlier recipe-action fixtures.
\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('noms.fixture_owner', :'fixture_owner', true);
DO $fixture$
DECLARE
    fixture_namespace CONSTANT text := 'NOMS020 CP8 mutation response loss 2026-10-04';
    fixture_owner uuid := current_setting('noms.fixture_owner')::uuid;
    fixture_recipe CONSTANT uuid := md5(fixture_namespace || ' recipe')::uuid;
    fixture_title CONSTANT text := 'CP8 Response loss – Local test only';
    fixture_snapshot jsonb;
    ordinal integer;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM users WHERE id = fixture_owner) THEN
        RAISE EXCEPTION 'An existing local mock account is required';
    END IF;
    -- Refuse replay after browser mutations rather than silently resetting state.
    IF EXISTS (SELECT 1 FROM recipes WHERE id = fixture_recipe) THEN
        RAISE EXCEPTION 'Response-loss fixture already exists; inspect retained state';
    END IF;
    INSERT INTO recipes (id, user_id, title, description, commentary,
        ingredients, instructions, visibility)
    VALUES (fixture_recipe, fixture_owner, fixture_title,
        'Synthetic local accepted-response-loss acceptance. Not a real recipe.',
        'Private synthetic fixture; do not publish publicly.',
        '[{"amount":"2","unit":"cups","name":"water"}]'::jsonb,
        '[{"text":"Synthetic second-version method.","sub_steps":[]}]'::jsonb,
        'private');
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
            'Synthetic local response-loss version ' || ordinal);
    END LOOP;
END
$fixture$;
COMMIT;
