-- curl-runner Cloud schema. Secrets (codes, tokens, links) are stored as
-- hex SHA-256(SECRET_KEY || value), never in clear. Timestamps come from the
-- app's clock, so they have no defaults.

CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE users (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email      citext NOT NULL UNIQUE,
    name       text,
    created_at timestamptz NOT NULL
);

CREATE TABLE identities (
    id       uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id  uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider text NOT NULL,
    subject  text NOT NULL,
    email    citext,
    UNIQUE (provider, subject)
);
CREATE INDEX identities_user_id ON identities (user_id);

CREATE TABLE email_codes (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email      citext NOT NULL,
    code_hash  text NOT NULL,
    -- NULL for device codes: their email has no link, only the code.
    link_hash  text UNIQUE,
    attempts   integer NOT NULL DEFAULT 0,
    expires_at timestamptz NOT NULL,
    used_at    timestamptz,
    -- Which client asked for the code, so the session made at verify time
    -- gets the right kind and label whether the code or the emailed link is used.
    client     text NOT NULL CHECK (client IN ('device', 'web')),
    label      text
);
CREATE INDEX email_codes_email ON email_codes (email);

CREATE TABLE sessions (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind         text NOT NULL CHECK (kind IN ('device', 'web')),
    token_hash   text NOT NULL UNIQUE,
    label        text,
    created_at   timestamptz NOT NULL,
    last_used_at timestamptz NOT NULL,
    expires_at   timestamptz NOT NULL,
    revoked_at   timestamptz
);
CREATE INDEX sessions_user_id ON sessions (user_id);

-- label: the device label, because the device session itself is created at
-- redeem time (its clear token cannot be stored).
CREATE TABLE auth_handoffs (
    code_hash  text PRIMARY KEY,
    session_id uuid NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    expires_at timestamptz NOT NULL,
    label      text
);

-- client: echoed to the dashboard, which hands off for devices.
-- binding_hash: ties the flow to the browser that started it.
CREATE TABLE oauth_states (
    state_hash   text PRIMARY KEY,
    provider     text NOT NULL,
    verifier     text NOT NULL,
    return_to    text,
    expires_at   timestamptz NOT NULL,
    client       text NOT NULL CHECK (client IN ('device', 'web')),
    binding_hash text NOT NULL
);

CREATE TABLE teams (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    slug       citext NOT NULL UNIQUE,
    name       text NOT NULL,
    plan       text NOT NULL DEFAULT 'free' CHECK (plan IN ('free', 'team')),
    created_at timestamptz NOT NULL
);

CREATE TABLE memberships (
    team_id    uuid NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    user_id    uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role       text NOT NULL CHECK (role IN ('owner', 'member')),
    created_at timestamptz NOT NULL,
    PRIMARY KEY (team_id, user_id)
);
CREATE INDEX memberships_user_id ON memberships (user_id);

CREATE TABLE member_invites (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id     uuid NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    email       citext NOT NULL,
    role        text NOT NULL CHECK (role IN ('owner', 'member')),
    token_hash  text NOT NULL UNIQUE,
    invited_by  uuid REFERENCES users (id) ON DELETE SET NULL,
    expires_at  timestamptz NOT NULL,
    accepted_at timestamptz
);
CREATE INDEX member_invites_team_id ON member_invites (team_id);

CREATE TABLE tokens (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id      uuid NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    name         text NOT NULL,
    token_hash   text NOT NULL UNIQUE,
    created_by   uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at   timestamptz NOT NULL,
    expires_at   timestamptz,
    last_used_at timestamptz,
    revoked_at   timestamptz
);
CREATE INDEX tokens_team_id ON tokens (team_id);

-- link_token_enc: the share link token, XChaCha20-Poly1305-encrypted with a
-- key derived from SECRET_KEY, so the dashboard can show the link again.
-- link_hash is what lookups use. This is the only secret kept reversibly.
CREATE TABLE collections (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id         uuid NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    slug            citext NOT NULL,
    name            text NOT NULL,
    description     text,
    source          text NOT NULL DEFAULT 'git' CHECK (source IN ('git', 'cloud')),
    access          text NOT NULL DEFAULT 'off' CHECK (access IN ('off', 'public', 'restricted')),
    link_hash       text UNIQUE,
    link_token_enc  text,
    link_expires_at timestamptz,
    current_version integer,
    UNIQUE (team_id, slug)
);

CREATE TABLE versions (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_id uuid NOT NULL REFERENCES collections (id) ON DELETE CASCADE,
    number        integer NOT NULL,
    hash          text NOT NULL,
    inputs        jsonb NOT NULL DEFAULT '[]',
    published_by  text NOT NULL,
    git           jsonb,
    created_at    timestamptz NOT NULL,
    UNIQUE (collection_id, number)
);
CREATE INDEX versions_collection_hash ON versions (collection_id, hash);

CREATE TABLE blobs (
    hash text PRIMARY KEY,
    text text NOT NULL
);

CREATE TABLE version_files (
    version_id uuid NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
    path       text NOT NULL,
    hash       text NOT NULL REFERENCES blobs (hash),
    PRIMARY KEY (version_id, path)
);

CREATE TABLE access_entries (
    collection_id uuid NOT NULL REFERENCES collections (id) ON DELETE CASCADE,
    entry         citext NOT NULL,
    kind          text NOT NULL CHECK (kind IN ('email', 'domain')),
    invited_at    timestamptz NOT NULL,
    PRIMARY KEY (collection_id, entry)
);

CREATE TABLE share_opens (
    collection_id uuid NOT NULL REFERENCES collections (id) ON DELETE CASCADE,
    user_id       uuid REFERENCES users (id) ON DELETE SET NULL,
    entry         citext,
    opened_at     timestamptz NOT NULL
);
CREATE INDEX share_opens_collection_id ON share_opens (collection_id);

CREATE TABLE outbox_emails (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    to_addr         text NOT NULL,
    subject         text NOT NULL,
    text_body       text NOT NULL,
    html_body       text,
    attempts        integer NOT NULL DEFAULT 0,
    next_attempt_at timestamptz NOT NULL,
    sent_at         timestamptz,
    last_error      text
);
CREATE INDEX outbox_emails_pending ON outbox_emails (next_attempt_at) WHERE sent_at IS NULL;

-- Server-wide key/value settings. `first_operator` is the email of the first
-- account on a self-hosted server with SIGNUP=domains and no domains listed.
CREATE TABLE settings (
    key   text PRIMARY KEY,
    value text NOT NULL
);
