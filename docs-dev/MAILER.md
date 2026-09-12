# Mailer

Email message and Mime lite types live in **`serenade-mailer`** ([#153](https://github.com/Interchouette-ITC/Serenade/issues/153)).

This crate defines message shapes only. Transports (null, file, SMTP) and DI registration are documented when that slice lands.

## Types

| Type | Role |
| --- | --- |
| `Address` | Mailbox string with optional display name |
| `Body` | Plain text and/or HTML |
| `Attachment` | Filename, MIME type, raw bytes |
| `Email` | Builder for headers, subject, body, attachments |
| `MailerError` | Invalid address while building |

## Example

```rust
use serenade_mailer::{Attachment, Email};

let email = Email::new()
    .from("shop@example.test")?
    .to("buyer@example.test")?
    .subject("Order confirmation")
    .text("Thanks for your order.")
    .html("<p>Thanks for your order.</p>")
    .attach(Attachment::from_bytes(
        "receipt.txt",
        "text/plain",
        b"order-1".as_slice(),
    ));

assert_eq!(email.subject_line(), "Order confirmation");
```

## Related

- Parent epic: [#145](https://github.com/Interchouette-ITC/Serenade/issues/145)
- Transports / DI: [#152](https://github.com/Interchouette-ITC/Serenade/issues/152)
