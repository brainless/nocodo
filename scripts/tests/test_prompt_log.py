import importlib.machinery
import json
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "prompt-log"
prompt_log = importlib.machinery.SourceFileLoader("prompt_log", str(SCRIPT)).load_module()


class PromptLogTests(unittest.TestCase):
    def test_base_record_has_stable_identity_and_hash(self):
        record = prompt_log.base_record(
            timestamp="2026-01-01T00:00:00+00:00",
            agent="codex",
            role="user",
            content="Fix the bug",
            project="/tmp/project",
            recorded_cwd="/tmp/project",
            session_id="session",
            message_id="message",
        )
        self.assertEqual(record["schema_version"], 1)
        self.assertEqual(record["record_id"], "codex:session:message")
        self.assertEqual(record["origin"], "human")
        self.assertEqual(len(record["content_sha256"]), 64)

    def test_write_jsonl_shape_is_serializable(self):
        record = prompt_log.base_record(
            timestamp="2026-01-01T00:00:00+00:00",
            agent="opencode",
            role="agent",
            content="Done",
            project="/tmp/project",
            recorded_cwd="/tmp/project/worktree",
            session_id="session",
            message_id="message",
        )
        encoded = json.dumps(record)
        self.assertIn('"session_id": "session"', encoded)
        self.assertEqual(record["origin"], "agent")

    def test_markdown_writer_requires_private_mode_from_caller(self):
        record = prompt_log.base_record(
            timestamp="2026-01-01T00:00:00+00:00",
            agent="claude-code",
            role="user",
            content="Review this",
            project="/tmp/project",
            recorded_cwd="/tmp/project",
            session_id="session",
            message_id="message",
        )
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "prompt.md"
            prompt_log.write_markdown([record], output)
            output.chmod(0o600)
            self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o600)
            self.assertIn("Review this", output.read_text())

    def test_worktree_belongs_to_main_project(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "repo"
            worktree = Path(directory) / "worktree"
            root.mkdir()
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            subprocess.run(
                ["git", "-C", str(root), "config", "user.email", "test@example.com"],
                check=True,
            )
            subprocess.run(
                ["git", "-C", str(root), "config", "user.name", "Test"],
                check=True,
            )
            (root / "README.md").write_text("fixture\n")
            subprocess.run(["git", "-C", str(root), "add", "README.md"], check=True)
            subprocess.run(["git", "-C", str(root), "commit", "-qm", "fixture"], check=True)
            subprocess.run(
                ["git", "-C", str(root), "worktree", "add", "-q", "-b", "fixture-worktree", str(worktree)],
                check=True,
            )
            self.assertTrue(prompt_log.belongs_to_project(str(worktree), str(root)))


if __name__ == "__main__":
    unittest.main()
