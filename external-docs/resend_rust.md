> ## Documentation Index
> Fetch the complete documentation index at: https://resend.com/docs/llms.txt
> Use this file to discover all available pages before exploring further.

# Send emails with Rust

> Learn how to send your first email using the Resend Rust SDK.

## Prerequisites

To get the most out of this guide, you'll need to:

* [Create an API key](https://resend.com/api-keys)

## Install

First, create a rust project with cargo and `cd` into it.

```bash theme={"theme":{"light":"github-light","dark":"vesper"}}
cargo init resend-rust-example
cd resend-rust-example
```

Next, add add the Rust Resend SDK as well as [Tokio](https://tokio.rs):

```bash theme={"theme":{"light":"github-light","dark":"vesper"}}
cargo add resend-rs
cargo add tokio -F macros,rt-multi-thread
```

The Rust SDK is Async-first so Tokio is needed.

## Send email

```rust theme={"theme":{"light":"github-light","dark":"vesper"}}
use resend_rs::types::CreateEmailBaseOptions;
use resend_rs::{Resend, Result};

#[tokio::main]
async fn main() -> Result<()> {
  let resend = Resend::new("re_xxxxxxxxx");

  let from = "Acme <onboarding@resend.dev>";
  let to = ["delivered@resend.dev"];
  let subject = "Hello World";

  let email = CreateEmailBaseOptions::new(from, to, subject)
    .with_html("<strong>It works!</strong>");

  let _email = resend.emails.send(email).await?;

  Ok(())
}
```

## Reading the API key

Instead of using `Resend::new` and hardcoding the API key, the `RESEND_API_KEY` environment variable
can be used instead. The `Resend::default()` should be used in that scenario instead.

### Reading the API key from a `.env` file

Another popular option is to use a `.env` file for environment variables. You can use the
[`dotenvy`](https://crates.io/crates/dotenvy) crate for that:

```bash theme={"theme":{"light":"github-light","dark":"vesper"}}
cargo add dotenvy
```

```rust theme={"theme":{"light":"github-light","dark":"vesper"}}
// main.rs
use dotenvy::dotenv;
use resend_rs::types::CreateEmailBaseOptions;
use resend_rs::{Resend, Result};

#[tokio::main]
async fn main() -> Result<()> {
  let _env = dotenv().unwrap();

  let resend = Resend::default();

  let from = "Acme <onboarding@resend.dev>";
  let to = ["delivered@resend.dev"];
  let subject = "Hello World";

  let email = CreateEmailBaseOptions::new(from, to, subject)
    .with_html("<strong>It works!</strong>");

  let _email = resend.emails.send(email).await?;

  Ok(())
}
```

```toml theme={"theme":{"light":"github-light","dark":"vesper"}}
# .env
RESEND_API_KEY=re_xxxxxxxxx
```

## 3. Try it yourself

<CardGroup cols={3}>
  <Card title="Basic Send" icon="arrow-up-right-from-square" href="https://github.com/resend/resend-examples/blob/main/rust-resend-examples/examples/basic_send.rs">
    Basic email sending
  </Card>

  <Card title="Attachments" icon="arrow-up-right-from-square" href="https://github.com/resend/resend-examples/blob/main/rust-resend-examples/examples/with_attachments.rs">
    Send emails with file attachments
  </Card>

  <Card title="Templates" icon="arrow-up-right-from-square" href="https://github.com/resend/resend-examples/blob/main/rust-resend-examples/examples/with_template.rs">
    Send emails using Resend hosted templates
  </Card>

  <Card title="Scheduling" icon="arrow-up-right-from-square" href="https://github.com/resend/resend-examples/blob/main/rust-resend-examples/examples/scheduled_send.rs">
    Schedule emails for future delivery
  </Card>

  <Card title="Audiences" icon="arrow-up-right-from-square" href="https://github.com/resend/resend-examples/blob/main/rust-resend-examples/examples/audiences.rs">
    Manage contacts and audiences
  </Card>

  <Card title="Domains" icon="arrow-up-right-from-square" href="https://github.com/resend/resend-examples/blob/main/rust-resend-examples/examples/domains.rs">
    Create and manage sending domains
  </Card>

  <Card title="Inbound Webhooks" icon="arrow-up-right-from-square" href="https://github.com/resend/resend-examples/blob/main/rust-resend-examples/examples/inbound.rs">
    Receive and process inbound emails
  </Card>

  <Card title="Double Opt-in" icon="arrow-up-right-from-square" href="https://github.com/resend/resend-examples/blob/main/rust-resend-examples/examples/double_optin_subscribe.rs">
    Double opt-in subscription flow
  </Card>

  <Card title="Axum App" icon="arrow-up-right-from-square" href="https://github.com/resend/resend-examples/tree/main/rust-resend-examples/axum_app">
    Full Axum web framework application
  </Card>
</CardGroup>
