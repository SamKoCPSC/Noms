-- NOMS-020 checkpoint-8 private pagination fixture, never a production seed.
-- Check com.noms.dev-environment=true on the isolated Postgres container first.
-- Reuses the existing mock-account dense graph recipes; creates no recipes and
-- changes no existing collection memberships or visibility settings.
\set ON_ERROR_STOP on
BEGIN;
DO $fixture$
DECLARE
    fixture_id CONSTANT uuid := md5('NOMS020 CP8 pagination collection 2026-09-29')::uuid;
    fixture_owner CONSTANT uuid := 'da1ff4ae-c7b8-4ab5-96cb-ea982ddcad1f';
    graph_id CONSTANT uuid := 'aef72aa2-2c1c-4576-b253-14e0d531e634';
    fixture_name CONSTANT text := 'NOMS020 CP8 pagination collection 2026-09-29';
    existing_name text;
BEGIN
    IF (SELECT count(*) FROM recipes WHERE user_id = fixture_owner
        AND visibility = 'private' AND (id = graph_id OR id IN (
            SELECT forked_recipe_id FROM fork_relationships WHERE original_recipe_id = graph_id
        ))) <> 31 THEN
        RAISE EXCEPTION 'Expected 31 private mock-account graph recipes are missing';
    END IF;
    IF EXISTS (SELECT 1 FROM collections WHERE id = fixture_id) THEN
        -- Compare the selected value outside the indexed query: this database
        -- indexes collection names under two distinct BM25 field aliases.
        SELECT name INTO existing_name FROM collections WHERE id = fixture_id;
        IF NOT EXISTS (SELECT 1 FROM collections WHERE id = fixture_id
            AND owner_id = fixture_owner AND visibility = 'private') OR existing_name <> fixture_name
        THEN RAISE EXCEPTION 'Pagination fixture ID belongs to a different collection'; END IF;
        RAISE NOTICE 'Pagination collection already exists; no rows inserted';
    ELSE
        INSERT INTO collections (id, owner_id, name, description, visibility)
        VALUES (fixture_id, fixture_owner, fixture_name, 'Private 31-recipe pagination acceptance fixture', 'private');
        INSERT INTO collection_recipes (collection_id, recipe_id, sort_order)
        SELECT fixture_id, id, (row_number() OVER (ORDER BY title, id) - 1)::integer
        FROM recipes WHERE user_id = fixture_owner AND visibility = 'private'
            AND (id = graph_id OR id IN (
                SELECT forked_recipe_id FROM fork_relationships WHERE original_recipe_id = graph_id
            ));
    END IF;
    IF (SELECT count(*) FROM collection_recipes WHERE collection_id = fixture_id) <> 31
        OR EXISTS (
            SELECT 1 FROM collection_recipes AS membership
            JOIN recipes AS recipe ON recipe.id = membership.recipe_id
            WHERE membership.collection_id = fixture_id
                AND (recipe.user_id <> fixture_owner OR recipe.visibility <> 'private'
                    OR NOT (recipe.id = graph_id OR recipe.id IN (
                        SELECT forked_recipe_id FROM fork_relationships WHERE original_recipe_id = graph_id
                    )))
        ) THEN
        RAISE EXCEPTION 'Pagination collection must contain exactly the 31 private graph recipes';
    END IF;
END
$fixture$;
COMMIT;
