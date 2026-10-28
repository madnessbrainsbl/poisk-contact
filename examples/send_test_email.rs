use poisk_contact::{mailer, models::ContactRecord, storage::load_app_settings};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let direct = args.first().is_some_and(|arg| arg == "--direct");
    if direct {
        args.remove(0);
    }
    let (mut config, template) =
        load_app_settings("settings.json").ok_or("Saved mail settings could not be loaded")?;
    if args.first().is_some_and(|arg| arg == "--template") && (args.len() == 2 || args.len() == 3) {
        let recipient = args[1].clone();
        let lead_idx: usize = if args.len() == 3 {
            args[2].parse().unwrap_or(0)
        } else {
            0
        };
        let contacts =
            poisk_contact::storage::load_contacts_from_json("contacts.json").unwrap_or_default();
        let mut contact = if let Some(found) = contacts.get(lead_idx) {
            found.clone()
        } else {
            ContactRecord {
                name: "Astreyastom team".into(),
                company: "Astreyastom".into(),
                email: recipient.clone(),
                city: "Красноярск".into(),
                ..Default::default()
            }
        };
        contact.email = recipient.clone();
        let subject = mailer::render_template(&template.subject, &contact, &config.sender_name);
        let body = mailer::render_template(&template.body, &contact, &config.sender_name);
        assert!(!subject.contains("{name}") && !body.contains("{sender_name}"));
        args = vec![recipient, subject, body];
    }
    if args.len() != 3 {
        return Err("Usage: send_test_email [--direct] RECIPIENT SUBJECT BODY, or [--direct] --template RECIPIENT".into());
    }
    if config.direct_mx && !direct {
        return Err("Use --direct to explicitly request Direct MX".into());
    }
    config.direct_mx = direct;
    if direct {
        config.host.clear();
        config.username.clear();
        config.password.clear();
        assert!(config.direct_mx && config.host.is_empty() && config.password.is_empty());
    }
    let identity = mailer::validate_delivery_identity(&config, direct)?;
    let message_id = if direct {
        tokio::time::timeout(
            std::time::Duration::from_secs(40),
            mailer::send_direct_mx(&args[0], &config, &args[1], &args[2]),
        )
        .await
        .map_err(
            |_| "Direct attempt timed out; delivery is unconfirmed. Do not retry automatically.",
        )??
    } else {
        let transport = mailer::build_transport(&config)?;
        mailer::send_single_email(&transport, &config, &args[0], &args[1], &args[2])?
    };
    println!(
        "{}",
        serde_json::json!({
            "smtp_accepted": true,
            "verified_starttls": direct,
            "method": if direct { "direct_mx" } else { "smtp_relay" },
            "recipient": args[0],
            "subject": args[1],
            "requested_from": config.from_email,
            "reply_to": identity.reply_to,
            "message_id": message_id,
            "accepted_at": chrono::Utc::now().to_rfc3339()
        })
    );
    Ok(())
}
