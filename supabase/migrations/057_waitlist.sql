-- Waitlist table for website operator signups
CREATE TABLE IF NOT EXISTS waitlist (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  email text NOT NULL UNIQUE,
  source text DEFAULT 'website',
  created_at timestamptz DEFAULT now()
);

ALTER TABLE waitlist ENABLE ROW LEVEL SECURITY;

-- Anon can insert only (no read/update/delete)
CREATE POLICY "anon_insert" ON waitlist FOR INSERT TO anon WITH CHECK (true);
