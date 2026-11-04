-- oauth_states.client: echoed to the dashboard, which hands off for devices.
-- oauth_states.binding_hash: ties the flow to the browser that started it.
-- auth_handoffs.label: the device label, because the device session itself is
-- created at redeem time (its clear token cannot be stored).
ALTER TABLE oauth_states
    ADD COLUMN client text NOT NULL DEFAULT 'web' CHECK (client IN ('device', 'web')),
    ADD COLUMN binding_hash text NOT NULL DEFAULT '';
ALTER TABLE auth_handoffs
    ADD COLUMN label text;
