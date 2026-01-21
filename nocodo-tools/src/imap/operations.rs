use super::client::ImapClient;
use super::types::*;
use anyhow::Result;
use imap_proto::types::Address;
use mail_parser::{MessageParser, MimeHeaders};

pub fn list_mailboxes(client: &mut ImapClient, pattern: Option<&str>) -> Result<Vec<MailboxInfo>> {
    let pattern = pattern.unwrap_or("*");
    let mailboxes = client
        .session()
        .list(Some(""), Some(pattern))
        .map_err(|e| anyhow::anyhow!("Failed to list mailboxes: {}", e))?;

    let result = mailboxes
        .iter()
        .map(|mb| MailboxInfo {
            name: mb.name().to_string(),
            delimiter: mb.delimiter().map(|c| c.to_string()),
            flags: mb.attributes().iter().map(|a| format!("{:?}", a)).collect(),
        })
        .collect();

    Ok(result)
}

pub fn mailbox_status(client: &mut ImapClient, mailbox: &str) -> Result<MailboxStatusInfo> {
    let status = client
        .session()
        .status(mailbox, "(MESSAGES RECENT UNSEEN UIDNEXT UIDVALIDITY)")
        .map_err(|e| anyhow::anyhow!("Failed to get mailbox status: {}", e))?;

    Ok(MailboxStatusInfo {
        mailbox: mailbox.to_string(),
        messages: Some(status.exists),
        recent: Some(status.recent),
        unseen: status.unseen,
        uid_next: status.uid_next,
        uid_validity: status.uid_validity,
    })
}

pub fn search_emails(
    client: &mut ImapClient,
    mailbox: &str,
    criteria: &crate::types::SearchCriteria,
    limit: Option<usize>,
) -> Result<Vec<u32>> {
    client.examine_mailbox(mailbox)?;

    let query = build_search_query(criteria)?;

    let uids = client
        .session()
        .uid_search(&query)
        .map_err(|e| anyhow::anyhow!("Failed to execute search: {}", e))?;

    let mut uid_vec: Vec<u32> = uids.into_iter().collect();
    uid_vec.sort_unstable_by(|a, b| b.cmp(a));

    if let Some(limit) = limit {
        uid_vec.truncate(limit);
    }

    Ok(uid_vec)
}

pub fn fetch_headers(
    client: &mut ImapClient,
    mailbox: &str,
    uids: &[u32],
) -> Result<Vec<EmailHeader>> {
    if uids.is_empty() {
        return Ok(Vec::new());
    }

    client.examine_mailbox(mailbox)?;

    let uid_set = build_uid_set(uids);

    let messages = client
        .session()
        .uid_fetch(&uid_set, "(UID ENVELOPE FLAGS INTERNALDATE RFC822.SIZE)")
        .map_err(|e| anyhow::anyhow!("Failed to fetch headers: {}", e))?;

    let mut headers = Vec::new();
    for msg in messages.iter() {
        if let Some(envelope) = msg.envelope() {
            headers.push(EmailHeader {
                uid: msg.uid.unwrap_or(0),
                subject: envelope
                    .subject
                    .as_ref()
                    .and_then(|s| String::from_utf8(s.to_vec()).ok()),
                from: envelope.from.as_ref().map(|addrs| format_addresses(addrs)),
                to: envelope.to.as_ref().map(|addrs| format_addresses(addrs)),
                date: envelope
                    .date
                    .as_ref()
                    .and_then(|d| String::from_utf8(d.to_vec()).ok()),
                flags: msg.flags().iter().map(|f| format!("{:?}", f)).collect(),
                size: msg.size,
            });
        }
    }

    Ok(headers)
}

pub fn fetch_email(
    client: &mut ImapClient,
    mailbox: &str,
    uid: u32,
    include_html: bool,
    include_text: bool,
) -> Result<EmailContent> {
    client.examine_mailbox(mailbox)?;

    let messages = client
        .session()
        .uid_fetch(uid.to_string(), "RFC822")
        .map_err(|e| anyhow::anyhow!("Failed to fetch email: {}", e))?;

    let message = messages
        .iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Email not found"))?;

    let body = message
        .body()
        .ok_or_else(|| anyhow::anyhow!("Email has no body"))?;

    let parsed = parse_email_body(body, include_html, include_text)?;

    Ok(parsed)
}

