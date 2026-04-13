-- Add sql_name column to sheet_tab_column for storing actual SQL column names
-- This separates display names (user-friendly) from SQL column names
ALTER TABLE sheet_tab_column ADD COLUMN sql_name TEXT;

-- Populate sql_name with snake_case conversion of existing name for backwards compatibility
-- This will be overridden by proper values in seed_internal_schema.rs
UPDATE sheet_tab_column SET sql_name = LOWER(REPLACE(REPLACE(name, ' ', '_'), '-', '_'));
