-- NOMS-020 CP8 dense public profile. Local test data only; NEVER production.
-- User approved additional local test data on 2026-09-30.
-- Verify com.noms.dev-environment=true on the exact Postgres container first.
-- Insert-only, transactional and idempotent. No existing content is changed.
\set ON_ERROR_STOP on
BEGIN;
DO $fixture$
DECLARE
    fixture_namespace CONSTANT text := 'NOMS020 CP8 public profile 2026-09-30';
    fixture_owner CONSTANT uuid := md5(fixture_namespace || ' owner')::uuid;
    ordinal integer;
    fixture_recipe_id uuid;
    fixture_collection_id uuid;
    fixture_visibility text;
    fixture_title text;
    fixture_name text;
    existing_owner uuid;
    existing_text text;
    existing_visibility text;
    fixture_snapshot jsonb;
BEGIN
    SELECT username INTO existing_text FROM users WHERE id = fixture_owner;
    IF FOUND THEN
        IF existing_text <> 'cp8profile' THEN
            RAISE EXCEPTION 'Synthetic profile owner ID collision';
        END IF;
    ELSE
        INSERT INTO users (id, username, display_name, email, bio)
        VALUES (fixture_owner, 'cp8profile', 'CP8 Profile – Synthetic',
            'cp8profile@example.invalid', 'Synthetic local pagination profile. Not a real person.');
    END IF;
    FOR ordinal IN 1..27 LOOP
        fixture_visibility := CASE WHEN ordinal <= 25 THEN 'public'
            WHEN ordinal = 26 THEN 'private' ELSE 'unlisted' END;
        fixture_recipe_id := md5(fixture_namespace || ' recipe ' || ordinal)::uuid;
        fixture_collection_id := md5(fixture_namespace || ' collection ' || ordinal)::uuid;
        fixture_title := 'CP8Profile Recipe ' || lpad(ordinal::text, 2, '0') || ' – Local test only';
        fixture_name := 'CP8Profile Collection ' || lpad(ordinal::text, 2, '0') || ' – Local test only';

        -- Compare indexed fields outside the SELECT to avoid BM25 alias ambiguity.
        SELECT user_id, title, visibility INTO existing_owner, existing_text, existing_visibility
        FROM recipes WHERE id = fixture_recipe_id;
        IF FOUND THEN
            IF existing_owner <> fixture_owner OR existing_text <> fixture_title
                OR existing_visibility <> fixture_visibility THEN
                RAISE EXCEPTION 'Synthetic profile recipe ID collision or changed fixture';
            END IF;
        ELSE
            INSERT INTO recipes (id, user_id, title, description, commentary,
                prep_time_minutes, cook_time_minutes, servings, ingredients, instructions,
                visibility, created_at, updated_at)
            VALUES (fixture_recipe_id, fixture_owner, fixture_title,
                'Synthetic local public-profile acceptance data. Not a real recipe.',
                'Test fixture only.', ordinal, 10, 2,
                '[{"amount":"2","unit":"cups","name":"water"}]'::jsonb,
                '[{"text":"Synthetic test method.","sub_steps":[]}]'::jsonb,
                fixture_visibility,
                '2026-09-30 01:00:00+00'::timestamptz + ordinal * interval '1 minute',
                '2026-09-30 01:00:00+00'::timestamptz + ordinal * interval '1 minute');
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

        SELECT owner_id, name, visibility INTO existing_owner, existing_text, existing_visibility
        FROM collections WHERE id = fixture_collection_id;
        IF FOUND THEN
            IF existing_owner <> fixture_owner OR existing_text <> fixture_name
                OR existing_visibility <> fixture_visibility THEN
                RAISE EXCEPTION 'Synthetic profile collection ID collision or changed fixture';
            END IF;
        ELSE
            INSERT INTO collections (id, owner_id, name, description, visibility, created_at, updated_at)
            VALUES (fixture_collection_id, fixture_owner, fixture_name,
                'Synthetic local public-profile collection. No real community content.', fixture_visibility,
                '2026-09-30 01:00:00+00'::timestamptz + ordinal * interval '1 minute',
                '2026-09-30 01:00:00+00'::timestamptz + ordinal * interval '1 minute');
        END IF;
    END LOOP;
    IF (SELECT count(*) FROM recipes WHERE user_id = fixture_owner AND visibility = 'public') <> 25
        OR (SELECT count(*) FROM collections WHERE owner_id = fixture_owner AND visibility = 'public') <> 25
        OR (SELECT count(*) FROM recipes WHERE user_id = fixture_owner) <> 27
        OR (SELECT count(*) FROM collections WHERE owner_id = fixture_owner) <> 27 THEN
        RAISE EXCEPTION 'Synthetic profile visibility/count validation failed';
    END IF;
END
$fixture$;
COMMIT;
