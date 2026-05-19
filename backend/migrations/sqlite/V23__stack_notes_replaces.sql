ALTER TABLE stack_note ADD COLUMN replaces_id INTEGER REFERENCES stack_note(id);
