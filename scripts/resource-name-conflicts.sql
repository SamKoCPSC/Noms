-- Read-only audit. Every row must be resolved before creating the unique indexes.
-- Each resource type has its own namespace; collection parents do not affect it.
WITH names AS (
    SELECT 'collection' AS resource_type, owner_id, id, name FROM collections
    UNION ALL
    SELECT 'pantry', owner_id, id, name FROM kitchen_profiles
    UNION ALL
    SELECT 'grocery list', owner_id, id, name FROM grocery_lists
), normalized AS (
    SELECT *, lower(btrim(regexp_replace(name, '[[:space:]]+', ' ', 'g'))) AS normalized_name
    FROM names
)
SELECT resource_type, owner_id, normalized_name,
       count(*) AS conflicts,
       jsonb_agg(jsonb_build_object('id', id, 'name', name) ORDER BY id) AS resources
FROM normalized
GROUP BY resource_type, owner_id, normalized_name
HAVING count(*) > 1
ORDER BY resource_type, owner_id, normalized_name;
