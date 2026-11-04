-- oauth_states.client: the session kind to create at callback time.
-- auth_handoffs.label: the device label, because the device session itself is
-- created at redeem time (its clear token cannot be stored).
ALTER TABLE oauth_states
    ADD COLUMN client text NOT NULL DEFAULT 'web' CHECK (client IN ('device', 'web'));
ALTER TABLE auth_handoffs
    ADD COLUMN label text;
