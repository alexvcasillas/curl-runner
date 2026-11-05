-- When a sign-in provider was linked. Existing rows take the account's creation time.
ALTER TABLE identities ADD COLUMN created_at timestamptz;
UPDATE identities i SET created_at = u.created_at FROM users u WHERE u.id = i.user_id;
ALTER TABLE identities ALTER COLUMN created_at SET NOT NULL;
ALTER TABLE identities ALTER COLUMN created_at SET DEFAULT now();
