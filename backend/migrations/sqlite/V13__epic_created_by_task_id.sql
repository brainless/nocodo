ALTER TABLE epic ADD COLUMN created_by_task_id INTEGER REFERENCES task(id);
