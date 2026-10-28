use poisk_contact::{
    mailer::validate_delivery_identity,
    models::{EmailTemplate, SmtpConfig},
};

#[test]
fn fresh_install_requires_the_users_own_sender_identity() {
    let config = SmtpConfig::default();
    for value in [
        &config.username,
        &config.password,
        &config.from_email,
        &config.reply_to,
        &config.sender_name,
    ] {
        assert!(
            value.is_empty(),
            "Fresh installs must not contain a sender identity"
        );
    }
    for direct_mx in [false, true] {
        assert!(validate_delivery_identity(&config, direct_mx).is_err());
    }
}

#[test]
fn default_template_has_no_personal_contacts_or_portfolio() {
    let template = EmailTemplate::default();
    assert!(template.body.contains("{sender_name}"));
    assert!(template.body.contains("{company}"));
    for value in [&template.template_name, &template.subject, &template.body] {
        assert!(!value.contains('@'));
        assert!(!value.contains("http://"));
        assert!(!value.contains("https://"));
    }
}
