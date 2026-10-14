-- NOMS-020 CP8 synthetic discovery fixtures. NEVER a production seed.
-- Explicit user approval: local public test data, 2026-09-30.
-- Before running, verify com.noms.dev-environment=true on the exact Postgres
-- container. Only deterministic fixture IDs are inserted; nothing is updated
-- or deleted. Existing private fixtures and mock account profiles stay intact.
\set ON_ERROR_STOP on
BEGIN;
DO $fixture$
DECLARE
    collection_owner CONSTANT uuid := 'da1ff4ae-c7b8-4ab5-96cb-ea982ddcad1f';
    fixture_namespace CONSTANT text := 'NOMS020 CP8 discovery catalog 2026-09-30';
    ordinal integer;
    cook_id uuid;
    fixture_recipe_id uuid;
    collection_id uuid;
    fixture_username text;
    fixture_title text;
    fixture_name text;
    existing_username text;
    existing_title text;
    existing_name text;
    existing_owner uuid;
    existing_visibility text;
    fixture_snapshot jsonb;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM users WHERE id = collection_owner) THEN
        RAISE EXCEPTION 'Expected existing local mock owner is missing';
    END IF;
    FOR ordinal IN 1..25 LOOP
        cook_id := md5(fixture_namespace || ' cook ' || ordinal)::uuid;
        fixture_recipe_id := md5(fixture_namespace || ' recipe ' || ordinal)::uuid;
        collection_id := md5(fixture_namespace || ' collection ' || ordinal)::uuid;
        fixture_username := 'cp8catalog-' || lpad(ordinal::text, 2, '0');
        fixture_title := 'CP8Catalog Recipe ' || lpad(ordinal::text, 2, '0') || ' – Local test only';
        fixture_name := 'CP8Catalog Collection ' || lpad(ordinal::text, 2, '0') || ' – Local test only';

        SELECT username INTO existing_username FROM users WHERE id = cook_id;
        IF FOUND THEN
            IF existing_username <> fixture_username THEN
                RAISE EXCEPTION 'Synthetic cook ID collision';
            END IF;
        ELSE
            INSERT INTO users (id, username, display_name, email, bio)
            VALUES (cook_id, fixture_username,
                'CP8Catalog Cook ' || lpad(ordinal::text, 2, '0') || ' – Synthetic',
                fixture_username || '@example.invalid',
                'Synthetic local test profile for NOMS-020 pagination. Not a real person.');
        END IF;

        -- Compare indexed text outside the query to avoid BM25 field-alias
        -- ambiguity on databases with both word and partial-name indexes.
        SELECT user_id, title, visibility INTO existing_owner, existing_title, existing_visibility
        FROM recipes WHERE id = fixture_recipe_id;
        IF FOUND THEN
            IF existing_owner <> cook_id OR existing_title <> fixture_title OR existing_visibility <> 'public' THEN
                RAISE EXCEPTION 'Synthetic recipe ID collision or changed fixture';
            END IF;
        ELSE
            INSERT INTO recipes (id, user_id, title, description, commentary,
                prep_time_minutes, cook_time_minutes, servings, ingredients, instructions,
                visibility, created_at, updated_at)
            VALUES (fixture_recipe_id, cook_id, fixture_title,
                'Synthetic public recipe for local discovery acceptance. No real community content.',
                'Test fixture only.', ordinal, 10, 2,
                '[{"amount":"2","unit":"cups","name":"water"}]'::jsonb,
                '[{"text":"Synthetic test method; not a real recipe.","sub_steps":[]}]'::jsonb,
                'public', '2026-09-30 00:00:00+00'::timestamptz + ordinal * interval '1 minute',
                '2026-09-30 00:00:00+00'::timestamptz + ordinal * interval '1 minute');
        END IF;
        SELECT jsonb_build_object('title', title, 'description', description,
            'commentary', commentary, 'prep_time_minutes', prep_time_minutes,
            'cook_time_minutes', cook_time_minutes, 'servings', servings,
            'ingredients', ingredients, 'instructions', instructions,
            'equipment', equipment, 'images', images, 'tags', '[]'::jsonb)
        INTO fixture_snapshot FROM recipes WHERE id = fixture_recipe_id;
        INSERT INTO recipe_versions (id, recipe_id, version_number, snapshot, notes)
        VALUES (md5(fixture_namespace || ' version ' || ordinal)::uuid,
            fixture_recipe_id, 1, fixture_snapshot, 'Synthetic local test version.')
        ON CONFLICT (recipe_id, version_number) DO NOTHING;
        INSERT INTO recipe_tags (recipe_id, tag) VALUES (fixture_recipe_id, 'CP8Catalog')
        ON CONFLICT DO NOTHING;

        SELECT owner_id, name, visibility INTO existing_owner, existing_name, existing_visibility
        FROM collections WHERE id = collection_id;
        IF FOUND THEN
            IF existing_owner <> collection_owner OR existing_name <> fixture_name OR existing_visibility <> 'private' THEN
                RAISE EXCEPTION 'Synthetic collection ID collision or changed fixture';
            END IF;
        ELSE
            INSERT INTO collections (id, owner_id, name, description, visibility)
            VALUES (collection_id, collection_owner, fixture_name,
                'Synthetic private collection for local dense Search acceptance.', 'private');
        END IF;
    END LOOP;
    IF (SELECT count(*) FROM users WHERE id IN (
        SELECT md5(fixture_namespace || ' cook ' || item)::uuid FROM generate_series(1, 25) item)) <> 25
        OR (SELECT count(*) FROM recipes WHERE visibility = 'public' AND id IN (
        SELECT md5(fixture_namespace || ' recipe ' || item)::uuid FROM generate_series(1, 25) item)) <> 25
        OR (SELECT count(*) FROM collections WHERE owner_id = collection_owner AND visibility = 'private' AND id IN (
        SELECT md5(fixture_namespace || ' collection ' || item)::uuid FROM generate_series(1, 25) item)) <> 25
    THEN
        RAISE EXCEPTION 'Synthetic fixture count/ownership/visibility validation failed';
    END IF;
END
$fixture$;
COMMIT;
