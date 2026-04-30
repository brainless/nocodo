-- Groups rows that belong to the same LLM response turn.
-- Equals the id of the first row inserted in the turn; single-row turns have turn_id = id.
-- Existing rows left NULL; new rows always populated by the storage layer.
ALTER TABLE agent_chat_message ADD COLUMN turn_id INTEGER;
