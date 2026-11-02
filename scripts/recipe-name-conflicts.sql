-- Read-only deployment preflight. Resolve every returned group before applying
-- the unique-name index. Rename through the application to retain history.
SELECT user_id,
       lower(btrim(regexp_replace(title, '[[:space:]]+', ' ', 'g'))) AS normalized_name,
       count(*) AS recipe_count,
       jsonb_agg(jsonb_build_object('id', id, 'title', title) ORDER BY created_at, id) AS recipes
FROM recipes
GROUP BY user_id, lower(btrim(regexp_replace(title, '[[:space:]]+', ' ', 'g')))
HAVING count(*) > 1
ORDER BY user_id, normalized_name;
