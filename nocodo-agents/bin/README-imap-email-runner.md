# IMAP Email Runner - Test Binary

A standalone CLI binary for testing the IMAP Email Agent manually.

## Features

- 🔐 **Secure Password Input**: Password prompted at runtime (not echoed to terminal)
- 📧 **Single Query Mode**: Execute one email query and exit
- 🔄 **Interactive Mode**: Multiple queries in a persistent session
- 💾 **Session Persistence**: Conversation history maintained in interactive mode
- 🎯 **All IMAP Providers**: Works with Gmail, Outlook, Yahoo, iCloud, etc.

## Quick Start

### Build the Binary

```bash
# Debug build (faster compilation)
cargo build --bin imap-email-runner

# Release build (optimized performance)
cargo build --bin imap-email-runner --release
```

### Single Query Mode

Execute one query and exit:

```bash
cargo run --bin imap-email-runner -- \
  --config /path/to/config.toml \
  --host imap.gmail.com \
  --port 993 \
  --username your-email@gmail.com \
  --prompt "Show me unread emails from last week"
```

The binary will prompt for your password:
```
Enter IMAP password: [hidden input]
```

### Interactive Mode

Start an interactive session with multiple queries:

```bash
cargo run --bin imap-email-runner -- \
  --config /path/to/config.toml \
  --host imap.gmail.com \
  --username your-email@gmail.com \
  --interactive \
  --prompt "List my mailboxes"
```

In interactive mode:
- The initial `--prompt` is executed first
- You can then enter additional queries at the prompt
- Type `quit` or `exit` to end the session
- Session history is preserved across queries

Example interactive session:
```
📧 Your query> Show me unread emails from support@company.com
⏳ Processing...

--- 📬 Agent Result ---
Found 3 unread emails...

📧 Your query> Summarize the most recent one
⏳ Processing...

--- 📬 Agent Result ---
The most recent email from support@company.com...

📧 Your query> quit
👋 Ending session. Goodbye!
```

## Command-Line Arguments

| Argument | Required | Default | Description |
|----------|----------|---------|-------------|
| `--prompt`, `-p` | Yes | - | User prompt/query for the agent |
| `--config`, `-c` | Yes | - | Path to config file with API keys |
| `--host` | Yes | - | IMAP server hostname |
| `--port` | No | 993 | IMAP server port (TLS) |
| `--username` | Yes | - | Email address for IMAP login |
| `--interactive`, `-i` | No | false | Enable interactive mode |

**Note:** Password is NEVER passed as a CLI argument. It's prompted securely at runtime.

## Common IMAP Providers

### Gmail
```bash
--host imap.gmail.com --port 993
```
**Important:** Gmail requires an [app-specific password](https://support.google.com/accounts/answer/185833), not your regular account password.

### Microsoft Outlook / Office 365
```bash
--host outlook.office365.com --port 993
```

### Yahoo Mail
```bash
--host imap.mail.yahoo.com --port 993
```

### iCloud Mail
```bash
--host imap.mail.me.com --port 993
```
**Note:** Requires an [app-specific password](https://support.apple.com/en-us/HT204397).

## Example Queries

### Email Triage
```
Show me unread emails from important-client@example.com
```

### Information Extraction
```
Find the order confirmation from Amazon last week
```

### Mailbox Exploration
```
What folders do I have and how many emails are in each?
```

### Email Summarization
```
Summarize emails from the team@company.com this month
```

### Search by Date
```
Show me emails from boss@company.com since January 1st
```

### Search by Subject
```
Find emails with "invoice" in the subject from last week
```

## Configuration File

The `--config` parameter points to a TOML file with your LLM API credentials:

```toml
[zai]
api_key = "your-api-key-here"
coding_plan = "your-coding-plan"
```

See other runner binaries for examples.

## Troubleshooting

### "Authentication failed"
- Verify your username and password are correct
- For Gmail/iCloud: Use an app-specific password, not your account password
- Check if 2FA is enabled and generate app password accordingly

### "Connection timeout"
- Verify the IMAP server hostname and port
- Check your network connectivity
- Some corporate networks block IMAP ports

### "Mailbox not found"
- Use the "List my mailboxes" query first to see available folders
- Mailbox names are case-sensitive (e.g., "INBOX" not "inbox")

### "Too many results"
- Add more specific search criteria
- Use date ranges to narrow results
- Query specific mailboxes instead of all folders

## Security Notes

- ✅ Password never appears in CLI history (prompted at runtime)
- ✅ Password not echoed to terminal during input
- ✅ Session data stored in memory only (`:memory:` database)
- ✅ No credentials logged or persisted to disk
- ⚠️  Use app-specific passwords for Gmail/iCloud (never account passwords)

## Development

The binary is located at `nocodo-agents/bin/imap_email_runner.rs` and follows the standard runner pattern used by other agent test binaries in this crate.

To add logging:
```bash
RUST_LOG=debug cargo run --bin imap-email-runner -- [args...]
```

Log levels: `trace`, `debug`, `info`, `warn`, `error`
