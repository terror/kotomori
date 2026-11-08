ALTER TABLE sessions
ADD COLUMN snapshot TEXT CHECK (snapshot IS NULL OR JSON_VALID(snapshot));
