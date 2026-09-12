use super::*;

#[test]
fn address_rejects_empty_and_bare_at() {
    assert!(Address::new("").is_err());
    assert!(Address::new("@").is_err());
    assert!(Address::new("no-at").is_err());
    assert!(Address::new("@x").is_err());
    assert!(Address::new("x@").is_err());
}

#[test]
fn address_with_display_name() {
    let addr = Address::with_name("a@b.test", Some("Ada")).expect("valid");
    assert_eq!(addr.email(), "a@b.test");
    assert_eq!(addr.name(), Some("Ada"));
}

#[test]
fn email_builder_sets_headers_body_and_attachment() {
    let email = Email::new()
        .from("from@example.test")
        .expect("from")
        .to("to@example.test")
        .expect("to")
        .cc("cc@example.test")
        .expect("cc")
        .bcc("bcc@example.test")
        .expect("bcc")
        .reply_to("reply@example.test")
        .expect("reply")
        .subject("Hello")
        .text("plain")
        .html("<p>html</p>")
        .attach(Attachment::from_bytes(
            "note.txt",
            "text/plain",
            b"hi".as_slice(),
        ));

    assert_eq!(email.from_addresses()[0].email(), "from@example.test");
    assert_eq!(email.to_addresses()[0].email(), "to@example.test");
    assert_eq!(email.cc_addresses()[0].email(), "cc@example.test");
    assert_eq!(email.bcc_addresses()[0].email(), "bcc@example.test");
    assert_eq!(email.reply_to_addresses()[0].email(), "reply@example.test");
    assert_eq!(email.subject_line(), "Hello");
    assert_eq!(email.message_body().text_part(), Some("plain"));
    assert_eq!(email.message_body().html_part(), Some("<p>html</p>"));
    assert_eq!(email.attachments().len(), 1);
    assert_eq!(email.attachments()[0].filename(), "note.txt");
    assert_eq!(email.attachments()[0].body(), b"hi");
}

#[test]
fn body_helpers() {
    assert!(Body::empty().is_empty());
    assert_eq!(Body::text("a").text_part(), Some("a"));
    assert_eq!(Body::html("<b>").html_part(), Some("<b>"));
    let both = Body::both("t", "h");
    assert_eq!(both.text_part(), Some("t"));
    assert_eq!(both.html_part(), Some("h"));
}

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}
