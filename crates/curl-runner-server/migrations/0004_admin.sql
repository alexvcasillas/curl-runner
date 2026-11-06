-- The admin panel (plan 2b), all of it at once. Additive only: every new
-- column on an existing table is nullable or has a default, so this applies
-- to a populated database. "By" columns on live rows are foreign keys that go
-- NULL when the admin's account is deleted; the logs (admin_events,
-- plan_history) keep a bare id so they outlive the account.

-- Admins other than the owners (OPERATOR_EMAILS and the first operator).
-- permissions: users, teams, billing, emails, settings, admins.
CREATE TABLE admins (
    user_id     uuid PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    permissions text[] NOT NULL,
    added_by    uuid REFERENCES users (id) ON DELETE SET NULL,
    added_at    timestamptz NOT NULL
);

-- One row per admin write, in the same transaction as the write. Deleted
-- after 2 years. action is a stable snake_case verb such as `user.suspend`.
CREATE TABLE admin_events (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_user_id uuid,
    -- The admin's email when it happened, so the log names deleted admins.
    admin_email   text NOT NULL,
    action        text NOT NULL,
    target_kind   text,
    target_id     text,
    target_label  text,
    detail        jsonb NOT NULL DEFAULT '{}',
    ip            text,
    at            timestamptz NOT NULL
);
CREATE INDEX admin_events_at ON admin_events (at DESC, id DESC);
CREATE INDEX admin_events_admin ON admin_events (admin_user_id, at DESC);
CREATE INDEX admin_events_target ON admin_events (target_kind, target_id, at DESC);

-- Settings that used to be env vars. At most one row; the server inserts it
-- from SIGNUP, SIGNUP_DOMAINS and DEFAULT_PLAN on its first start, keeping
-- those env values in env_seed so a later env change can be noticed.
CREATE TABLE server_settings (
    singleton         bool PRIMARY KEY DEFAULT true CHECK (singleton),
    signup_mode       text NOT NULL DEFAULT 'domains' CHECK (signup_mode IN ('closed', 'domains', 'open')),
    signup_domains    text[] NOT NULL DEFAULT '{}',
    default_plan      text NOT NULL DEFAULT 'team' CHECK (default_plan IN ('free', 'team')),
    free_member_limit int NOT NULL DEFAULT 3 CHECK (free_member_limit BETWEEN 1 AND 100),
    free_restricted   bool NOT NULL DEFAULT false,
    grace_days        int NOT NULL DEFAULT 14 CHECK (grace_days >= 0),
    banner_message    text,
    banner_tone       text NOT NULL DEFAULT 'info' CHECK (banner_tone IN ('info', 'warning')),
    banner_audience   text NOT NULL DEFAULT 'everyone' CHECK (banner_audience IN ('everyone', 'signed_in')),
    banner_link       text,
    banner_version    int NOT NULL DEFAULT 0,
    publishing_paused bool NOT NULL DEFAULT false,
    env_seed          jsonb,
    updated_at        timestamptz,
    updated_by        uuid REFERENCES users (id) ON DELETE SET NULL
);

-- A suspended user cannot sign in. product_emails: broadcasts only; sign-in
-- codes and invites ignore it.
ALTER TABLE users
    ADD COLUMN suspended_at timestamptz,
    ADD COLUMN suspended_by uuid REFERENCES users (id) ON DELETE SET NULL,
    ADD COLUMN suspended_reason text,
    ADD COLUMN product_emails bool NOT NULL DEFAULT true;

-- Team suspension, and the plan override that beats Stripe while
-- override_until is NULL or in the future.
ALTER TABLE teams
    ADD COLUMN suspended_at timestamptz,
    ADD COLUMN suspended_by uuid REFERENCES users (id) ON DELETE SET NULL,
    ADD COLUMN suspended_reason text,
    ADD COLUMN override_plan text CHECK (override_plan IN ('free', 'team')),
    ADD COLUMN override_until timestamptz,
    ADD COLUMN override_reason text,
    ADD COLUMN override_by uuid REFERENCES users (id) ON DELETE SET NULL,
    ADD COLUMN override_at timestamptz,
    ADD COLUMN override_reminded_at timestamptz;
