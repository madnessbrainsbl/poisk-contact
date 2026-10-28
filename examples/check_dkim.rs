use poisk_contact::{dkim, models::SmtpConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 {
        return Err(
            "Usage: check_dkim INPUT.eml OUTPUT.eml KEY.pem DOMAIN SELECTOR (local files only)"
                .into(),
        );
    }
    let config = SmtpConfig {
        dkim_key_path: args[2].clone(),
        dkim_selector: args[4].clone(),
        ..Default::default()
    };
    let payload = std::fs::read_to_string(&args[0])?;
    std::fs::write(&args[1], dkim::sign_message(&payload, &config, &args[3])?)?;
    println!("Signed local fixture; no network connection or email sent.");
    Ok(())
}
