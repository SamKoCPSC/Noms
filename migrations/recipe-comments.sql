
CREATE TABLE IF NOT EXISTS recipe_comments (
    request_id UUID NOT NULL,
    UNIQUE (author_id, request_id),
    id BIGSERIAL PRIMARY KEY,
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    parent_id BIGINT,
    author_id UUID REFERENCES users(id) ON DELETE SET NULL,
    body TEXT CHECK (body IS NULL OR char_length(body) BETWEEN 1 AND 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (recipe_id, id),
    FOREIGN KEY (recipe_id, parent_id) REFERENCES recipe_comments(recipe_id, id),
    FOREIGN KEY (recipe_id, version_number) REFERENCES recipe_versions(recipe_id, version_number) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_recipe_comments_thread
    ON recipe_comments(recipe_id, parent_id, id);

CREATE TABLE IF NOT EXISTS recipe_comment_likes (
    comment_id BIGINT NOT NULL REFERENCES recipe_comments(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (comment_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_recipe_comment_likes_user ON recipe_comment_likes(user_id);
