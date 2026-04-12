-- ============================================================================
-- V5__add_column_width.sql
-- Add width column to sheet_tab_column for user-defined column widths
-- ============================================================================

ALTER TABLE sheet_tab_column ADD COLUMN width INTEGER DEFAULT 120;
