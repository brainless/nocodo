ALTER TABLE user_chat_session
  ADD COLUMN handoff_session_id INTEGER NULL REFERENCES user_chat_session(id);
