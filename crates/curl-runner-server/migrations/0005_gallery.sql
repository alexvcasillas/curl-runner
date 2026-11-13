-- The public gallery (www.curl-runner.com/collections). A collection is
-- listed while `listed_at` is set: only when its access is public and it
-- has a description. Listed is newest first.
ALTER TABLE collections ADD COLUMN listed_at timestamptz;
CREATE INDEX collections_listed ON collections (listed_at DESC) WHERE listed_at IS NOT NULL;

-- Requests in a version, counted from its YAML when published (the gallery's
-- "18 requests"). NULL for versions published before this column.
ALTER TABLE versions ADD COLUMN request_count int;
