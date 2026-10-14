-- NOMS-020 checkpoint-8 local test data, never a production seed.
-- Run only against the isolated development Postgres container after checking
-- its com.noms.dev-environment=true label and the private mock-user template.
-- Insert-only and idempotent for this instance. The private rows are retained
-- so browser pagination, concurrency and permission checks are reproducible.
\set ON_ERROR_STOP on
BEGIN;

DO $fixture$
DECLARE
    fixture_id CONSTANT uuid := 'aef72aa2-2c1c-4576-b253-14e0d531e634';
    owner_id CONSTANT uuid := 'da1ff4ae-c7b8-4ab5-96cb-ea982ddcad1f';
    template_id CONSTANT uuid := '490fa456-22d9-4f99-83f2-a7721184bcca';
    fixture_title CONSTANT text := 'NOMS020 CP8 dense graph fixture 2026-09-29';
    fork_id uuid;
    fork_title text;
    fork_number integer;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM recipes WHERE id = template_id AND user_id = owner_id AND visibility = 'private'
    ) OR NOT EXISTS (
        SELECT 1 FROM recipe_versions WHERE recipe_id = template_id AND version_number = 2
    ) THEN
        RAISE EXCEPTION 'Expected private NOMS-020 mock-account template is missing';
    END IF;

    IF EXISTS (SELECT 1 FROM recipes WHERE id = fixture_id) THEN
        RAISE NOTICE 'Dense graph fixture already exists; no rows inserted';
        RETURN;
    END IF;

    INSERT INTO recipes (
        id, user_id, title, description, commentary,
        prep_time_minutes, cook_time_minutes, servings,
        ingredients, instructions, equipment, images, visibility
    )
    SELECT fixture_id, owner_id, fixture_title,
           'Dense graph version 32 of 32', commentary,
           prep_time_minutes, cook_time_minutes, servings,
           ingredients, instructions, equipment, '[]'::jsonb, 'private'
    FROM recipes WHERE id = template_id;

    INSERT INTO recipe_versions (recipe_id, version_number, snapshot, notes, created_at)
    SELECT fixture_id, generated.version_number,
           jsonb_set(
               jsonb_set(
                   jsonb_set(template.snapshot, '{title}', to_jsonb(fixture_title)),
                   '{description}', to_jsonb(format('Dense graph version %s of 32', generated.version_number))
               ),
               '{images}', '[]'::jsonb
           ),
           format('Private graph fixture version %s', generated.version_number),
           now() - (33 - generated.version_number) * interval '1 minute'
    FROM recipe_versions AS template
    CROSS JOIN generate_series(1, 32) AS generated(version_number)
    WHERE template.recipe_id = template_id AND template.version_number = 2;

    FOR fork_number IN 1..30 LOOP
        fork_id := md5(fixture_id::text || ':fork:' || fork_number)::uuid;
        fork_title := format('NOMS020 CP8 graph fork %s', lpad(fork_number::text, 2, '0'));

        INSERT INTO recipes (
            id, user_id, title, description, commentary,
            prep_time_minutes, cook_time_minutes, servings,
            ingredients, instructions, equipment, images, visibility
        )
        SELECT fork_id, owner_id, fork_title,
               format('Private graph fork %s', fork_number), commentary,
               prep_time_minutes, cook_time_minutes, servings,
               ingredients, instructions, equipment, '[]'::jsonb, 'private'
        FROM recipes WHERE id = template_id;

        INSERT INTO recipe_versions (recipe_id, version_number, snapshot, notes, created_at)
        SELECT fork_id, 1,
               jsonb_set(
                   jsonb_set(
                       jsonb_set(snapshot, '{title}', to_jsonb(fork_title)),
                       '{description}', to_jsonb(format('Private graph fork %s', fork_number))
                   ),
                   '{images}', '[]'::jsonb
               ),
               format('Private graph fixture fork %s', fork_number),
               now() - (31 - fork_number) * interval '1 second'
        FROM recipe_versions
        WHERE recipe_id = template_id AND version_number = 2;

        INSERT INTO fork_relationships (
            original_recipe_id, forked_recipe_id, forked_by,
            original_recipe_key, forked_recipe_key, forked_version_number,
            original_title, forked_title,
            original_owner_id, original_owner_username, original_visibility,
            forked_owner_id, forked_owner_username, forked_visibility,
            message, created_at
        ) VALUES (
            fixture_id, fork_id, owner_id,
            fixture_id, fork_id, 32,
            fixture_title, fork_title,
            owner_id, 'user-332d', 'private',
            owner_id, 'user-332d', 'private',
            'Private graph fixture branch',
            now() - (31 - fork_number) * interval '1 second'
        );
    END LOOP;
END
$fixture$;

DO $verify$
DECLARE
    fixture_id CONSTANT uuid := 'aef72aa2-2c1c-4576-b253-14e0d531e634';
BEGIN
    IF (SELECT count(*) FROM recipe_versions WHERE recipe_id = fixture_id) <> 32
       OR (SELECT count(*) FROM fork_relationships WHERE original_recipe_id = fixture_id) <> 30
       OR (SELECT count(*) FROM recipes WHERE id IN (
           SELECT forked_recipe_id FROM fork_relationships WHERE original_recipe_id = fixture_id
       )) <> 30
       OR (SELECT count(*) FROM recipe_versions WHERE recipe_id IN (
           SELECT forked_recipe_id FROM fork_relationships WHERE original_recipe_id = fixture_id
       ) AND version_number = 1) <> 30
       OR EXISTS (
           SELECT 1 FROM recipes
           WHERE id = fixture_id OR id IN (
               SELECT forked_recipe_id FROM fork_relationships WHERE original_recipe_id = fixture_id
           )
           GROUP BY user_id, visibility
           HAVING user_id <> 'da1ff4ae-c7b8-4ab5-96cb-ea982ddcad1f'::uuid
               OR visibility <> 'private'
       ) THEN
        RAISE EXCEPTION 'Dense graph fixture shape or privacy check failed';
    END IF;
END
$verify$;

COMMIT;