fn build_search_query(criteria: &crate::types::SearchCriteria) -> Result<String> {
    if let Some(raw) = &criteria.raw_query {
        return Ok(raw.clone());
    }

    let mut parts = Vec::new();

    if let Some(from) = &criteria.from {
        parts.push(format!("FROM \"{}\"", escape_query_string(from)));
    }
    if let Some(to) = &criteria.to {
        parts.push(format!("TO \"{}\"", escape_query_string(to)));
    }
    if let Some(subject) = &criteria.subject {
        parts.push(format!("SUBJECT \"{}\"", escape_query_string(subject)));
    }
    if let Some(since) = &criteria.since_date {
        parts.push(format!("SINCE {}", since));
    }
    if let Some(before) = &criteria.before_date {
        parts.push(format!("BEFORE {}", before));
    }
    if criteria.unseen_only {
        parts.push("UNSEEN".to_string());
    }

    if parts.is_empty() {
        Ok("ALL".to_string())
    } else {
        Ok(parts.join(" "))
    }
}

fn escape_query_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn build_uid_set(uids: &[u32]) -> String {
    uids.iter()
        .map(|u| u.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn format_addresses(addrs: &[Address]) -> Vec<String> {
    addrs
        .iter()
        .filter_map(|addr| {
            let name = addr
                .name
                .as_ref()
                .and_then(|n| String::from_utf8(n.to_vec()).ok());
            let mailbox = addr
                .mailbox
                .as_ref()
                .and_then(|m| String::from_utf8(m.to_vec()).ok());
            let host = addr
                .host
                .as_ref()
                .and_then(|h| String::from_utf8(h.to_vec()).ok());

            match (mailbox, host) {
                (Some(m), Some(h)) => {
                    if let Some(n) = name {
                        Some(format!("{} <{}@{}>", n, m, h))
                    } else {
                        Some(format!("{}@{}", m, h))
                    }
                }
                _ => None,
            }
        })
        .collect()
}

fn parse_email_body(body: &[u8], include_html: bool, include_text: bool) -> Result<EmailContent> {
    let parser = MessageParser::default();
    let message = parser
        .parse(body)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse email"))?;

    let text_body = if include_text {
        message.body_text(0).map(|t| t.to_string())
    } else {
        None
    };

    let html_body = if include_html {
        message.body_html(0).map(|h| h.to_string())
    } else {
        None
    };

    let mut attachments = Vec::new();
    let mut i = 0;
    while let Some(att) = message.attachment(i) {
        attachments.push(AttachmentInfo {
            filename: att.attachment_name().map(|n| n.to_string()),
            content_type: att
                .content_type()
                .map(|ct| ct.c_type.as_ref())
                .unwrap_or("application/octet-stream")
                .to_string(),
            size: att.len(),
        });
        i += 1;
    }

    Ok(EmailContent {
        text_body,
        html_body,
        attachments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SearchCriteria;

    #[test]
    fn test_build_search_query() {
        let criteria = SearchCriteria {
            from: Some("sender@example.com".to_string()),
            to: None,
            subject: Some("Meeting".to_string()),
            since_date: None,
            before_date: None,
            unseen_only: true,
            raw_query: None,
        };

        let query = build_search_query(&criteria).unwrap();
        assert!(query.contains("FROM"));
        assert!(query.contains("SUBJECT"));
        assert!(query.contains("UNSEEN"));
    }

    #[test]
    fn test_build_uid_set() {
        let uids = vec![1, 3, 5, 7];
        let uid_set = build_uid_set(&uids);
        assert_eq!(uid_set, "1,3,5,7");
    }

    #[test]
    fn test_escape_query_string() {
        let input = r#"test "quoted" text"#;
        let escaped = escape_query_string(input);
        assert!(escaped.contains(r#"\""#));
    }
}
