-- comment_id is a historical identifier: deleting a leaf must not erase its reports.
CREATE TABLE IF NOT EXISTS comment_review_cases (
    id BIGSERIAL PRIMARY KEY,
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    comment_id BIGINT NOT NULL,
    queued_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    CHECK (resolved_at IS NULL OR queued_at IS NOT NULL)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_comment_review_active
    ON comment_review_cases(comment_id) WHERE resolved_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_comment_review_queue
    ON comment_review_cases(queued_at, id) WHERE queued_at IS NOT NULL AND resolved_at IS NULL;

CREATE TABLE IF NOT EXISTS comment_reports (
    id BIGSERIAL PRIMARY KEY,
    case_id BIGINT NOT NULL REFERENCES comment_review_cases(id) ON DELETE CASCADE,
    reporter_id UUID REFERENCES users(id) ON DELETE SET NULL,
    reason TEXT NOT NULL CHECK (char_length(trim(reason)) BETWEEN 1 AND 500),
    reported_body TEXT NOT NULL,
    recipe_version INTEGER NOT NULL,
    reported_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (case_id, reporter_id)
);
