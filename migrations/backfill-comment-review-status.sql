-- Existing cases only recorded timestamps. A legacy resolution has no known verdict.
UPDATE comment_review_cases
SET status = CASE WHEN resolved_at IS NULL THEN 'queued'::comment_review_status END
WHERE status = 'collecting' AND queued_at IS NOT NULL;
