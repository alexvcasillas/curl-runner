-- Which client asked for the sign-in code, so the session made at verify time
-- gets the right kind and label whether the code or the emailed link is used.
ALTER TABLE email_codes
    ADD COLUMN client text NOT NULL DEFAULT 'web' CHECK (client IN ('device', 'web')),
    ADD COLUMN label text;
