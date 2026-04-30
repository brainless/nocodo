-- Merge agent_tool_call into agent_chat_message.
-- tool_name: set on role='assistant' (invocation) and role='tool' (result) rows.
-- tool_call_id: now also set on role='assistant' rows to correlate with their result.
-- agent_type: reserved for future multi-agent sessions (NULL = human user).
ALTER TABLE agent_chat_message ADD COLUMN agent_type TEXT;
ALTER TABLE agent_chat_message ADD COLUMN tool_name  TEXT;

DROP TABLE agent_tool_call;
