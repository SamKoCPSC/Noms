-- NOMS-020 Settings acceptance: an isolated, expendable local identity only.
-- NEVER production. Verify com.noms.dev-environment=true on the exact DB container.
-- User approved fixture preparation on 2026-10-01, not destructive UI actions.
-- Upload the generated full/thumbnail/original avatar objects to the exact local
-- MinIO bucket first. Supply: psql -v fixture_media_base=http://127.0.0.1:PORT/noms-media -f ...
-- For a new approved fixture only, also pass -v fixture_allow_create=true.
-- Insert-only and transactional. Replay validates the untouched fixture; after
-- partial unlink/removal it refuses restoration rather than resetting a test.
-- Creation requires explicit fixture_allow_create=true; ordinary replay never
-- recreates an account after complete deletion and media cleanup.
\set ON_ERROR_STOP on
\if :{?fixture_allow_create}
\else
\set fixture_allow_create false
\endif
BEGIN;
SELECT set_config('noms.fixture_media_base', :'fixture_media_base', true);
SELECT set_config('noms.fixture_allow_create', :'fixture_allow_create', true);
DO $fixture$
DECLARE
    fixture_namespace CONSTANT text := 'NOMS020 CP8 Settings disposable 2026-10-01';
    fixture_owner CONSTANT uuid := md5(fixture_namespace || ' owner')::uuid;
    fixture_asset CONSTANT uuid := md5(fixture_namespace || ' avatar')::uuid;
    google_id CONSTANT uuid := md5(fixture_namespace || ' google')::uuid;
    github_id CONSTANT uuid := md5(fixture_namespace || ' github')::uuid;
    google_subject CONSTANT text := 'cp8settingsdisposable';
    -- The repository's local mock-github-api.conf emits this fixed identity.
    github_subject CONSTANT text := '424242';
    media_base text := current_setting('noms.fixture_media_base');
    avatar_prefix text := 'avatars/' || fixture_owner || '/' || fixture_asset;
    existing_username text;
    existing_display_name text;
    existing_email text;
    fixture_exists boolean;
BEGIN
    IF media_base !~ '^http://127[.]0[.]0[.]1:[0-9]+/noms-media$' THEN
        RAISE EXCEPTION 'Only the verified loopback MinIO public base is allowed';
    END IF;
    SELECT username, display_name, email
    INTO existing_username, existing_display_name, existing_email
    FROM users WHERE id = fixture_owner;
    fixture_exists := FOUND;
    IF fixture_exists THEN
        -- Compare indexed text outside SQL predicates (ParadeDB field aliases).
        IF existing_username IS DISTINCT FROM 'cp8settings'
            OR existing_display_name IS DISTINCT FROM 'CP8 Settings – Disposable'
            OR existing_email IS DISTINCT FROM 'cp8settings@example.invalid' THEN
            RAISE EXCEPTION 'Settings owner ID collision or changed fixture';
        END IF;
    ELSE
        IF current_setting('noms.fixture_allow_create') <> 'true' THEN
            RAISE EXCEPTION 'Settings fixture absent; replay must not recreate a deleted account';
        END IF;
        IF EXISTS (SELECT 1 FROM media_assets WHERE id = fixture_asset)
            OR EXISTS (SELECT 1 FROM oauth_accounts WHERE id IN (google_id, github_id)
                OR (provider = 'google' AND provider_user_id = google_subject)
                OR (provider = 'github' AND provider_user_id = github_subject)) THEN
            RAISE EXCEPTION 'Settings media/provider identity collision';
        END IF;
        INSERT INTO users (id, username, display_name, email, bio)
        VALUES (fixture_owner, 'cp8settings', 'CP8 Settings – Disposable',
            'cp8settings@example.invalid',
            'Synthetic local Settings acceptance account. No real person or content.');
        INSERT INTO oauth_accounts
            (id, user_id, provider, provider_user_id, email, email_verified, profile_data)
        VALUES
            (google_id, fixture_owner, 'google', google_subject,
                'cp8settings@example.invalid', true,
                jsonb_build_object('synthetic_fixture', fixture_namespace)),
            (github_id, fixture_owner, 'github', github_subject,
                'cp8settings@example.invalid', true,
                jsonb_build_object('synthetic_fixture', fixture_namespace));
        -- These are mock identities; no credential or real provider token is seeded.
        INSERT INTO media_assets
            (id, uploader_id, purpose, full_url, thumbnail_url, original_url,
             full_key, thumbnail_key, original_key, lifecycle, retained_at)
        VALUES (fixture_asset, fixture_owner, 'avatar',
            media_base || '/' || avatar_prefix || '.webp',
            media_base || '/' || avatar_prefix || '-thumb.webp',
            media_base || '/' || avatar_prefix || '-original',
            avatar_prefix || '.webp', avatar_prefix || '-thumb.webp',
            avatar_prefix || '-original', 'retained', NOW());
        INSERT INTO avatar_media_refs (user_id, asset_id)
        VALUES (fixture_owner, fixture_asset);
    END IF;

    IF (SELECT count(*) FROM oauth_accounts WHERE user_id = fixture_owner) <> 2
        OR NOT EXISTS (SELECT 1 FROM oauth_accounts WHERE id = google_id
            AND user_id = fixture_owner AND provider = 'google' AND provider_user_id = google_subject)
        OR NOT EXISTS (SELECT 1 FROM oauth_accounts WHERE id = github_id
            AND user_id = fixture_owner AND provider = 'github' AND provider_user_id = github_subject)
        OR NOT EXISTS (SELECT 1 FROM avatar_media_refs WHERE user_id = fixture_owner AND asset_id = fixture_asset)
        OR NOT EXISTS (SELECT 1 FROM media_assets WHERE id = fixture_asset
            AND uploader_id = fixture_owner AND purpose = 'avatar' AND lifecycle = 'retained'
            AND full_url = media_base || '/' || avatar_prefix || '.webp'
            AND thumbnail_url = media_base || '/' || avatar_prefix || '-thumb.webp'
            AND original_url = media_base || '/' || avatar_prefix || '-original'
            AND full_key = avatar_prefix || '.webp'
            AND thumbnail_key = avatar_prefix || '-thumb.webp'
            AND original_key = avatar_prefix || '-original'
            AND capability_digest IS NULL AND expires_at IS NULL) THEN
        RAISE EXCEPTION 'Settings fixture was consumed or changed; do not silently reset it';
    END IF;
    IF EXISTS (SELECT 1 FROM recipes WHERE user_id = fixture_owner)
        OR EXISTS (SELECT 1 FROM collections WHERE owner_id = fixture_owner)
        OR EXISTS (SELECT 1 FROM recipe_drafts WHERE user_id = fixture_owner) THEN
        RAISE EXCEPTION 'Settings fixture has content; do not use it for destructive acceptance';
    END IF;
END
$fixture$;
COMMIT;
