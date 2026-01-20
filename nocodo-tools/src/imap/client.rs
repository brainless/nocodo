use anyhow::{Context, Result};
use imap::{ClientBuilder, Connection};
use std::time::Duration;

pub struct ImapClient {
    session: imap::Session<Connection>,
}

impl ImapClient {
    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        _timeout: Duration,
    ) -> Result<Self> {
        let client = ClientBuilder::new(host, port)
            .connect()
            .context("Failed to connect to IMAP server")?;

        let session = client
            .login(username, password)
            .map_err(|e| anyhow::anyhow!("IMAP login failed: {}", e.0))?;

        Ok(Self { session })
    }

    pub fn select_mailbox(&mut self, mailbox: &str) -> Result<()> {
        self.session
            .select(mailbox)
            .context(format!("Failed to select mailbox: {}", mailbox))?;
        Ok(())
    }

    pub fn examine_mailbox(&mut self, mailbox: &str) -> Result<()> {
        self.session
            .examine(mailbox)
            .context(format!("Failed to examine mailbox: {}", mailbox))?;
        Ok(())
    }

    pub fn session(&mut self) -> &mut imap::Session<Connection> {
        &mut self.session
    }

    pub fn logout(mut self) -> Result<()> {
        self.session.logout().context("Failed to logout")?;
        Ok(())
    }
}