CREATE INDEX teams_override_until ON teams (override_until) WHERE override_plan IS NOT NULL;

-- Every change of a team's effective plan. source: where it came from, such
-- as `stripe`, `override` or `default`.
CREATE TABLE plan_history (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id       uuid NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    at            timestamptz NOT NULL,
    source        text NOT NULL,
    plan          text NOT NULL CHECK (plan IN ('free', 'team')),
    admin_user_id uuid,
    reason        text,
    detail        jsonb NOT NULL DEFAULT '{}'
);
CREATE INDEX plan_history_team ON plan_history (team_id, at DESC);

-- A share link taken down by an admin answers 404 and the team cannot reset it.
ALTER TABLE collections
    ADD COLUMN link_disabled_at timestamptz,
    ADD COLUMN link_disabled_by uuid REFERENCES users (id) ON DELETE SET NULL,
    ADD COLUMN link_disabled_reason text;

CREATE TABLE broadcasts (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    subject      text NOT NULL DEFAULT '',
    body_md      text NOT NULL DEFAULT '',
    audience     text NOT NULL DEFAULT 'everyone'
                 CHECK (audience IN ('everyone', 'team_owners', 'paying_teams', 'free_teams', 'granted_teams')),
    status       text NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'scheduled', 'sending', 'sent')),
    scheduled_at timestamptz,
    sent_at      timestamptz,
    sent_count   int NOT NULL DEFAULT 0,
    created_by   uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at   timestamptz NOT NULL,
    updated_at   timestamptz NOT NULL
);
CREATE INDEX broadcasts_due ON broadcasts (scheduled_at) WHERE status IN ('scheduled', 'sending');

-- kind: `transactional` for everything sent so far. A broadcast queues one
-- row per recipient user; the unique index makes a resumed send skip the
-- ones already queued. created_at stays NULL on rows queued before this
-- migration; failed_at is set when the worker gives up.
ALTER TABLE outbox_emails
    ADD COLUMN kind text NOT NULL DEFAULT 'transactional',
    ADD COLUMN broadcast_id uuid REFERENCES broadcasts (id) ON DELETE SET NULL,
    ADD COLUMN user_id uuid REFERENCES users (id) ON DELETE SET NULL,
    ADD COLUMN created_at timestamptz,
    ADD COLUMN failed_at timestamptz;
ALTER TABLE outbox_emails ALTER COLUMN created_at SET DEFAULT now();
CREATE UNIQUE INDEX outbox_emails_broadcast_recipient
    ON outbox_emails (broadcast_id, user_id) WHERE broadcast_id IS NOT NULL;

-- One row a day for the 90-day revenue chart.
CREATE TABLE mrr_snapshots (
    day       date PRIMARY KEY,
    mrr_cents bigint NOT NULL,
    teams     int NOT NULL,
    seats     int NOT NULL
);

-- Filled when an event is applied, for the Stripe page's event list.
ALTER TABLE stripe_events
    ADD COLUMN team_id uuid REFERENCES teams (id) ON DELETE SET NULL,
    ADD COLUMN summary text,
    ADD COLUMN mrr_delta_cents bigint,
    -- The team's MRR before the event: a rise from 0 is new, any other expansion.
    ADD COLUMN mrr_before_cents bigint;
-- The overview's day buckets filter these by a created_at range.
CREATE INDEX users_created_at ON users (created_at);
CREATE INDEX teams_created_at ON teams (created_at);
CREATE INDEX versions_created_at ON versions (created_at);
CREATE INDEX stripe_events_received_at ON stripe_events (received_at DESC);
