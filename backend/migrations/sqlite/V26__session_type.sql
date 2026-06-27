ALTER TABLE user_chat_session
  ADD COLUMN session_type TEXT NOT NULL DEFAULT 'intake';
