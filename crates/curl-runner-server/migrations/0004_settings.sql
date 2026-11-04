-- Server-wide key/value settings. `first_operator` is the email of the first
-- account on a self-hosted server with SIGNUP=domains and no domains listed.
CREATE TABLE settings (
    key   text PRIMARY KEY,
    value text NOT NULL
);
