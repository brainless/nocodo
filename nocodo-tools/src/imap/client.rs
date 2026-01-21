use anyhow::{Context, Result};
use imap::Session;
use rustls_connector::RustlsConnector;
use std::net::TcpStream;
use std::time::Duration;

pub struct ImapClient {
    session: Session<rustls_connector::TlsStream<TcpStream>>,
}

impl ImapClient {
    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        _timeout: Duration,
    ) -> Result<Self> {
        // Establish TCP connection
        let tcp_stream = TcpStream::connect((host, port))
            .context("Failed to establish TCP connection to IMAP server")?;

        // Wrap with TLS using rustls
        let tls_connector =
            RustlsConnector::new_with_native_certs().context("Failed to create TLS connector")?;

        let tls_stream = tls_connector
            .connect(host, tcp_stream)
            .context("Failed to establish TLS connection")?;

        // Create IMAP client and login
        let client = imap::Client::new(tls_stream);

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

    pub fn session(&mut self) -> &mut Session<rustls_connector::TlsStream<TcpStream>> {
        &mut self.session
    }

    pub fn logout(mut self) -> Result<()> {
        self.session.logout().context("Failed to logout")?;
        Ok(())
    }
}
