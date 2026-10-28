use poisk_contact::models::{SearchFilter, SendStatus};
use poisk_contact::search::{SearchProgress, run_hybrid_search};
use poisk_contact::storage::{
    export_contacts_to_csv, export_contacts_to_vcf, load_app_settings, load_contacts_from_json,
    save_contacts_to_json,
};
use std::path::PathBuf;
use std::sync::mpsc::channel;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut city = "Remote Worldwide".to_string();
    let mut stack = "AppSec / Cybersecurity".to_string();
    let mut max_items = 25;
    let mut output_json = PathBuf::from("contacts.json");
    let mut output_csv = PathBuf::from("contacts.csv");

    let mut check_port25 = false;
    let mut direct_mx_target = None;
    let mut relay_target = None;
    let mut send_contacts = false;
    let mut send_followups = false;
    let mut sync_replies = false;
    let mut enrich_contacts = false;
    let mut dry_run = false;
    let mut export_vcf = false;
    let mut export_csv = false;
    let mut audit_domain_target: Option<String> = None;
    let mut check_template = false;
    let mut detect_catchall_target: Option<String> = None;
    let mut check_dnsbl_target: Option<String> = None;
    let mut spintax_preview = false;
    let mut permutate = false;
    let mut perm_first = String::new();
    let mut perm_last = String::new();
    let mut perm_domain = String::new();
    let mut only_personal = false;
    let mut min_score = 0u8;

    let mut detect_tech_url: Option<String> = None;
    let mut recon_domain_target: Option<String> = None;
    let mut domain_pattern_target: Option<String> = None;
    let mut fix_typos = false;
    let mut warmup_plan_max: Option<usize> = None;
    let mut extract_firmo_url: Option<String> = None;
    let mut sanitize_db = false;
    let mut followups_step: Option<usize> = None;
    let mut dork_ats = false;
    let mut dork_persona = "creative".to_string();

    let mut custom_from = String::new();
    let mut custom_reply_to = String::new();

    let mut batch_target = None;
    let mut batch_relay_target = None;
    let mut batch_count = 100usize;
    let mut batch_delay_ms = 400u64;

    let mut dedup_merge = false;
    let mut audit_phone_target: Option<String> = None;
    let mut github_search_target: Option<String> = None;
    let mut audit_campaign = false;
    let mut tax_audit_target: Option<String> = None;
    let mut test_alert_msg: Option<String> = None;
    let mut audit_mta_sts_target: Option<String> = None;
    let mut audit_bimi_target: Option<String> = None;
    let mut audit_compliance_target: Option<String> = None;
    let mut shadow_recon_target: Option<String> = None;
    let mut export_crm_target: Option<String> = None;
    let mut smart_enrich = false;
    let mut probe_well_known_target: Option<String> = None;
    let mut mine_pgp_target: Option<String> = None;
    let mut audit_security_target: Option<String> = None;
    let mut verify_permutations = false;
    let mut bounce_forecast = false;
    let mut prune_risky = false;
    let mut warmup_matrix = false;
    let mut target_volume: usize = 50;
    let mut warmup_speed = "standard".to_string();
    let mut analyze_outreach = false;
    let mut audit_lookalikes_target: Option<String> = None;
    let mut audit_mx_target: Option<String> = None;
    let mut classify_mailbox_target: Option<String> = None;
    let mut probe_smtp_target: Option<String> = None;
    let mut audit_spam = false;
    let mut detect_commerce_target: Option<String> = None;
    let mut classify_title_target: Option<String> = None;
    let mut prospect_domain_target: Option<String> = None;
    let mut detect_intent_target: Option<String> = None;
    let mut recon_mail_infra_target: Option<String> = None;
    let mut generate_unsub_target: Option<String> = None;
    let mut audit_voip_target: Option<String> = None;
    let mut audit_links = false;
    let mut score_catchall_email: Option<String> = None;
    let mut score_catchall_mx: Option<String> = None;
    let mut detect_cloud_url: Option<String> = None;
    let mut optimize_subject_target: Option<String> = None;
    let mut guard_trap_email: Option<String> = None;
    let mut sender_health_target: Option<String> = None;
    let mut classify_company_target: Option<String> = None;
    let mut classify_bounce_target: Option<String> = None;
    let mut check_window_target: Option<String> = None;
    let mut resolve_mx_target: Option<String> = None;
    let mut audit_mime = false;
    let mut probe_starttls_target: Option<String> = None;
    let mut audit_ttl_target: Option<String> = None;
    let mut mine_social_proof_target: Option<String> = None;
    let mut audit_fcrdns_target: Option<String> = None;
    let mut detect_typo_target: Option<String> = None;
    let mut audit_role_risk_target: Option<String> = None;
    let mut score_tech_maturity_target: Option<String> = None;
    let mut audit_spf_lookups_target: Option<String> = None;
    let mut audit_reading_load_target: Option<String> = None;
    let mut audit_dmarc_target: Option<String> = None;
    let mut audit_spam_matrix_target: Option<String> = None;
    let mut synthesize_patterns_flag = false;
    let mut validate_fcrdns_target: Option<String> = None;
    let mut audit_cta_friction_target: Option<String> = None;
    let mut audit_greylist_target: Option<String> = None;
    let mut normalize_title_target: Option<String> = None;
    let mut audit_mx_redundancy_target: Option<String> = None;
    let mut preflight_compliance_target: Option<String> = None;
    let mut classify_reply_target: Option<String> = None;
    let mut optimize_spf_target: Option<String> = None;
    let mut profile_framework_target: Option<String> = None;
    let mut audit_mailbox_archetype_target: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--probe-smtp" if i + 1 < args.len() => {
                probe_smtp_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--analyze-outreach" => {
                analyze_outreach = true;
                i += 1;
            }
            "--audit-lookalikes" if i + 1 < args.len() => {
                audit_lookalikes_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-mx" if i + 1 < args.len() => {
                audit_mx_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--classify-mailbox" if i + 1 < args.len() => {
                classify_mailbox_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-spam" => {
                audit_spam = true;
                i += 1;
            }
            "--detect-commerce" if i + 1 < args.len() => {
                detect_commerce_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--classify-title" if i + 1 < args.len() => {
                classify_title_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--prospect-domain" if i + 1 < args.len() => {
                prospect_domain_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--detect-intent" if i + 1 < args.len() => {
                detect_intent_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--recon-mail-infra" if i + 1 < args.len() => {
                recon_mail_infra_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--generate-unsubscribe" if i + 1 < args.len() => {
                generate_unsub_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-voip" if i + 1 < args.len() => {
                audit_voip_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-links" => {
                audit_links = true;
                i += 1;
            }
            "--score-catchall" if i + 1 < args.len() => {
                score_catchall_email = Some(args[i + 1].clone());
                i += 2;
                if i < args.len() && !args[i].starts_with("--") {
                    score_catchall_mx = Some(args[i].clone());
                    i += 1;
                }
            }
            "--detect-cloud" if i + 1 < args.len() => {
                detect_cloud_url = Some(args[i + 1].clone());
                i += 2;
            }
            "--optimize-subject" if i + 1 < args.len() => {
                optimize_subject_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--guard-trap" if i + 1 < args.len() => {
                guard_trap_email = Some(args[i + 1].clone());
                i += 2;
            }
            "--sender-health" if i + 1 < args.len() => {
                sender_health_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--classify-company" if i + 1 < args.len() => {
                classify_company_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--classify-bounce" if i + 1 < args.len() => {
                classify_bounce_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--check-window" if i + 1 < args.len() && !args[i + 1].starts_with("--") => {
                check_window_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--check-window" => {
                check_window_target = Some("MSK".to_string());
                i += 1;
            }
            "--resolve-mx" if i + 1 < args.len() => {
                resolve_mx_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-mime" => {
                audit_mime = true;
                i += 1;
            }
            "--probe-starttls" if i + 1 < args.len() => {
                probe_starttls_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-ttl" if i + 1 < args.len() => {
                audit_ttl_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--mine-social-proof" if i + 1 < args.len() => {
                mine_social_proof_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-fcrdns" if i + 1 < args.len() => {
                audit_fcrdns_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--detect-typo" if i + 1 < args.len() => {
                detect_typo_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-role-risk" if i + 1 < args.len() => {
                audit_role_risk_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--score-tech-maturity" if i + 1 < args.len() => {
                score_tech_maturity_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-spf-lookups" if i + 1 < args.len() => {
                audit_spf_lookups_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-reading-load" if i + 1 < args.len() && !args[i + 1].starts_with("--") => {
                audit_reading_load_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-reading-load" => {
                audit_reading_load_target = Some("template".to_string());
                i += 1;
            }
            "--audit-dmarc" if i + 1 < args.len() => {
                audit_dmarc_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-spam-matrix" if i + 1 < args.len() && !args[i + 1].starts_with("--") => {
                audit_spam_matrix_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-spam-matrix" => {
                audit_spam_matrix_target = Some("template".to_string());
                i += 1;
            }
            "--synthesize-patterns" => {
                synthesize_patterns_flag = true;
                i += 1;
            }
            "--validate-fcrdns" if i + 1 < args.len() => {
                validate_fcrdns_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-cta-friction" if i + 1 < args.len() && !args[i + 1].starts_with("--") => {
                audit_cta_friction_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-cta-friction" => {
                audit_cta_friction_target = Some("template".to_string());
                i += 1;
            }
            "--audit-greylist" if i + 1 < args.len() => {
                audit_greylist_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--normalize-title" if i + 1 < args.len() => {
                normalize_title_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-mx-redundancy" if i + 1 < args.len() => {
                audit_mx_redundancy_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--preflight-compliance" if i + 1 < args.len() && !args[i + 1].starts_with("--") => {
                preflight_compliance_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--preflight-compliance" => {
                preflight_compliance_target = Some("template".to_string());
                i += 1;
            }
            "--classify-reply" if i + 1 < args.len() => {
                classify_reply_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--optimize-spf" if i + 1 < args.len() => {
                optimize_spf_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--profile-framework" if i + 1 < args.len() => {
                profile_framework_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-mailbox-archetype" if i + 1 < args.len() => {
                audit_mailbox_archetype_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-security" if i + 1 < args.len() => {
                audit_security_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--verify-permutations" => {
                verify_permutations = true;
                i += 1;
            }
            "--bounce-forecast" => {
                bounce_forecast = true;
                i += 1;
            }
            "--prune" => {
                prune_risky = true;
                i += 1;
            }
            "--warmup-matrix" => {
                warmup_matrix = true;
                i += 1;
            }
            "--target-volume" if i + 1 < args.len() => {
                if let Ok(v) = args[i + 1].parse::<usize>() {
                    target_volume = v;
                }
                i += 2;
            }
            "--speed" if i + 1 < args.len() => {
                warmup_speed = args[i + 1].clone();
                i += 2;
            }
            "--audit-mta-sts" if i + 1 < args.len() => {
                audit_mta_sts_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-bimi" if i + 1 < args.len() => {
                audit_bimi_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-compliance" if i + 1 < args.len() => {
                audit_compliance_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--shadow-recon" if i + 1 < args.len() => {
                shadow_recon_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--probe-well-known" if i + 1 < args.len() => {
                probe_well_known_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--mine-pgp" if i + 1 < args.len() => {
                mine_pgp_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--export-crm" if i + 1 < args.len() => {
                export_crm_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--smart-enrich" | "--full-pipeline" => {
                smart_enrich = true;
                i += 1;
            }
            "--dork-ats" => {
                dork_ats = true;
                i += 1;
            }
            "--dedup-merge" => {
                dedup_merge = true;
                i += 1;
            }
            "--audit-phone" if i + 1 < args.len() => {
                audit_phone_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--github-search" if i + 1 < args.len() => {
                github_search_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--audit-campaign" => {
                audit_campaign = true;
                i += 1;
            }
            "--tax-audit" if i + 1 < args.len() => {
                tax_audit_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--test-alert" if i + 1 < args.len() => {
                test_alert_msg = Some(args[i + 1].clone());
                i += 2;
            }
            "--check-port25" => {
                check_port25 = true;
                i += 1;
            }
            "--audit-sender" if i + 1 < args.len() => {
                audit_domain_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--check-template" => {
                check_template = true;
                i += 1;
            }
            "--detect-catchall" if i + 1 < args.len() => {
                detect_catchall_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--check-dnsbl" if i + 1 < args.len() => {
                check_dnsbl_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--spintax-preview" => {
                spintax_preview = true;
                i += 1;
            }
            "--permutate" => {
                permutate = true;
                i += 1;
            }
            "--first" if i + 1 < args.len() => {
                perm_first = args[i + 1].clone();
                i += 2;
            }
            "--last" if i + 1 < args.len() => {
                perm_last = args[i + 1].clone();
                i += 2;
            }
            "--domain" if i + 1 < args.len() => {
                perm_domain = args[i + 1].clone();
                i += 2;
            }
            "--only-personal" => {
                only_personal = true;
                i += 1;
            }
            "--min-score" if i + 1 < args.len() => {
                if let Ok(s) = args[i + 1].parse::<u8>() {
                    min_score = s;
                }
                i += 2;
            }
            "--detect-tech" if i + 1 < args.len() => {
                detect_tech_url = Some(args[i + 1].clone());
                i += 2;
            }
            "--recon-domain" if i + 1 < args.len() => {
                recon_domain_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--domain-pattern" if i + 1 < args.len() => {
                domain_pattern_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--fix-typos" => {
                fix_typos = true;
                i += 1;
            }
            "--warmup-plan" if i + 1 < args.len() => {
                if let Ok(m) = args[i + 1].parse::<usize>() {
                    warmup_plan_max = Some(m);
                } else {
                    warmup_plan_max = Some(50);
                }
                i += 2;
            }
            "--warmup-plan" => {
                warmup_plan_max = Some(50);
                i += 1;
            }
            "--extract-firmographics" if i + 1 < args.len() => {
                extract_firmo_url = Some(args[i + 1].clone());
                i += 2;
            }
            "--sanitize-db" => {
                sanitize_db = true;
                i += 1;
            }
            "--send-direct-mx" if i + 1 < args.len() => {
                direct_mx_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--send-relay" if i + 1 < args.len() => {
                relay_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--batch-direct-mx" if i + 1 < args.len() => {
                batch_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--batch-relay" if i + 1 < args.len() => {
                batch_relay_target = Some(args[i + 1].clone());
                i += 2;
            }
            "--send-contacts" => {
                send_contacts = true;
                i += 1;
            }
            "--send-followups"
                if i + 1 < args.len() && (args[i + 1] == "1" || args[i + 1] == "2") =>
            {
                send_followups = true;
                followups_step = args[i + 1].parse().ok();
                i += 2;
            }
            "--send-followups" => {
                send_followups = true;
                i += 1;
            }
            "--sync-replies" => {
                sync_replies = true;
                i += 1;
            }
            "--enrich-contacts" => {
                enrich_contacts = true;
                i += 1;
            }
            "--export-vcf" => {
                export_vcf = true;
                i += 1;
            }
            "--export-csv" => {
                export_csv = true;
                i += 1;
            }
            "--count" if i + 1 < args.len() => {
                if let Ok(c) = args[i + 1].parse::<usize>() {
                    batch_count = c;
                }
                i += 2;
            }
            "--delay-ms" if i + 1 < args.len() => {
                if let Ok(d) = args[i + 1].parse::<u64>() {
                    batch_delay_ms = d;
                }
                i += 2;
            }
            "--from" if i + 1 < args.len() => {
                custom_from = args[i + 1].clone();
                i += 2;
            }
            "--reply-to" if i + 1 < args.len() => {
                custom_reply_to = args[i + 1].clone();
                i += 2;
            }
            "--city" if i + 1 < args.len() => {
                city = args[i + 1].clone();
                i += 2;
            }
            "--stack" if i + 1 < args.len() => {
                stack = args[i + 1].clone();
                i += 2;
            }
            "--max" if i + 1 < args.len() => {
                if let Ok(n) = args[i + 1].parse::<usize>() {
                    max_items = n;
                }
                i += 2;
            }
            "--output" if i + 1 < args.len() => {
                output_json = PathBuf::from(&args[i + 1]);
                output_csv = output_json.with_extension("csv");
                i += 2;
            }
            "--persona" if i + 1 < args.len() => {
                let p_arg = args[i + 1].to_lowercase();
                dork_persona = if p_arg.contains("sec") || p_arg.contains("app") {
                    "appsec".to_string()
                } else {
                    "creative".to_string()
                };
                let profiles_path = if std::path::Path::new("profiles.json").exists() {
                    PathBuf::from("profiles.json")
                } else if std::path::Path::new("../profiles.json").exists() {
                    PathBuf::from("../profiles.json")
                } else {
                    PathBuf::from("profiles.json")
                };
                if let Ok(content) = std::fs::read_to_string(&profiles_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        let p_key = if p_arg.contains("sec") || p_arg.contains("app") {
                            "cybersec"
                        } else {
                            "creative"
                        };
                        if let Some(p) = val.get("personas").and_then(|ps| ps.get(p_key)) {
                            let (mut smtp_cfg, _) = load_app_settings(&PathBuf::from(
                                "settings.json",
                            ))
                            .unwrap_or_else(|| {
                                (
                                    poisk_contact::models::SmtpConfig::default(),
                                    poisk_contact::models::EmailTemplate::default(),
                                )
                            });
                            let sender_name =
                                p.get("sender_name").and_then(|v| v.as_str()).unwrap_or("");
                            let reply_to = p.get("email").and_then(|v| v.as_str()).unwrap_or("");
                            smtp_cfg.sender_name = sender_name.to_string();
                            smtp_cfg.reply_to = reply_to.to_string();
                            let tmpl_val = p.get("outreach_template");
                            let tmpl = poisk_contact::models::EmailTemplate {
                                template_name: tmpl_val
                                    .and_then(|t| t.get("template_name"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                subject: tmpl_val
                                    .and_then(|t| t.get("subject"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                body: tmpl_val
                                    .and_then(|t| t.get("body"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                follow_up_1: tmpl_val
                                    .and_then(|t| t.get("follow_up_1"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                follow_up_2: tmpl_val
                                    .and_then(|t| t.get("follow_up_2"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                            };
                            let _ = poisk_contact::storage::save_app_settings(
                                &smtp_cfg,
                                &tmpl,
                                &PathBuf::from("settings.json"),
                            );
                            println!(
                                "[PERSONA] Switched active profile to '{}' ({})",
                                p_key, sender_name
                            );
                        }
                    }
                }
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--help" | "-h" => {
                println!("============================================================");
                println!("  PoiskContact Pro — CLI OSINT Search Engine & Outreach");
                println!("============================================================");
                println!();
                println!("Использование: poisk_contact_cli.exe [ОПЦИИ]");
                println!();
                println!("Опции поиска контактов:");
                println!(
                    "  --city <ГОРОД>          Целевая локация (по умолчанию: Remote Worldwide)"
                );
                println!(
                    "  --stack <СТЕК>          Стек или отрасль (по умолчанию: AppSec / Cybersecurity)"
                );
                println!("  --max <ЧИСЛО>           Лимит собираемых записей (по умолчанию: 25)");
                println!(
                    "  --output <ФАЙЛ>         Путь к файлу сохранения JSON (по умолчанию: contacts.json)"
                );
                println!();
                println!("Опции аудита и валидации доставляемости (Reacher/Hunter/Instantly):");
                println!(
                    "  --audit-sender <ДОМЕН>   Полный аудит домена отправителя (MX, SPF, DMARC, DNSBL)"
                );
                println!(
                    "  --check-template         Анализ шаблона письма на спам-триггеры и стоп-слова"
                );
                println!(
                    "  --detect-catchall <ДОМЕН> Проверить домен на Catch-All конфигурацию (порт 25)"
                );
                println!(
                    "  --check-dnsbl <ДОМЕН>    Проверить домен по базам черных списков (Spamhaus, SORBS)"
                );
                println!(
                    "  --spintax-preview        Сгенерировать 5 вариантов письма из Spintax шаблона"
                );
                println!(
                    "  --permutate              Сгенерировать email кандидатов по имени, фамилии и домену"
                );
                println!("    --first <ИМЯ>          Имя контакта");
                println!("    --last <ФАМИЛИЯ>       Фамилия контакта");
                println!("    --domain <ДОМЕН>       Домен компании");
                println!(
                    "  --extract-firmographics <URL> Извлечь фирмаграфику (юрлицо, описание, адрес, соцсети)"
                );
                println!(
                    "  --sanitize-db            Очистить базу: нормализация email, удаление plus-тегов, B2B Quality Index"
                );
                println!(
                    "  --detect-tech <URL>      Определить CMS, CRM, онлайн-чат и ID аналитики сайта"
                );
                println!(
                    "  --recon-domain <ДОМЕН>   OSINT аудит домена: возраст и регистратор (RDAP), security.txt, поддомены crt.sh"
                );
                println!(
                    "  --domain-pattern <ДОМЕН> Рассчитать доминантный шаблон email компании (Hunter.io style)"
                );
                println!(
                    "  --fix-typos              Найти и автоматически исправить опечатки в доменах базы"
                );
                println!(
                    "  --warmup-plan [ЛИМИТ]    Построить 14-дневный график плавного прогрева ящика"
                );
                println!();
                println!("Опции рассылки и аутрича:");
                println!(
                    "  --send-contacts         Разослать предложения по контактам со статусом 'Ожидает'"
                );
                println!(
                    "  --only-personal          Фильтр: пропускать общие ящики (info@, support@, admin@)"
                );
                println!(
                    "  --min-score <ЧИСЛО>      Фильтр: отправлять только контактам с Lead Score >= ЧИСЛО"
                );
                println!(
                    "  --send-followups [1|2]  Разослать Drip Follow-Up (шаг 1: 3+ дня, шаг 2: 7+ дней в едином треде)"
                );
                println!(
                    "  --sync-replies          Синхронизировать входящие ответы и баунсы из IMAP (Gmail)"
                );
                println!("  --dry-run               Тестовый режим (без реальной отправки писем)");
                println!("  --count <ЧИСЛО>         Лимит отправляемых писем в этой сессии");
                println!(
                    "  --persona <ИМЯ>         Переключить активную персону ('creative' или 'cybersec')"
                );
                println!(
                    "  --check-port25          Проверить доступность исходящего порта 25 (Direct MX)"
                );
                println!(
                    "  --send-direct-mx <EMAIL> Отправить тестовое письмо напрямую через DNS MX"
                );
                println!(
                    "  --send-relay <EMAIL>    Отправить тестовое письмо через настроенный SMTP-релей"
                );
                println!(
                    "  --enrich-contacts       Обогатить базу контактов через краулер сайтов и пермутатор"
                );
                println!(
                    "  --export-vcf            Экспортировать базу контактов в формат vCard (.vcf) для смартфона/Telegram"
                );
                println!(
                    "  --export-crm <amocrm|bitrix24> Экспорт в CSV схему AmoCRM или Bitrix24 для прямого импорта"
                );
                println!(
                    "  --dork-ats               Поиск скрытых и неиндексированных вакансий в ATS (Greenhouse, Ashby, Lever)"
                );
                println!(
                    "  --dedup-merge            Waterfall слияние дубликатов базы контактов (Dropcontact style)"
                );
                println!(
                    "  --smart-enrich           Сквозной автоматический пайплайн: DSU дедуп + телефон + ИНН + найм + айсбрейкер"
                );
                println!(
                    "  --audit-phone <НОМЕР>    Телеком-анализ номера: E.164, оператор, регион, тип линии, ссылки (PhoneInfoga)"
                );
                println!(
                    "  --github-search <EMAIL>  OSINT поиск разработчика на GitHub по email (User-Scanner style)"
                );
                println!(
                    "  --audit-campaign         Pre-flight расчет риска спама кампании перед запуском (Folderly style)"
                );
                println!(
                    "  --audit-mta-sts <ДОМЕН>  Аудит стандарта шифрования MTA-STS и TLS-RPT (RFC 8461/8460)"
                );
                println!(
                    "  --audit-bimi <ДОМЕН>     Проверка BIMI аватара бренда и VMC сертификата в почтовых сервисах"
                );
                println!(
                    "  --audit-compliance <URL> Проверка юридического комплаенса сайта (152-ФЗ РФ и GDPR ЕС)"
                );
                println!(
                    "  --shadow-recon <ДОМЕН>   Разведка Certificate Transparency и сбор контактов со скрытых поддоменов"
                );
                println!(
                    "  --tax-audit <ТЕКСТ/URL>  Проверка ИНН, ОГРН, КПП и европейского VAT (Rusprofile style)"
                );
                println!(
                    "  --test-alert <ТЕКСТ>     Отправка тестового уведомления в Telegram Bot и Webhook"
                );
                println!(
                    "  --audit-security <URL>   Аудит заголовков веб-безопасности B2B (HSTS, CSP, XFO, nosniff, grade A+..F)"
                );
                println!(
                    "  --verify-permutations    Hunter-style live проверка email пермутаций с защитой Catch-All (--first, --last, --domain)"
                );
                println!(
                    "  --bounce-forecast [--prune] Прогноз Bounce Rate (NeverBounce style) с опцией очистки базы от рисковых адресов"
                );
                println!(
                    "  --warmup-matrix          30-дневная матрица прогрева ящика (--target-volume <N>, --speed <conservative|standard|aggressive>)"
                );
                println!(
                    "  --analyze-outreach       Психологический анализ текста и темы аутрича (Lavender.ai & Hemingway style)"
                );
                println!(
                    "  --audit-lookalikes <ДОМЕН> Сканер доменов-двойников и тайпсквоттинга (dnstwist style) через DoH"
                );
                println!(
                    "  --audit-mx <ДОМЕН/MX>    Фингерпринтинг входящего спам-шлюза (Proofpoint, Mimecast, Google, MS 365, Yandex)"
                );
                println!(
                    "  --classify-mailbox <EMAIL> Определение типа ящика: Корпоративный B2B, Free Webmail или Одноразовый (Kickbox)"
                );
                println!(
                    "  --audit-spam             Аудит письма на спам-триггеры и стоп-слова (Mail-Tester & SpamAssassin)"
                );
                println!(
                    "  --detect-commerce <URL>  Детекция платежных систем и e-commerce движков (BuiltWith / Wappalyzer style)"
                );
                println!(
                    "  --classify-title <ДОЛЖНОСТЬ> Классификация грейда ЛПР и департамента (Apollo.io style)"
                );
                println!(
                    "  --prospect-domain <ДОМЕН> Сбор сотрудников компании по корпоративным роутам (Snov.io style)"
                );
                println!(
                    "  --detect-intent <URL>    Аудит рекламных пикселей и готовности к закупкам (BuiltWith AdTech style)"
                );
                println!(
                    "  --recon-mail-infra <ДОМЕН> DoH разведка скрытых почтовых поддоменов (theHarvester style)"
                );
                println!(
                    "  --generate-unsubscribe <EMAIL> Генерация RFC 8058 One-Click List-Unsubscribe заголовков"
                );
                println!(
                    "  --audit-voip <НОМЕР>     Детекция облачных VoIP АТС vs реальных мобильных SIM (Kaspr/Lusha style)"
                );
                println!(
                    "  --audit-links            Аудит плотности ссылок и опасных редиректов в письме (Mailmeteor/Woodpecker style)"
                );
                println!(
                    "  --score-catchall <EMAIL> [MX] Статистическая вероятность доставки на Catch-All ящик (Hunter.io style)"
                );
                println!(
                    "  --detect-cloud <URL>     Детекция облачной инфраструктуры и CDN (AWS, Cloudflare, GCP, Azure, Selectel)"
                );
                println!(
                    "  --optimize-subject <ТЕМА> Оптимизация темы письма под мобильные экраны (<=40 симв., A/B варианты)"
                );
                println!(
                    "  --guard-trap <EMAIL>     Проверка email на Honey Pot и спам-ловушки (ZeroBounce / Spamhaus style)"
                );
                println!(
                    "  --sender-health <ДОМЕН>  Комплексный индекс здоровья отправителя и грейд A+..F (Smartlead / GlockApps)"
                );
                println!(
                    "  --classify-company <URL> Определение размера штата и вертикали компании (Clearbit / Apollo.io)"
                );
                println!(
                    "  --classify-bounce <КОД/ТЕКСТ> Классификация SMTP DSN баунса (Hard/Soft/SpamBlock - SendGrid/Mailgun)"
                );
                println!(
                    "  --check-window [ЧАСОВОЙ_ПОЯС] Проверка рабочих часов аутрича (09:00 - 18:00 Пн-Пт - Instantly/Lemlist)"
                );
                println!(
                    "  --resolve-mx <ДОМЕН>     Изоляция первичного шлюза MX и аудит отказоустойчивости (Reacher/Truemail)"
                );
                println!(
                    "  --audit-mime            Аудит MIME-структуры письма: HTML/текст баланс, скрытый текст, zero-font traps (Mailmeteor)"
                );
                println!(
                    "  --probe-starttls <ХОСТ>  Зондирование порта 25 на STARTTLS и расширения SMTP (Mailgun/Amazon SES)"
                );
                println!(
                    "  --audit-ttl <ДОМЕН>      Аудит TTL записей и риска эфемеридности Fast-Flux ботнетов (ZeroBounce/Spamhaus)"
                );
                println!(
                    "  --mine-social-proof <URL> Майнинг кейсов, логотипов клиентов и сертификатов ФСТЭК/ISO/Сколково (Clay/Apollo)"
                );
                println!(
                    "  --audit-fcrdns <IP>      Проверка Forward-Confirmed Reverse DNS (FCrDNS - RFC 8601/SpamAssassin)"
                );
                println!(
                    "  --detect-typo <ДОМЕН>    Аудит опечаток и lookalike-доменов (Levenshtein distance - Hunter/ZeroBounce)"
                );
                println!(
                    "  --audit-role-risk <EMAIL> Ролевой риск ящика: ЛПР, Commercial Inbound, Generic или Support Trap (NeverBounce/ZeroBounce)"
                );
                println!(
                    "  --score-tech-maturity <URL> Индекс технологической зрелости и соответствия ICP Tier 1..3 (Apollo/Clay)"
                );
                println!(
                    "  --audit-spf-lookups <ДОМЕН/SPF> Аудит лимита 10 DNS-запросов SPF (RFC 7208 PermError - GlockApps)"
                );
                println!(
                    "  --audit-reading-load [ТЕКСТ] Оценка когнитивной нагрузки и 30-секундного правила беглого чтения (Lavender/Mailshake)"
                );
                println!(
                    "  --audit-dmarc <ДОМЕН>    Аудит политики DMARC RFC 7489 и BIMI брендинга (Google/Yahoo 2024 compliance)"
                );
                println!(
                    "  --audit-spam-matrix [ТЕКСТ] Категориальная матрица спам-триггеров (Urgency/Financial/Freebies/Hype) (Lemlist/Saleshandy)"
                );
                println!(
                    "  --synthesize-patterns   Синтез паттернов корпоративных email (Hunter.io / Clearbit) с флагами --first, --last, --domain"
                );
                println!(
                    "  --validate-fcrdns <IP>  Строгая валидация FCrDNS (RFC 8601) и детекция пулов динамических IP/PBL (Mail-Tester/Spamhaus)"
                );
                println!(
                    "  --audit-cta-friction [ТЕКСТ] Оценка трения и когнитивного барьера CTA (Interest vs Slot vs Hard Sell) (Woodpecker/Gong)"
                );
                println!(
                    "  --audit-greylist <КОД/ТЕКСТ> Детекция SMTP грейлистинга и оценка интервала повтора (Kickbox / NeverBounce)"
                );
                println!(
                    "  --normalize-title <ДОЛЖНОСТЬ> Иерархическая нормализация должности и веса ЛПР (C-Level/VP/Dir - Snov.io/Lusha)"
                );
                println!(
                    "  --audit-mx-redundancy <ДОМЕН> Аудит отказоустойчивости и сетевой диверсификации MX (MXToolbox / Postmark)"
                );
                println!(
                    "  --preflight-compliance [ТЕКСТ] Комплексный префлайт-чек RFC 8058, 152-ФЗ, CAN-SPAM и отписки (Instantly/Smartlead)"
                );
                println!(
                    "  --classify-reply <ТЕКСТ> Классификация тональности, возражений и OOO ответов лидов (Salesloft / Outreach)"
                );
                println!(
                    "  --optimize-spf <SPF/ДОМЕН> Синтаксическая оптимизация, дедупликация и аудит безопасности SPF (Valimail / Red Sift)"
                );
                println!(
                    "  --profile-framework <URL> Профилирование современных веб-фреймворков и Jamstack архитектуры (BuiltWith / Wappalyzer)"
                );
                println!(
                    "  --audit-mailbox-archetype <EMAIL> Аудит архетипа и deliverability tier почтового ящика (ZeroBounce / TrueMail)"
                );
                println!(
                    "  --export-crm <amocrm|bitrix24|instantly|lemlist> Экспорт базы лидов в CRM или платформы аутрича"
                );
                println!(
                    "  --export-csv            Экспортировать базу контактов в формат CSV (Excel UTF-8 BOM)"
                );

                println!();
                println!("Примеры:");
                println!(
                    "  .\\poisk_contact_cli.exe --classify-reply \"Тема интересная, давайте созвонимся на этой неделе\""
                );
                println!(
                    "  .\\poisk_contact_cli.exe --optimize-spf \"v=spf1 mx ~all ip4:1.2.3.4 mx\""
                );
                println!("  .\\poisk_contact_cli.exe --profile-framework https://tilda.cc");
                println!("  .\\poisk_contact_cli.exe --audit-mailbox-archetype team@startup.io");
                println!("  .\\poisk_contact_cli.exe --audit-role-risk info@company.ru");
                println!("  .\\poisk_contact_cli.exe --score-tech-maturity https://selectel.ru");
                println!("  .\\poisk_contact_cli.exe --audit-spf-lookups yandex.ru");
                println!("  .\\poisk_contact_cli.exe --audit-reading-load");
                println!(
                    "  .\\poisk_contact_cli.exe --audit-spam-matrix \"Срочно! 100% бесплатно и без вложений!\""
                );
                println!(
                    "  .\\poisk_contact_cli.exe --synthesize-patterns --first \"Алексей\" --last \"Смирнов\" --domain \"kaspersky.ru\""
                );
                println!("  .\\poisk_contact_cli.exe --validate-fcrdns 77.88.21.249");
                println!(
                    "  .\\poisk_contact_cli.exe --audit-cta-friction \"Будет ли вам интересно взглянуть на краткую презентацию?\""
                );
                println!(
                    "  .\\poisk_contact_cli.exe --audit-greylist \"451 4.7.1 Greylisting in action, please come back later\""
                );
                println!(
                    "  .\\poisk_contact_cli.exe --normalize-title \"Senior VP of Cybersecurity & Head of AppSec\""
                );
                println!("  .\\poisk_contact_cli.exe --audit-mx-redundancy yandex.ru");
                println!("  .\\poisk_contact_cli.exe --preflight-compliance");
                println!("  .\\poisk_contact_cli.exe --audit-mime");
                println!("  .\\poisk_contact_cli.exe --probe-starttls smtp.google.com");
                println!("  .\\poisk_contact_cli.exe --audit-ttl yandex.ru");
                println!("  .\\poisk_contact_cli.exe --mine-social-proof https://kaspersky.ru");
                println!("  .\\poisk_contact_cli.exe --classify-company https://yandex.ru");
                println!("  .\\poisk_contact_cli.exe --classify-bounce \"550 5.1.1 User unknown\"");
                println!("  .\\poisk_contact_cli.exe --check-window MSK");
                println!("  .\\poisk_contact_cli.exe --resolve-mx google.com");
                println!("  .\\poisk_contact_cli.exe --detect-cloud https://tilda.cc");
                println!(
                    "  .\\poisk_contact_cli.exe --optimize-subject \"Вопрос по безопасности Яндекс\""
                );
                println!("  .\\poisk_contact_cli.exe --guard-trap \"spamtrap@target.com\"");
                println!("  .\\poisk_contact_cli.exe --sender-health yandex.ru");
                println!("  .\\poisk_contact_cli.exe --audit-voip \"+7 (913) 555-01-02\"");
                println!("  .\\poisk_contact_cli.exe --audit-links");
                println!(
                    "  .\\poisk_contact_cli.exe --score-catchall \"alexey.smirnov@company.ru\" \"Google Workspace\""
                );
                println!("  .\\poisk_contact_cli.exe --prospect-domain yandex.ru");
                println!("  .\\poisk_contact_cli.exe --detect-intent https://tilda.cc");
                println!("  .\\poisk_contact_cli.exe --recon-mail-infra yandex.ru");
                println!("  .\\poisk_contact_cli.exe --generate-unsubscribe lead@company.com");
                println!("  .\\poisk_contact_cli.exe --audit-spam");
                println!("  .\\poisk_contact_cli.exe --detect-commerce https://tilda.cc");
                println!("  .\\poisk_contact_cli.exe --classify-title \"Генеральный директор\"");
                println!("  .\\poisk_contact_cli.exe --analyze-outreach");
                println!("  .\\poisk_contact_cli.exe --audit-lookalikes yandex.ru");
                println!("  .\\poisk_contact_cli.exe --audit-mx aspmx.l.google.com");
                println!("  .\\poisk_contact_cli.exe --classify-mailbox lead@tempmail.com");
                println!("  .\\poisk_contact_cli.exe --export-crm instantly");
                println!("  .\\poisk_contact_cli.exe --export-crm lemlist");
                println!("  .\\poisk_contact_cli.exe --audit-security https://yandex.ru");
                println!(
                    "  .\\poisk_contact_cli.exe --verify-permutations --first \"Павел\" --last \"Дуров\" --domain \"telegram.org\""
                );
                println!("  .\\poisk_contact_cli.exe --bounce-forecast --prune");
                println!(
                    "  .\\poisk_contact_cli.exe --warmup-matrix --target-volume 100 --speed standard"
                );
                println!("  .\\poisk_contact_cli.exe --dedup-merge");
                println!("  .\\poisk_contact_cli.exe --audit-phone \"+7 (913) 555-01-02\"");
                println!("  .\\poisk_contact_cli.exe --github-search dev@example.com");
                println!("  .\\poisk_contact_cli.exe --audit-campaign");
                println!("  .\\poisk_contact_cli.exe --tax-audit 7736207543");
                println!("  .\\poisk_contact_cli.exe --audit-sender yandex.ru");
                println!("  .\\poisk_contact_cli.exe --detect-tech https://tilda.cc");
                println!("  .\\poisk_contact_cli.exe --recon-domain yandex.ru");
                println!("  .\\poisk_contact_cli.exe --domain-pattern telegram.org");
                println!("  .\\poisk_contact_cli.exe --fix-typos");
                println!("  .\\poisk_contact_cli.exe --warmup-plan 50");
                println!("  .\\poisk_contact_cli.exe --check-template");
                println!("  .\\poisk_contact_cli.exe --spintax-preview");
                println!(
                    "  .\\poisk_contact_cli.exe --permutate --first \"Павел\" --last \"Дуров\" --domain \"telegram.org\""
                );
                println!("  .\\poisk_contact_cli.exe --detect-catchall mail.ru");
                println!(
                    "  .\\poisk_contact_cli.exe --send-contacts --only-personal --min-score 60"
                );
                println!("  .\\poisk_contact_cli.exe --dork-ats --persona creative --max 20");
                return;
            }
            _ => {
                i += 1;
            }
        }
    }

    if dedup_merge {
        println!("============================================================");
        println!("  Waterfall Deduplication & Cluster Merging (Dropcontact Style)");
        println!("  Файл базы: {}", output_json.display());
        println!("============================================================");
        let contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let (merged, stats) =
            poisk_contact::search::waterfall_merge::deduplicate_and_merge(contacts);
        let _ = save_contacts_to_json(&merged, &output_json);
        let _ = export_contacts_to_csv(&merged, &output_csv);
        println!("Исходных записей:            {}", stats.initial_count);
        println!("Итоговых уникальных записей: {}", stats.final_count);
        println!("Объединено дубликатов:       {}", stats.duplicates_merged);
        println!("Обогащено полей без потерь:  {}", stats.fields_enriched);
        println!(
            "База обновлена и сохранена в {} и {}",
            output_json.display(),
            output_csv.display()
        );
        println!("============================================================");
        return;
    }

    if let Some(raw_phone) = audit_phone_target {
        println!("============================================================");
        println!("  Telecom Phone Intelligence (PhoneInfoga Style)");
        println!("  Исходный номер: {}", raw_phone);
        println!("============================================================");
        let p = poisk_contact::search::phone_intel::parse_and_enrich_phone(&raw_phone);
        println!(
            "E.164 формат:       {}",
            if p.e164.is_empty() {
                "Невалидный"
            } else {
                &p.e164
            }
        );
        println!("Форматированный:    {}", p.formatted);
        println!("Тип линии:          {}", p.line_type);
        println!("Оператор связи:     {}", p.carrier);
        println!("Регион:             {}", p.region);
        println!(
            "Валидность:         {}",
            if p.is_valid { "ДА" } else { "НЕТ" }
        );
        if let Some(wa) = p.whatsapp_url {
            println!("WhatsApp ссылка:    {}", wa);
        }
        if let Some(tg) = p.telegram_url {
            println!("Telegram ссылка:    {}", tg);
        }
        println!("============================================================");
        return;
    }

    if let Some(raw_phone) = audit_voip_target {
        println!("============================================================");
        println!("  VoIP Cloud PBX vs Mobile SIM Verification (Kaspr / Lusha Style)");
        println!("  Номер: {}", raw_phone);
        println!("============================================================");
        let voip = poisk_contact::search::voip_detect::classify_phone_line(&raw_phone);
        println!("Форматированный:    {}", voip.phone_number);
        println!(
            "Облачная АТС (VoIP):{}",
            if voip.is_voip_cloud_pbx {
                "ДА (Mango, Zadarma, Telphin, Sipuni, UIS, Twilio)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Реальный мобильный: {}",
            if voip.is_real_mobile {
                "ДА (SIM-карта: МТС, МегаФон, Билайн, Tele2, Yota)"
            } else {
                "НЕТ"
            }
        );
        println!("Категория линии:    {}", voip.carrier_category);
        println!("Вердикт/Описание:   {}", voip.description);
        println!("============================================================");
        return;
    }

    if let Some(email) = github_search_target {
        println!("============================================================");
        println!("  Developer OSINT — Поиск профиля GitHub по Email");
        println!("  Email: {}", email);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap();
        if let Some(prof) =
            poisk_contact::search::github_osint::probe_github_user_by_email(&client, &email).await
        {
            println!("Логин на GitHub:    {}", prof.login);
            println!(
                "ФИО разработчика:   {}",
                if prof.name.is_empty() {
                    "Не указано"
                } else {
                    &prof.name
                }
            );
            println!(
                "Компания:           {}",
                if prof.company.is_empty() {
                    "Не указано"
                } else {
                    &prof.company
                }
            );
            println!(
                "Bio / Описание:     {}",
                if prof.bio.is_empty() {
                    "Не указано"
                } else {
                    &prof.bio
                }
            );
            println!(
                "Сайт / Блог:        {}",
                if prof.blog.is_empty() {
                    "Не указано"
                } else {
                    &prof.blog
                }
            );
            println!("Ссылка на профиль:  {}", prof.html_url);
            println!("URL аватара:        {}", prof.avatar_url);
            println!("Статус в CRM:       HOT (+25 Lead Score, IT Lead)");
        } else {
            println!(
                "Публичный профиль разработчика для {} на GitHub не найден",
                email
            );
        }
        println!("============================================================");
        return;
    }

    if audit_campaign {
        println!("============================================================");
        println!("  Pre-flight Campaign Risk Audit (Folderly / Smartlead Style)");
        println!("  Файл базы: {}", output_json.display());
        println!("============================================================");
        let contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let (smtp_cfg, tmpl) = load_app_settings("settings.json").unwrap_or_else(|| {
            (
                poisk_contact::models::SmtpConfig::default(),
                poisk_contact::models::EmailTemplate::default(),
            )
        });
        let report =
            poisk_contact::mailer::calculate_campaign_preflight_risk(&contacts, &smtp_cfg, &tmpl);
        println!("Risk Score:           {}/100", report.risk_score);
        println!("Вердикт безопасности: {}", report.safety_grade);
        println!("Получателей всего:    {}", report.total_recipients);
        println!(
            "Catch-All адресов:    {} ({:.1}%)",
            report.catch_all_count, report.catch_all_percent
        );
        println!(
            "Общих ящиков (Role):  {} ({:.1}%)",
            report.role_count, report.role_percent
        );
        println!(
            "Одноразовых почт:     {} ({:.1}%)",
            report.disposable_count, report.disposable_percent
        );
        println!("Молодых доменов:      {}", report.young_domain_count);
        println!(
            "Стоп-слова в тексте:  {}",
            if report.spam_words_found.is_empty() {
                "Не обнаружены".to_string()
            } else {
                report.spam_words_found.join(", ")
            }
        );
        println!(
            "Персонализация:       {}",
            if report.missing_personalization {
                "Отсутствует ({name}/{company})"
            } else {
                "Присутствует"
            }
        );
        println!();
        println!("Рекомендации по безопасности рассылки:");
        for rec in &report.recommendations {
            println!("  - {}", rec);
        }
        println!("============================================================");
        return;
    }

    if let Some(target_url) = audit_security_target {
        println!("============================================================");
        println!("  B2B Web Security Posture & Enterprise Readiness Audit");
        println!("  Цель: {}", target_url);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        let report =
            poisk_contact::search::security_posture::audit_security_posture(&client, &target_url)
                .await;
        println!("URL:                   {}", report.url);
        println!("Security Score:        {}/100", report.score);
        println!("Security Grade:        {}", report.grade);
        println!(
            "Enterprise Ready:      {}",
            if report.is_enterprise_ready {
                "ДА (Соответствует корпоративным стандартам)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "HSTS (RFC 6797):       {} (max-age: {} сек)",
            if report.has_hsts {
                "ВКЛЮЧЕН"
            } else {
                "ОТСУТСТВУЕТ"
            },
            report.hsts_max_age
        );
        println!(
            "CSP (Content-Security):{}",
            if report.has_csp {
                "ВКЛЮЧЕН"
            } else {
                "ОТСУТСТВУЕТ"
            }
        );
        println!(
            "X-Frame-Options:       {}",
            if report.has_xfo {
                "ВКЛЮЧЕН (Защита от Clickjacking)"
            } else {
                "ОТСУТСТВУЕТ"
            }
        );
        println!(
            "X-Content-Type-Options:{}",
            if report.has_nosniff {
                "nosniff (Защита от MIME sniffing)"
            } else {
                "ОТСУТСТВУЕТ"
            }
        );
        println!(
            "Referrer-Policy:       {}",
            if report.has_referrer_policy {
                "НАСТРОЕНА"
            } else {
                "ОТСУТСТВУЕТ"
            }
        );
        if !report.recommendations.is_empty() {
            println!("\nРекомендации по усилению безопасности:");
            for r in &report.recommendations {
                println!("  - {}", r);
            }
        }
        println!("============================================================");
        return;
    }

    if verify_permutations {
        println!("============================================================");
        println!("  Hunter/Anymail Style Live Permutation & Catch-All Verification");
        println!(
            "  Имя: {} | Фамилия: {} | Домен: {}",
            perm_first, perm_last, perm_domain
        );
        println!("============================================================");
        if perm_first.is_empty() || perm_last.is_empty() || perm_domain.is_empty() {
            println!("Ошибка: укажите --first <Имя> --last <Фамилия> --domain <Домен>");
            return;
        }
        let res = poisk_contact::search::permutator::verify_candidate_permutations(
            &perm_first,
            &perm_last,
            &perm_domain,
            None,
        )
        .await;

        if res.is_catch_all {
            println!(
                "[ВНИМАНИЕ] Домен '{}' является Catch-All! Все адреса принимаются сервером.",
                perm_domain
            );
            println!("SMTP верификация отдельных ящиков не дает 100% гарантии отсутствия баунса.");
            if !res.best_guess.is_empty() {
                println!(
                    "Рекомендуемый наиболее вероятный шаблон: {}",
                    res.best_guess
                );
            }
        } else {
            println!("Catch-All статус: Отключен (сервер честно отклоняет несуществующие ящики).");
        }

        if let Some(v_email) = res.verified_email {
            println!("\n[УСПЕХ] Найден и подтвержден живой email: {}", v_email);
        } else {
            println!(
                "\n[РЕЗУЛЬТАТ] Ни один из пермутированных адресов не подтвердился через live SMTP probe."
            );
            if !res.best_guess.is_empty() {
                println!("Наиболее вероятный адрес по алгоритму: {}", res.best_guess);
            }
        }
        println!("Статус проверки: {}", res.status);
        println!("============================================================");
        return;
    }

    if bounce_forecast {
        println!("============================================================");
        println!("  Pre-flight Bounce Risk Forecast (NeverBounce / ZeroBounce Style)");
        println!("  Файл базы: {}", output_json.display());
        println!("============================================================");
        let contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let forecast = poisk_contact::mailer::calculate_bounce_risk_forecast(&contacts);
        println!("Контактов в базе:       {}", forecast.total_contacts);
        println!(
            "Прогнозируемый Bounce Rate: {:.1}%",
            forecast.predicted_bounce_rate
        );
        println!(
            "Прогноз отказов доставки:   {} шт.",
            forecast.predicted_bounces
        );
        println!("Вердикт безопасности:       {}", forecast.safety_verdict);
        println!("Безопасных контактов:       {}", forecast.safe_count);
        println!(
            "Умеренный риск (нужна проверка): {}",
            forecast.moderate_risk_count
        );
        println!(
            "Высокий риск (баунс/ошибки):     {}",
            forecast.high_risk_count
        );
        if !forecast.recommendations.is_empty() {
            println!("\nРекомендации:");
            for r in &forecast.recommendations {
                println!("  - {}", r);
            }
        }

        if prune_risky {
            println!("\nВыполняется удаление высокорисковых контактов (--prune)...");
            let (clean, pruned_count) = poisk_contact::mailer::prune_risky_contacts(&contacts);
            if pruned_count > 0 {
                let _ = save_contacts_to_json(&clean, &output_json);
                let _ = export_contacts_to_csv(&clean, &output_csv);
                println!(
                    "Удалено {} высокорисковых контактов. База успешно очищена!",
                    pruned_count
                );
            } else {
                println!("Высокорисковых контактов для удаления не обнаружено.");
            }
        } else if forecast.high_risk_count > 0 {
            println!(
                "\nПодсказка: запустите с флагом --prune для автоматической очистки базы перед рассылкой:"
            );
            println!("  .\\poisk_contact_cli.exe --bounce-forecast --prune");
        }
        println!("============================================================");
        return;
    }

    if warmup_matrix {
        println!("============================================================");
        println!("  Warmup Ramp-Up Matrix (Lemwarm / Instantly Warmup Engine)");
        println!(
            "  Целевой объем: {} писем/день | Темп: {}",
            target_volume, warmup_speed
        );
        println!("============================================================");
        let schedule = poisk_contact::mailer::generate_warmup_matrix(target_volume, &warmup_speed);
        println!(" День | Лимит писем | Суммарно отправлено | Задержка между письмами");
        println!("------+-------------+---------------------+-------------------------");
        for day_plan in &schedule {
            println!(
                "  #{:>2} | {:>5} писем | {:>11} писем  | {:>4} сек.",
                day_plan.day,
                day_plan.daily_limit,
                day_plan.cumulative_sent,
                day_plan.delay_between_emails_sec
            );
        }
        println!("============================================================");
        return;
    }

    if analyze_outreach {
        println!("============================================================");
        println!("  Cold Outreach Psychology & Readability Analysis (Lavender.ai Style)");
        println!("============================================================");
        let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
            (
                poisk_contact::models::SmtpConfig::default(),
                poisk_contact::models::EmailTemplate::default(),
            )
        });
        let rep = poisk_contact::search::readability::analyze_outreach_psychology(
            &template.subject,
            &template.body,
        );
        println!(
            "Тема письма:          \"{}\" ({} слов)",
            template.subject, rep.subject_words
        );
        println!(
            "Количество слов:      {} слов (Время чтения: ~{} сек)",
            rep.word_count, rep.reading_time_sec
        );
        println!("Предложений всего:    {}", rep.sentence_count);
        println!("Вопросов (CTA):       {} шт.", rep.question_count);
        println!("Соотношение Вы / Мы:  {:.1}x", rep.you_to_i_ratio);
        println!("Общий балл психологии:{}/100", rep.readability_score);
        println!("Итоговый грейд:       {}", rep.grade);
        if !rep.recommendations.is_empty() {
            println!("\nРекомендации Lavender по повышению конверсии:");
            for r in &rep.recommendations {
                println!("  - {}", r);
            }
        } else {
            println!("\nПисьмо идеально оптимизировано для холодного B2B аутрича!");
        }
        println!("============================================================");
        return;
    }

    if let Some(domain) = audit_lookalikes_target {
        println!("============================================================");
        println!("  Brand Impersonation & Typosquatting Scanner (dnstwist style)");
        println!("  Домен: {}", domain);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let verifier = poisk_contact::search::doh::MxVerifier::new();
        println!("Генерация перестановок и проверка DoH DNS (первые 15 вариаций)...");
        let results =
            poisk_contact::search::lookalike::audit_lookalikes(&client, &verifier, &domain, 15)
                .await;
        let mut reg_count = 0;
        for r in &results {
            if r.is_registered {
                reg_count += 1;
                println!(
                    "  [ЗАРЕГИСТРИРОВАН] {:<25} | {:<32} | MX: {}",
                    r.candidate_domain,
                    r.mutation_type,
                    if r.has_mx { &r.dns_record } else { "Нет MX" }
                );
            } else {
                println!(
                    "  [СВОБОДЕН]        {:<25} | {}",
                    r.candidate_domain, r.mutation_type
                );
            }
        }
        println!(
            "\nВсего проверено: {} вариаций. Зарегистрировано в DNS: {} шт.",
            results.len(),
            reg_count
        );
        println!("============================================================");
        return;
    }

    if let Some(target_dom) = audit_mx_target {
        println!("============================================================");
        println!("  MX Provider & Security Gateway Fingerprinting (CheckTLS / SEG style)");
        println!("  Домен или MX: {}", target_dom);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let verifier = poisk_contact::search::doh::MxVerifier::new();
        let (has_mx, mx_host) = verifier.check_domain(&client, &target_dom).await;
        let host_to_check = if has_mx { &mx_host } else { &target_dom };
        let fp = poisk_contact::search::mx_fingerprint::fingerprint_mx(host_to_check);
        println!("MX Хост:             {}", fp.host);
        println!("Провайдер шлюза:     {}", fp.provider_name);
        println!("Тип инфраструктуры:  {}", fp.gateway_type);
        println!("Уровень строгости:   {}", fp.strictness_level);
        println!("\nРекомендации по доставляемости в этот шлюз:");
        println!("  {}", fp.deliverability_advice);
        println!("============================================================");
        return;
    }

    if let Some(email) = classify_mailbox_target {
        println!("============================================================");
        println!("  Mailbox Tier & Disposable Email Classification (Kickbox style)");
        println!("  Email: {}", email);
        println!("============================================================");
        let (tier, desc) = poisk_contact::search::disposable::classify_mailbox_tier(&email);
        println!("Тип ящика:     {}", tier);
        println!("Описание:      {}", desc);
        println!(
            "Одноразовый:   {}",
            if tier == poisk_contact::models::MailboxTier::Disposable {
                "ДА (Риск спам-ловушки)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Категория B2B: {}",
            if tier == poisk_contact::models::MailboxTier::CorporateB2b {
                "ДА (Целевой корпоративный контакт)"
            } else {
                "НЕТ (Потребительский или временный)"
            }
        );
        println!("============================================================");
        return;
    }

    if let Some(email) = guard_trap_email {
        println!("============================================================");
        println!("  Spam Trap & Honey Pot Pattern Guard (ZeroBounce / Spamhaus Style)");
        println!("  Email: {}", email);
        println!("============================================================");
        let trap = poisk_contact::search::spamtrap_guard::audit_spam_trap(&email);
        println!(
            "Спам-ловушка (Trap):  {}",
            if trap.is_spam_trap {
                "ОБНАРУЖЕНА (Spam Trap Detected)"
            } else {
                "НЕТ (Чистый адрес)"
            }
        );
        println!("Категория ящика:      {}", trap.trap_category);
        println!("Уровень риска:        {}", trap.risk_level);
        println!("Рекомендация:         {}", trap.action_recommended);
        println!("============================================================");
        return;
    }

    if audit_spam {
        println!("============================================================");
        println!("  Spam Trigger & Stop-Words Audit (Mail-Tester / SpamAssassin)");
        println!("============================================================");
        let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
            (
                poisk_contact::models::SmtpConfig::default(),
                poisk_contact::models::EmailTemplate::default(),
            )
        });
        println!("Тема: {}\nТело:\n{}\n", template.subject, template.body);
        let rep = poisk_contact::search::spam_trigger::check_spam_triggers(
            &template.subject,
            &template.body,
        );
        println!("Spam Penalty Score:    {}/100", rep.spam_score);
        println!(
            "Уровень риска:         {}",
            if rep.is_high_risk {
                "ВЫСОКИЙ (Высокая вероятность попасть в Спам)"
            } else {
                "НИЗКИЙ (Безопасное письмо)"
            }
        );
        println!("Доля заглавных (CAPS): {:.1}%", rep.caps_ratio * 100.0);
        println!("Восклицательных знаков:{}", rep.exclamation_count);
        if !rep.trigger_words_found.is_empty() {
            println!(
                "Найденные стоп-слова:   {}",
                rep.trigger_words_found.join(", ")
            );
        }
        if !rep.advice.is_empty() {
            println!("\nРекомендации по очистке текста:");
            for a in &rep.advice {
                println!("  - {}", a);
            }
        } else {
            println!("\nСпам-триггеров не обнаружено. Письмо безопасно для отправки!");
        }
        println!("============================================================");
        return;
    }

    if audit_links {
        println!("============================================================");
        println!("  Link Density & Deliverability Audit (Mailmeteor / Woodpecker Style)");
        println!("============================================================");
        let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
            (
                poisk_contact::models::SmtpConfig::default(),
                poisk_contact::models::EmailTemplate::default(),
            )
        });
        println!("Тема письма:\n  {}\n", template.subject);
        println!("Тело письма:\n{}\n", template.body);
        let audit = poisk_contact::search::link_density::audit_email_links(&template.body);
        println!("Количество ссылок:      {}", audit.link_count);
        println!(
            "Плотность ссылок:       {:.1}% текста",
            audit.link_density_pct
        );
        println!(
            "Подозрительные редиректы:{}",
            if audit.has_suspicious_redirects {
                "ОБНАРУЖЕНЫ (bit.ly, clck.ru, tinyurl.com и др.)"
            } else {
                "Не обнаружены"
            }
        );
        println!(
            "Готовность к инбоксу:   {}",
            if audit.is_safe_for_inbox {
                "БЕЗОПАСНО (Inbox Safe: оптимальный баланс текста и ссылок)"
            } else {
                "РИСК СПАМА (High Spam Penalty Risk)"
            }
        );
        if !audit.advice.is_empty() {
            println!("\nРекомендации по доставляемости:");
            for a in &audit.advice {
                println!("  - {}", a);
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(subj) = optimize_subject_target {
        println!("============================================================");
        println!("  Mobile Subject Line Truncation & A/B Optimizer (Instantly Style)");
        println!("  Исходная тема: \"{}\"", subj);
        println!("============================================================");
        let opt = poisk_contact::search::subject_optimizer::optimize_subject_line(&subj);
        println!(
            "Количество символов:    {} (Лимит экранов смартфонов: <= 40)",
            opt.char_count
        );
        println!(
            "Количество слов:        {} слов (Идеал: 2-5 слов)",
            opt.word_count
        );
        println!(
            "Мобильная адаптивность: {}",
            if opt.is_mobile_friendly {
                "ИДЕАЛЬНО (Mobile Friendly: помещается целиком на iOS/Android)"
            } else {
                "ТРЕБУЕТ ДОРАБОТКИ (Высокий риск обрезки или спама)"
            }
        );
        println!(
            "Маркетинговые стоп-слова:{}",
            if opt.has_spam_words {
                "ОБНАРУЖЕНЫ (Повышенный спам-скор)"
            } else {
                "Не обнаружены"
            }
        );
        println!("Рекомендуемый A/B тест: \"{}\"", opt.lowercase_variant);
        println!("\nАнализ и рекомендации:");
        for a in &opt.advice {
            println!("  * {}", a);
        }
        println!("============================================================");
        return;
    }

    if let Some(target_url) = detect_commerce_target {
        println!("============================================================");
        println!("  E-Commerce & Payment Gateway Fingerprinting (BuiltWith Style)");
        println!("  Сайт: {}", target_url);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        let html = if let Ok(resp) = client
            .get(&target_url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
        {
            resp.text().await.unwrap_or_default()
        } else {
            String::new()
        };
        let stack = poisk_contact::search::ecommerce_detect::detect_ecommerce_stack(&html);
        println!(
            "Коммерческий статус:  {}",
            if stack.is_commercial {
                "E-Commerce / Онлайн-продажи активны"
            } else {
                "Контентный или сервисный сайт"
            }
        );
        println!(
            "Платежные системы:    {}",
            if stack.payment_gateways.is_empty() {
                "Не обнаружены".to_string()
            } else {
                stack.payment_gateways.join(", ")
            }
        );
        println!(
            "Онлайн-чаты/виджеты:  {}",
            if stack.live_chats.is_empty() {
                "Не обнаружены".to_string()
            } else {
                stack.live_chats.join(", ")
            }
        );
        println!(
            "Платформа магазина:   {}",
            if stack.store_engines.is_empty() {
                "Не обнаружена".to_string()
            } else {
                stack.store_engines.join(", ")
            }
        );
        println!("============================================================");
        return;
    }

    if let Some(title) = classify_title_target {
        println!("============================================================");
        println!("  Decision-Maker Seniority & Department Classifier (Apollo Style)");
        println!("  Должность: {}", title);
        println!("============================================================");
        let (level, dept) = poisk_contact::search::seniority::classify_title(&title);
        println!("Грейд / Уровень:   {}", level.as_str());
        println!("Департамент / Роль:{}", dept.as_str());
        println!("============================================================");
        return;
    }

    if let Some(domain) = prospect_domain_target {
        println!("============================================================");
        println!("  Domain Search & Team Roster Harvester (Snov.io Style)");
        println!("  Домен: {}", domain);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        println!(
            "Сканирование корпоративных роутов команды (/team, /about, /contacts, /leadership)..."
        );
        let prospects =
            poisk_contact::search::domain_prospector::prospect_domain_team(&client, &domain, 25)
                .await;
        println!("Найдено сотрудников домена: {}", prospects.len());
        for (idx, p) in prospects.iter().enumerate() {
            println!(
                "  [{:>2}] {:<24} | {:<30} | {:<20} | {}",
                idx + 1,
                p.name,
                p.email,
                p.seniority,
                p.department
            );
        }
        if !prospects.is_empty() {
            let mut existing = load_contacts_from_json(&output_json).unwrap_or_default();
            existing.extend(prospects);
            let (merged, _) =
                poisk_contact::search::waterfall_merge::deduplicate_and_merge(existing);
            let _ = save_contacts_to_json(&merged, &output_json);
            let _ = export_contacts_to_csv(&merged, &output_csv);
            println!(
                "\nКонтакты объединены и сохранены в {}",
                output_json.display()
            );
        }
        println!("============================================================");
        return;
    }

    if let Some(target_url) = detect_intent_target {
        println!("============================================================");
        println!("  Ad Pixel & Marketing Automation Intent Scoring (BuiltWith AdTech)");
        println!("  Сайт: {}", target_url);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        let html = if let Ok(resp) = client
            .get(&target_url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
        {
            resp.text().await.unwrap_or_default()
        } else {
            String::new()
        };
        let prof = poisk_contact::search::intent_signals::detect_marketing_intent(&html);
        println!("B2B Intent Score:      {}/100", prof.intent_score);
        println!(
            "Готовность к закупкам: {}",
            if prof.is_high_intent {
                "ВЫСОКАЯ (High-Intent B2B Lead: активные бюджеты на рекламу/ESP)"
            } else {
                "СТАНДАРТНАЯ / НИЗКАЯ"
            }
        );
        println!(
            "Рекламные пиксели:     {}",
            if prof.ad_pixels.is_empty() {
                "Не обнаружены".to_string()
            } else {
                prof.ad_pixels.join(", ")
            }
        );
        println!(
            "Системы автоматизации: {}",
            if prof.automation_platforms.is_empty() {
                "Не обнаружены".to_string()
            } else {
                prof.automation_platforms.join(", ")
            }
        );
        println!("============================================================");
        return;
    }

    if let Some(domain) = recon_mail_infra_target {
        println!("============================================================");
        println!("  Mail Infrastructure Subdomain Intelligence (theHarvester Style)");
        println!("  Домен: {}", domain);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        println!(
            "DoH аудит скрытых почтовых поддоменов (mail., smtp., webmail., autodiscover.)..."
        );
        let subs =
            poisk_contact::search::mail_recon::recon_mail_infrastructure(&client, &domain).await;
        println!("Обнаружено активных почтовых узлов: {}", subs.len());
        for (idx, s) in subs.iter().enumerate() {
            println!(
                "  [{:>2}] {:<30} -> {:<18} | {}",
                idx + 1,
                s.subdomain,
                s.resolved_record,
                s.service_type
            );
        }
        println!("============================================================");
        return;
    }

    if let Some(target_email) = generate_unsub_target {
        println!("============================================================");
        println!("  One-Click List-Unsubscribe RFC 8058 Header Generator");
        println!("  Получатель: {}", target_email);
        println!("============================================================");
        let (active_cfg, _) = load_app_settings("settings.json").unwrap_or_else(|| {
            (
                poisk_contact::models::SmtpConfig::default(),
                poisk_contact::models::EmailTemplate::default(),
            )
        });
        let sender_domain =
            if !active_cfg.from_email.is_empty() && active_cfg.from_email.contains('@') {
                active_cfg
                    .from_email
                    .split('@')
                    .nth(1)
                    .unwrap_or("example.com")
            } else {
                "example.com"
            };
        let (unsub, post) = poisk_contact::mailer::generate_rfc8058_unsubscribe_headers(
            sender_domain,
            &target_email,
        );
        println!("Отправитель (домен):   {}", sender_domain);
        println!("List-Unsubscribe:      {}", unsub);
        println!("List-Unsubscribe-Post: {}", post);
        println!(
            "\nСоответствие RFC 8058 & RFC 2369: ДА (Требование Google Workspace & Yahoo Mail 2024-2026)"
        );
        println!("============================================================");
        return;
    }

    if let Some(raw_phone) = audit_voip_target {
        println!("============================================================");
        println!("  VoIP / Cloud PBX vs Real Mobile Phone Classifier");
        println!("  Номер: {}", raw_phone);
        println!("============================================================");
        let voip = poisk_contact::search::voip_detect::classify_phone_line(&raw_phone);
        println!("E.164 Номер:            {}", voip.phone_number);
        println!(
            "Виртуальная АТС (VoIP): {}",
            if voip.is_voip_cloud_pbx {
                "ДА (Облачная АТС / SIP транк)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Физический мобильный:   {}",
            if voip.is_real_mobile {
                "ДА (Личный мобильный номер ЛПР)"
            } else {
                "НЕТ"
            }
        );
        println!("Категория оператора:    {}", voip.carrier_category);
        println!("Описание:               {}", voip.description);
        println!("============================================================");
        return;
    }

    if audit_links {
        println!("============================================================");
        println!("  Email Link Density & Anti-Phishing Deliverability Audit");
        println!("============================================================");
        let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
            (
                poisk_contact::models::SmtpConfig::default(),
                poisk_contact::models::EmailTemplate::default(),
            )
        });
        let rep = poisk_contact::search::link_density::audit_email_links(&template.body);
        println!("Тема письма:           {}", template.subject);
        println!("Число ссылок:          {}", rep.link_count);
        println!("Плотность ссылок:      {:.1}%", rep.link_density_pct);
        println!(
            "Подозрительные редиректы: {}",
            if rep.has_suspicious_redirects {
                "ОБНАРУЖЕНЫ (bit.ly, clck.ru)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Безопасно для инбокса: {}",
            if rep.is_safe_for_inbox {
                "ДА (Высокая вероятность доставки в Inbox)"
            } else {
                "ВНИМАНИЕ (Риск попадания в спам)"
            }
        );
        if !rep.advice.is_empty() {
            println!("\nРекомендации:");
            for a in &rep.advice {
                println!("  - {}", a);
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(target_email) = score_catchall_email {
        println!("============================================================");
        println!("  Catch-All Deliverability Confidence Scorer (NeverBounce Style)");
        println!("  Email: {}", target_email);
        println!("============================================================");
        let mx_prov = score_catchall_mx.unwrap_or_else(|| "Unknown".to_string());
        let (conf, verdict) = poisk_contact::search::catchall_scorer::calculate_catchall_confidence(
            &target_email,
            &mx_prov,
        );
        println!("Почтовый провайдер:    {}", mx_prov);
        println!("Catch-All Доверие:     {}/100", conf);
        println!("Вердикт доставляемости: {}", verdict);
        println!("============================================================");
        return;
    }

    if let Some(target) = tax_audit_target {
        println!("============================================================");
        println!("  Tax & Legal Entity Verification (Rusprofile / VIES Style)");
        println!("  Цель: {}", target);
        println!("============================================================");
        let text = if target.starts_with("http://") || target.starts_with("https://") {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(6))
                .build()
                .unwrap_or_default();
            if let Ok(resp) = client
                .get(&target)
                .header("User-Agent", "Mozilla/5.0")
                .send()
                .await
            {
                resp.text().await.unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            target
        };
        let prof = poisk_contact::search::tax_id::extract_tax_identifiers(&text);
        println!(
            "ИНН:                {}",
            if prof.inn.is_empty() {
                "Не найден"
            } else {
                &prof.inn
            }
        );
        println!(
            "Валидность ИНН:     {}",
            if prof.is_valid_inn {
                "ВАЛИДЕН (Контрольная сумма верна)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "ОГРН:               {}",
            if prof.ogrn.is_empty() {
                "Не найден"
            } else {
                &prof.ogrn
            }
        );
        println!(
            "КПП:                {}",
            if prof.kpp.is_empty() {
                "Не найден"
            } else {
                &prof.kpp
            }
        );
        println!(
            "Европейский VAT:    {}",
            if prof.vat_id.is_empty() {
                "Не найден"
            } else {
                &prof.vat_id
            }
        );
        println!(
            "Тип субъекта:       {}",
            if prof.entity_type.is_empty() {
                "Не определен"
            } else {
                &prof.entity_type
            }
        );
        if !prof.registry_url.is_empty() {
            println!("Ссылка на реестр:   {}", prof.registry_url);
        }
        println!("============================================================");
        return;
    }

    if let Some(msg) = test_alert_msg {
        println!("============================================================");
        println!("  Тестовое оповещение (Telegram Bot & Webhook)");
        println!("  Сообщение: {}", msg);
        println!("============================================================");
        if let Ok(content) = std::fs::read_to_string("settings.json") {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let bot_token = val
                    .get("telegram_bot_token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let chat_id = val
                    .get("telegram_chat_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let webhook_url = val
                    .get("webhook_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if !bot_token.is_empty() && !chat_id.is_empty() {
                    match poisk_contact::mailer::send_telegram_lead_alert(bot_token, chat_id, &msg)
                        .await
                    {
                        Ok(_) => println!("Telegram alert отправлен успешно!"),
                        Err(e) => println!("Ошибка отправки Telegram: {}", e),
                    }
                } else {
                    println!(
                        "telegram_bot_token или telegram_chat_id не настроены в settings.json"
                    );
                }

                if !webhook_url.is_empty() {
                    let payload = serde_json::json!({
                        "event": "test_alert",
                        "message": msg,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    match poisk_contact::mailer::send_lead_webhook(webhook_url, &payload).await {
                        Ok(_) => println!("Webhook отправлен успешно!"),
                        Err(e) => println!("Ошибка отправки Webhook: {}", e),
                    }
                }
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(target_dom) = audit_mta_sts_target {
        println!("============================================================");
        println!("  MTA-STS & TLS-RPT Transport Security Audit (RFC 8461/8460)");
        println!("  Домен: {}", target_dom);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let verifier = poisk_contact::search::doh::MxVerifier::new();
        let audit = verifier.audit_mta_sts(&client, &target_dom).await;
        println!(
            "DNS TXT запись:     {}",
            if audit.dns_txt_valid {
                &audit.txt_record
            } else {
                "НЕ НАЙДЕНА (_mta-sts)"
            }
        );
        println!(
            "Файл политики:      {}",
            if audit.policy_fetched {
                "ДОСТУПЕН (.well-known/mta-sts.txt)"
            } else {
                "НЕ НАЙДЕН"
            }
        );
        println!(
            "Режим защиты (mode):{}",
            if audit.mode.is_empty() {
                "Не определен"
            } else {
                &audit.mode
            }
        );
        println!("Период кэша max-age:{} сек", audit.max_age);
        println!(
            "Разрешенные MX:     {}",
            if audit.mx_patterns.is_empty() {
                "Нет".to_string()
            } else {
                audit.mx_patterns.join(", ")
            }
        );
        println!(
            "TLS-RPT отчетность: {}",
            if audit.has_tls_rpt {
                &audit.tls_rpt_record
            } else {
                "НЕ НАЙДЕНА (_smtp._tls)"
            }
        );
        if !audit.recommendations.is_empty() {
            println!("\nРекомендации:");
            for r in &audit.recommendations {
                println!("  - {}", r);
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(target_dom) = audit_bimi_target {
        println!("============================================================");
        println!("  BIMI Brand Indicators & VMC Certificate Audit");
        println!("  Домен: {}", target_dom);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let verifier = poisk_contact::search::doh::MxVerifier::new();
        let audit = verifier.audit_bimi(&client, &target_dom).await;
        println!(
            "BIMI TXT запись:    {}",
            if audit.valid_bimi {
                &audit.raw_record
            } else {
                "НЕ НАЙДЕНА (default._bimi)"
            }
        );
        println!(
            "SVG логотип (l):    {}",
            if audit.svg_url.is_empty() {
                "Не указан"
            } else {
                &audit.svg_url
            }
        );
        println!(
            "VMC сертификат (a): {}",
            if audit.vmc_cert_url.is_empty() {
                "Не указан"
            } else {
                &audit.vmc_cert_url
            }
        );
        println!(
            "Синяя галочка Gmail:{}",
            if audit.has_vmc {
                "ПОДДЕРЖИВАЕТСЯ (VMC сертифицирован)"
            } else {
                "НЕТ (Требуется VMC сертификат)"
            }
        );
        if !audit.recommendations.is_empty() {
            println!("\nРекомендации:");
            for r in &audit.recommendations {
                println!("  - {}", r);
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(target_url) = audit_compliance_target {
        println!("============================================================");
        println!("  Legal & Privacy Compliance Audit (152-ФЗ РФ & GDPR ЕС)");
        println!("  Цель: {}", target_url);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let profile =
            poisk_contact::search::compliance::audit_website_compliance(&client, &target_url).await;
        println!(
            "Политика найдена:   {}",
            if profile.has_privacy_policy {
                "ДА"
            } else {
                "НЕТ"
            }
        );
        println!(
            "URL политики:       {}",
            if profile.privacy_policy_url.is_empty() {
                "Не найдена"
            } else {
                &profile.privacy_policy_url
            }
        );
        println!(
            "152-ФЗ РФ:          {}",
            if profile.law_152_fz_compliant {
                "СООТВЕТСТВУЕТ (Маркеры оператора и согласия найдены)"
            } else {
                "НЕТ / Риск"
            }
        );
        println!(
            "GDPR ЕС:            {}",
            if profile.gdpr_compliant {
                "СООТВЕТСТВУЕТ (GDPR / DPO маркеры найдены)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Уведомление Cookie: {}",
            if profile.has_cookie_notice {
                "ДА"
            } else {
                "НЕТ"
            }
        );
        println!("Оценка комплаенса:  {}/100", profile.compliance_score);
        println!("Статус безопасности:{}", profile.compliance_status);
        if !profile.detected_markers.is_empty() {
            println!(
                "Найденные маркеры:  {}",
                profile.detected_markers.join(", ")
            );
        }
        if !profile.recommendations.is_empty() {
            println!("\nРекомендации:");
            for r in &profile.recommendations {
                println!("  - {}", r);
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(target_dom) = shadow_recon_target {
        println!("============================================================");
        println!("  Shadow Subdomain Recon & Contact Harvesting");
        println!("  Домен: {}", target_dom);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let recon = poisk_contact::search::recon::run_domain_recon(&client, &target_dom).await;
        println!(
            "Найдено поддоменов в Certificate Transparency: {}",
            recon.subdomains.len()
        );
        for sub in &recon.subdomains {
            println!("  - {}", sub);
        }
        println!("\nОпрос операционных поддоменов на скрытые контакты отделов...");
        let contacts = poisk_contact::search::recon::probe_shadow_subdomain_contacts(
            &client,
            &target_dom,
            &recon.subdomains,
        )
        .await;
        if contacts.is_empty() {
            println!("Прямых контактов на найденных поддоменах не обнаружено.");
        } else {
            println!(
                "Обнаружено скрытых корпоративных контактов: {}",
                contacts.len()
            );
            for c in &contacts {
                println!(
                    "  - {} | {} | {} | тел: {}",
                    c.email, c.role, c.url, c.phone
                );
            }
            let mut existing = load_contacts_from_json(&output_json).unwrap_or_default();
            existing.extend(contacts);
            let (merged, _) =
                poisk_contact::search::waterfall_merge::deduplicate_and_merge(existing);
            let _ = save_contacts_to_json(&merged, &output_json);
            let _ = export_contacts_to_csv(&merged, &output_csv);
            println!(
                "Контакты объединены и сохранены в {}",
                output_json.display()
            );
        }
        println!("============================================================");
        return;
    }

    if let Some(crm_fmt) = export_crm_target {
        let contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let fmt_lower = crm_fmt.trim().to_lowercase();
        println!("============================================================");
        println!("  CRM Ready Direct CSV Export");
        println!("  Формат: {} | Записей: {}", fmt_lower, contacts.len());
        println!("============================================================");
        if fmt_lower.contains("amo") {
            let path = std::path::PathBuf::from("contacts_amocrm.csv");
            match poisk_contact::storage::export_contacts_to_amocrm_csv(&contacts, &path) {
                Ok(_) => println!("Успешно экспортировано в AmoCRM CSV: {}", path.display()),
                Err(e) => println!("Ошибка экспорта: {}", e),
            }
        } else if fmt_lower.contains("bitrix") {
            let path = std::path::PathBuf::from("contacts_bitrix24.csv");
            match poisk_contact::storage::export_contacts_to_bitrix24_csv(&contacts, &path) {
                Ok(_) => println!("Успешно экспортировано в Bitrix24 CSV: {}", path.display()),
                Err(e) => println!("Ошибка экспорта: {}", e),
            }
        } else if fmt_lower.contains("instantly") {
            let path = std::path::PathBuf::from("contacts_instantly.csv");
            match poisk_contact::storage::export_contacts_to_instantly_csv(&contacts, &path) {
                Ok(_) => println!(
                    "Успешно экспортировано в Instantly.ai CSV: {}",
                    path.display()
                ),
                Err(e) => println!("Ошибка экспорта: {}", e),
            }
        } else if fmt_lower.contains("lemlist") {
            let path = std::path::PathBuf::from("contacts_lemlist.csv");
            match poisk_contact::storage::export_contacts_to_lemlist_csv(&contacts, &path) {
                Ok(_) => println!("Успешно экспортировано в Lemlist CSV: {}", path.display()),
                Err(e) => println!("Ошибка экспорта: {}", e),
            }
        } else {
            println!(
                "Неизвестный формат CRM/аутрича: '{}'. Доступные форматы: 'amocrm', 'bitrix24', 'instantly', 'lemlist'.",
                crm_fmt
            );
        }
        println!("============================================================");
        return;
    }

    if let Some(domain) = probe_well_known_target {
        println!("============================================================");
        println!("  RFC 9116 security.txt & humans.txt Contact Harvester");
        println!("  Домен: {}", domain);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        match poisk_contact::search::well_known::probe_well_known(&client, &domain).await {
            Ok(res) => {
                println!("Найдено прямых контактов команды: {}", res.records.len());
                for r in &res.records {
                    println!("  - [{}] {} <{}> ({})", r.role, r.name, r.email, r.url);
                }
                if !res.hiring_links.is_empty() {
                    println!("Ссылки на найм / Hiring:");
                    for h in &res.hiring_links {
                        println!("  - {}", h);
                    }
                }
                if !res.records.is_empty() {
                    let mut existing = load_contacts_from_json(&output_json).unwrap_or_default();
                    existing.extend(res.records);
                    let (merged, _) =
                        poisk_contact::search::waterfall_merge::deduplicate_and_merge(existing);
                    let _ = save_contacts_to_json(&merged, &output_json);
                    let _ = export_contacts_to_csv(&merged, &output_csv);
                    println!("База обновлена и сохранена в {}", output_json.display());
                }
            }
            Err(e) => println!("Ошибка: {}", e),
        }
        println!("============================================================");
        return;
    }

    if let Some(domain) = mine_pgp_target {
        println!("============================================================");
        println!("  Passive Ubuntu PGP Keyserver Miner (HKP Protocol)");
        println!("  Домен: {}", domain);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        match poisk_contact::search::keyserver::mine_pgp_keyserver(&client, &domain, 50).await {
            Ok(records) => {
                println!("Найдено PGP ключей сотрудников: {}", records.len());
                for r in &records {
                    println!("  - {} <{}>", r.name, r.email);
                }
                if !records.is_empty() {
                    let mut existing = load_contacts_from_json(&output_json).unwrap_or_default();
                    existing.extend(records);
                    let (merged, _) =
                        poisk_contact::search::waterfall_merge::deduplicate_and_merge(existing);
                    let _ = save_contacts_to_json(&merged, &output_json);
                    let _ = export_contacts_to_csv(&merged, &output_csv);
                    println!("База обновлена и сохранена в {}", output_json.display());
                }
            }
            Err(e) => println!("Ошибка: {}", e),
        }
        println!("============================================================");
        return;
    }

    if let Some(target_email) = probe_smtp_target {
        println!("============================================================");
        println!("  Non-Intrusive SMTP Mailbox Deliverability Handshake (Port 25)");
        println!("  Target: {}", target_email);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let mx_verifier = poisk_contact::search::doh::MxVerifier::new();
        let (_, mx_rec) = mx_verifier.check_email(&client, &target_email).await;
        if mx_rec.is_empty() {
            println!("[ERROR] No MX records resolved for {}", target_email);
        } else {
            println!("Resolved MX Host: {}", mx_rec);
            let res = poisk_contact::search::smtp_probe::probe_email_deliverability(
                &target_email,
                &mx_rec,
                5,
            )
            .await;
            println!("Deliverability:   {:?}", res.deliverability);
            println!("SMTP Status Code: {}", res.status_code);
            println!("Server Response:  {}", res.raw_response);
        }
        println!("============================================================");
        return;
    }

    if smart_enrich {
        println!("============================================================");
        println!("  Smart Waterfall Lead Enrichment Pipeline (Clay / Apollo Style)");
        println!("  Файл базы: {}", output_json.display());
        println!("============================================================");
        let contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        println!("Шаг 1/5: Waterfall DSU дедупликация и объединение разрозненных профилей...");
        let (mut merged, stats) =
            poisk_contact::search::waterfall_merge::deduplicate_and_merge(contacts);
        println!(
            "  - Объединено дубликатов: {}, обогащено полей: {}",
            stats.duplicates_merged, stats.fields_enriched
        );

        println!("Шаг 2/5: Телеком-аналитика телефонов (нормализация E.164, оператор, регион)...");
        let mut phone_count = 0;
        for c in &mut merged {
            if !c.phone.is_empty() {
                let p = poisk_contact::search::phone_intel::parse_and_enrich_phone(&c.phone);
                poisk_contact::search::phone_intel::apply_phone_intel_to_contact(&p, c);
                poisk_contact::search::voip_detect::apply_voip_to_contact(c);
                phone_count += 1;
            }
        }
        println!("  - Обогащено телефонных номеров: {}", phone_count);

        println!("Шаг 3/5: Юридическая верификация (ИНН, ОГРН, VAT)...");
        let mut tax_count = 0;
        for c in &mut merged {
            let hay = format!("{} {} {}", c.company, c.legal_name, c.company_description);
            let tax_prof = poisk_contact::search::tax_id::extract_tax_identifiers(&hay);
            if !tax_prof.inn.is_empty() {
                poisk_contact::search::tax_id::apply_tax_profile_to_contact(&tax_prof, c);
                tax_count += 1;
            }
        }
        println!("  - Найдено и верифицировано реквизитов: {}", tax_count);

        println!(
            "Шаг 4/6: Анализ грейда ЛПР (Apollo.io) и классификация почтового ящика (Kickbox)..."
        );
        for c in &mut merged {
            poisk_contact::search::seniority::apply_seniority_to_contact(c);
            poisk_contact::search::disposable::apply_mailbox_tier_to_contact(c);
        }

        println!("Шаг 5/6: Генерация персонализированных динамических айсбрейкеров (Clay.com)...");
        for c in &mut merged {
            if c.icebreaker.is_empty() {
                c.icebreaker = poisk_contact::search::smart_hooks::generate_smart_hook(c);
            }
        }

        println!("Шаг 6/6: Пересчет Lead Scoring (Hot/Warm/Cold, 0..100)...");
        for c in &mut merged {
            let mut score = c.lead_score;
            if c.hiring_active {
                score = score.saturating_add(20);
            }
            if !c.inn.is_empty() {
                score = score.saturating_add(15);
            }
            if !c.phone_carrier.is_empty() {
                score = score.saturating_add(10);
            }
            if c.compliance_score >= 70 {
                score = score.saturating_add(10);
            }
            c.lead_score = score.min(100);
            if c.lead_score >= 80 {
                c.score = poisk_contact::models::LeadScore::Hot;
            } else if c.lead_score >= 50 {
                c.score = poisk_contact::models::LeadScore::Warm;
            } else {
                c.score = poisk_contact::models::LeadScore::Cold;
            }
        }

        let _ = save_contacts_to_json(&merged, &output_json);
        let _ = export_contacts_to_csv(&merged, &output_csv);
        println!(
            "\nСквозное обогащение завершено! Обработано мастер-записей: {}",
            merged.len()
        );
        println!(
            "База сохранена в {} и {}",
            output_json.display(),
            output_csv.display()
        );
        println!("============================================================");
        return;
    }

    if dork_ats {
        println!("============================================================");
        println!("  DuckDuckGo Deep ATS Dorking — Unindexed Vacancies");
        println!("  Persona: {} | Limit: {}", dork_persona, max_items);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .unwrap_or_default();
        match poisk_contact::search::web::dork_ats_vacancies(&client, &dork_persona, max_items)
            .await
        {
            Ok(urls) => {
                println!("Discovered {} unindexed ATS vacancies:", urls.len());
                for (idx, u) in urls.iter().enumerate() {
                    println!("  [{:>2}] {}", idx + 1, u);
                }
            }
            Err(e) => eprintln!("[ERROR] Dorking failed: {}", e),
        }
        return;
    }

    if export_vcf {
        let contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let vcf_path = output_json.with_extension("vcf");
        match export_contacts_to_vcf(&contacts, &vcf_path) {
            Ok(_) => println!(
                "[EXPORT] Экспортировано {} контактов в vCard: {}",
                contacts.len(),
                vcf_path.display()
            ),
            Err(e) => eprintln!("[ERROR] Ошибка экспорта vCard: {}", e),
        }
        return;
    }

    if export_csv {
        let contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        match export_contacts_to_csv(&contacts, &output_csv) {
            Ok(_) => println!(
                "[EXPORT] Экспортировано {} контактов в CSV: {}",
                contacts.len(),
                output_csv.display()
            ),
            Err(e) => eprintln!("[ERROR] Ошибка экспорта CSV: {}", e),
        }
        return;
    }

    if let Some(url) = detect_tech_url {
        println!("============================================================");
        println!("  Tech-Stack & Analytics Fingerprinting: {}", url);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        match client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(resp) => {
                let headers_str = format!("{:?}", resp.headers());
                if let Ok(body_text) = resp.text().await {
                    let tech = poisk_contact::search::tech_detect::detect_tech_stack(
                        &body_text,
                        &headers_str,
                    );
                    println!(
                        "CMS / Фреймворк: {}",
                        tech.cms.as_deref().unwrap_or("Не определена")
                    );
                    println!(
                        "CRM Система:     {}",
                        tech.crm.as_deref().unwrap_or("Не обнаружена")
                    );
                    println!(
                        "Онлайн-чат:      {}",
                        tech.live_chat.as_deref().unwrap_or("Не обнаружен")
                    );
                    if !tech.analytics.is_empty() {
                        println!("Системы аналитики (Digital Footprint):");
                        for a in &tech.analytics {
                            println!("  * {}", a);
                        }
                    }
                    if !tech.all_tags.is_empty() {
                        println!("Все определенные теги: {}", tech.all_tags.join(", "));
                    }
                } else {
                    eprintln!("[ERROR] Не удалось прочитать HTML страницу");
                }
            }
            Err(e) => eprintln!("[ERROR] Ошибка запроса к сайту: {}", e),
        }
        return;
    }

    if let Some(url) = detect_cloud_url {
        println!("============================================================");
        println!("  Cloud Infrastructure & CDN Fingerprinting (Clay / Datanyze Style)");
        println!("  Цель: {}", url);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        match client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(resp) => {
                let headers_str = format!("{:?}", resp.headers());
                let body_text = resp.text().await.unwrap_or_default();
                let prof = poisk_contact::search::cloud_infra::detect_cloud_infra(
                    &headers_str,
                    &body_text,
                );
                println!("Облачный провайдер:  {}", prof.provider);
                println!(
                    "Хостинг в облаке:    {}",
                    if prof.is_cloud_hosted {
                        "ДА (Cloud Native / Edge)"
                    } else {
                        "Dedicated / On-Premise"
                    }
                );
                println!(
                    "Сеть доставки (CDN): {}",
                    prof.cdn.as_deref().unwrap_or("Не обнаружена")
                );
                if !prof.detected_markers.is_empty() {
                    println!("Найденные маркеры:");
                    for m in &prof.detected_markers {
                        println!("  * {}", m);
                    }
                }
            }
            Err(e) => eprintln!("[ERROR] Ошибка запроса к {}: {}", url, e),
        }
        println!("============================================================");
        return;
    }

    if let Some(domain) = recon_domain_target {
        println!("============================================================");
        println!("  Комплексный OSINT-аудит домена: {}", domain);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(7))
            .build()
            .unwrap_or_default();

        println!("[1/2] Запрос ICANN RDAP (возраст домена и регистратор)...");
        match poisk_contact::search::rdap::lookup_rdap_domain(&client, &domain).await {
            Ok(rdap) => {
                println!(
                    "Регистратор:    {}",
                    rdap.registrar.as_deref().unwrap_or("Не указан")
                );
                println!(
                    "Дата создания:  {}",
                    rdap.registered_at.as_deref().unwrap_or("Не указана")
                );
                if let Some(days) = rdap.age_days {
                    let yrs = rdap.age_years.unwrap_or(0.0);
                    println!("Возраст домена: {} дней (~{:.1} лет)", days, yrs);
                    if rdap.is_young_domain {
                        println!(
                            "Статус риска:   [ВНИМАНИЕ] Молодой домен (< 60 дней) - повышенный риск спама"
                        );
                    } else if rdap.is_mature_domain {
                        println!(
                            "Статус риска:   [НАДЕЖНЫЙ] Зрелый домен (> 3 лет) - высокая репутация"
                        );
                    }
                }
            }
            Err(e) => println!("RDAP ошибка: {}", e),
        }

        println!(
            "\n[2/2] Поиск скрытых контактов (security.txt, humans.txt) и поддоменов crt.sh..."
        );
        let recon = poisk_contact::search::recon::run_domain_recon(&client, &domain).await;
        if !recon.security_contacts.is_empty() {
            println!("Контакты безопасности (security.txt RFC 9116):");
            for c in &recon.security_contacts {
                println!("  * {}", c);
            }
        } else {
            println!("security.txt: контактов не обнаружено.");
        }
        if !recon.human_contacts.is_empty() {
            println!("Контакты авторов (humans.txt):");
            for c in &recon.human_contacts {
                println!("  * {}", c);
            }
        }
        if !recon.subdomains.is_empty() {
            println!("Обнаруженные корпоративные поддомены (crt.sh):");
            for s in &recon.subdomains {
                println!("  * {}", s);
            }
        } else {
            println!("Интересных почтовых/crm поддоменов в логах CT не найдено.");
        }
        return;
    }

    if let Some(domain) = domain_pattern_target {
        println!("============================================================");
        println!(
            "  Анализ паттерна корпоративных email (Hunter.io style): {}",
            domain
        );
        println!("============================================================");
        let contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let domain_clean = domain.trim().trim_start_matches('@').to_lowercase();
        let domain_tuples: Vec<(String, String, String)> = contacts
            .iter()
            .filter(|c| {
                c.email
                    .to_lowercase()
                    .ends_with(&format!("@{}", domain_clean))
            })
            .map(|c| (c.name.clone(), c.email.clone(), domain_clean.clone()))
            .collect();

        if domain_tuples.is_empty() {
            println!(
                "В базе данных нет сохраненных контактов для домена '{}'.",
                domain_clean
            );
        } else {
            match poisk_contact::search::domain_pattern::analyze_domain_patterns(&domain_tuples) {
                Some(p_info) => {
                    println!("Доминантный паттерн:  {}", p_info.dominant_pattern);
                    println!("Уверенность:          {}%", p_info.confidence_percent);
                    println!("Выборка контактов:    {} шт.", p_info.sample_count);
                    println!("\nДетализация совпадений:");
                    for (pat, count) in &p_info.breakdown {
                        println!("  * {:<16} : {} адресов", pat, count);
                    }
                    let test_pred = poisk_contact::search::domain_pattern::predict_email_by_pattern(
                        "Иван",
                        "Петров",
                        &domain_clean,
                        &p_info.dominant_pattern,
                    );
                    println!("\nПример генерации для 'Иван Петров': {}", test_pred);
                }
                None => {
                    println!(
                        "Не удалось однозначно выявить паттерн (требуются имена из двух слов)."
                    );
                }
            }
        }
        return;
    }

    if fix_typos {
        println!("============================================================");
        println!("  Поиск и исправление опечаток в почтовых доменах (Typo Guard)");
        println!("============================================================");
        let mut contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let mut fixed_count = 0;
        for c in &mut contacts {
            if let Some(fix) = poisk_contact::search::domain_pattern::detect_domain_typo(&c.email) {
                println!("  [ИСПРАВЛЕНО] '{}' -> '{}' ({})", c.email, fix, c.company);
                c.email = fix;
                fixed_count += 1;
            }
        }
        if fixed_count > 0 {
            let _ = save_contacts_to_json(&contacts, &output_json);
            let _ = export_contacts_to_csv(&contacts, &output_csv);
            println!(
                "\nВсего исправлено {} адресов с опечатками. База обновлена.",
                fixed_count
            );
        } else {
            println!("Опечаток в доменах (gmai.com, yadnex.ru и др.) не обнаружено.");
        }
        return;
    }

    if let Some(max_daily) = warmup_plan_max {
        println!("============================================================");
        println!("  План плавного прогрева ящика (Warmup Ramp-Up Schedule)");
        println!(
            "  Целевой максимум: {} писем в день | Шаг: 5 писем/день",
            max_daily
        );
        println!("============================================================");
        println!(" День | Лимит писем | Рекомендуемый интервал отправки");
        println!("------+-------------+---------------------------------");
        for day in 1..=14 {
            let limit = poisk_contact::mailer::calculate_warmup_daily_limit(day - 1, 5, max_daily);
            let jitter = if day <= 3 {
                "70 - 140 сек."
            } else if day <= 7 {
                "45 - 90 сек."
            } else {
                "30 - 60 сек."
            };
            println!("  #{:>2} | {:>5} писем | {}", day, limit, jitter);
            if limit == max_daily {
                println!("  >>> Достигнут рабочий целевой объем на дне #{}!", day);
                break;
            }
        }
        return;
    }

    if spintax_preview {
        println!("============================================================");
        println!("  Предпросмотр вариантов Spintax (генерация 5 вариантов)");
        println!("============================================================");
        let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
            (
                poisk_contact::models::SmtpConfig::default(),
                poisk_contact::models::EmailTemplate::default(),
            )
        });
        println!("Исходный шаблон темы: {}", template.subject);
        println!(
            "Исходный шаблон тела:  {}\n",
            template.body.replace('\n', " ")
        );
        for n in 1..=5 {
            let s = poisk_contact::mailer::expand_spintax(&template.subject);
            let b = poisk_contact::mailer::expand_spintax(&template.body);
            println!("--- Вариант #{} ---", n);
            println!("Тема: {}", s);
            println!("Тело:\n{}\n", b);
        }
        return;
    }

    if permutate {
        println!("============================================================");
        println!("  Генерация комбинаций email (Email Permutator)");
        println!(
            "  Имя: '{}' | Фамилия: '{}' | Домен: '{}'",
            perm_first, perm_last, perm_domain
        );
        println!("============================================================");
        if perm_domain.is_empty() {
            eprintln!("[ERROR] Укажите домен компании с помощью --domain <ДОМЕН>");
            return;
        }
        let perms = poisk_contact::search::extractor::generate_email_permutations(
            &perm_first,
            &perm_last,
            &perm_domain,
        );
        println!("Сгенерировано {} кандидатов:", perms.len());
        for (idx, p) in perms.iter().enumerate() {
            println!("  [{:>2}] {}", idx + 1, p);
        }
        return;
    }

    if let Some(domain) = detect_catchall_target {
        println!("============================================================");
        println!("  Проверка домена на Catch-All конфигурацию: {}", domain);
        println!("============================================================");
        match poisk_contact::mailer::detect_catch_all_domain(&domain, None).await {
            Ok(true) => {
                println!("[CATCH-ALL: ДА] Сервер принимает любые несуществующие ящики (код 250).");
                println!("Внимание: SMTP валидация отдельных адресов для этого домена невозможна.");
            }
            Ok(false) => {
                println!("[CATCH-ALL: НЕТ] Сервер отклоняет несуществующие адреса.");
                println!("SMTP валидация и пермутации для этого домена ДОСТОВЕРНЫ.");
            }
            Err(e) => {
                println!("[ERROR] Ошибка проверки Catch-All: {}", e);
            }
        }
        return;
    }

    if let Some(target_email) = score_catchall_email {
        let mx_prov = score_catchall_mx.unwrap_or_else(|| "Google Workspace".to_string());
        println!("============================================================");
        println!("  Statistical Catch-All Pattern Confidence Scorer (Hunter.io Style)");
        println!("  Email:    {}", target_email);
        println!("  Провайдер:{}", mx_prov);
        println!("============================================================");
        let (confidence, verdict) =
            poisk_contact::search::catchall_scorer::calculate_catchall_confidence(
                &target_email,
                &mx_prov,
            );
        println!("Уверенность доставки: {}%", confidence);
        println!(
            "Оценка надежности:   {}",
            if confidence >= 80 {
                "ВЫСОКАЯ (Рекомендуется к отправке)"
            } else if confidence >= 60 {
                "СРЕДНЯЯ (Отправлять с осторожностью)"
            } else {
                "НИЗКАЯ / РИСК БАУНСА"
            }
        );
        println!("Аналитический вывод:  {}", verdict);
        println!("============================================================");
        return;
    }

    if let Some(target) = check_dnsbl_target {
        println!("============================================================");
        println!(
            "  Проверка домена/IP по базам черных списков (DNSBL): {}",
            target
        );
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        let res = poisk_contact::mailer::check_dnsbl_reputation(&client, &target).await;
        println!("Проверено спам-баз: {}", res.blacklists_checked);
        if res.listed {
            println!("[ALERT] Обнаружено внесение в черные списки:");
            for d in &res.detections {
                println!("  - {}", d);
            }
        } else {
            println!(
                "[CLEAN] Домен чист. В базах Spamhaus, SpamCop, SORBS, Barracuda не числится."
            );
        }
        return;
    }

    if let Some(target) = audit_domain_target {
        println!("============================================================");
        println!(
            "  Полный аудит домена отправителя (MX, SPF, DMARC, DNSBL): {}",
            target
        );
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let verifier = poisk_contact::search::doh::MxVerifier::new();
        let audit = poisk_contact::mailer::audit_sender_domain(&client, &verifier, &target).await;
        let dnsbl = poisk_contact::mailer::check_dnsbl_reputation(&client, &target).await;

        println!("Домен: {}", audit.domain);
        println!(
            "Грейд доставляемости (Deliverability Grade): {} (Оценка: {}/100)",
            audit.grade, audit.score
        );
        println!(
            "MX Записи:       {}",
            if audit.has_mx {
                format!("ДА ({})", audit.mx_record)
            } else {
                "НЕТ".to_string()
            }
        );
        println!(
            "SPF Запись:      {}",
            if audit.has_spf {
                format!("ДА ({})", audit.spf_record)
            } else {
                "НЕТ".to_string()
            }
        );
        println!(
            "DMARC Запись:    {}",
            if audit.has_dmarc {
                format!("ДА ({})", audit.dmarc_record)
            } else {
                "НЕТ".to_string()
            }
        );
        println!(
            "DKIM Селектор:   {}",
            if audit.has_dkim {
                format!("ДА (селектор '{}')", audit.dkim_selector)
            } else {
                "НЕ ОБНАРУЖЕН".to_string()
            }
        );
        println!(
            "BIMI Аватар:     {}",
            if audit.has_bimi {
                "ДА (default._bimi)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "DNSBL Репутация: {}",
            if dnsbl.listed {
                format!(
                    "ОБНАРУЖЕН В ЧЕРНОМ СПИСКЕ ({} детектов)",
                    dnsbl.detections.len()
                )
            } else {
                "ЧИСТО (0 детектов)".to_string()
            }
        );

        if !audit.recommendations.is_empty() {
            println!("\nРекомендации по настройке DNS:");
            for r in &audit.recommendations {
                println!("  * {}", r);
            }
        }
        return;
    }

    if let Some(target) = sender_health_target {
        println!("============================================================");
        println!(
            "  Unified Sender Health Index & Deliverability Grade (Smartlead / GlockApps Style)"
        );
        println!("  Домен: {}", target);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let verifier = poisk_contact::search::doh::MxVerifier::new();
        let audit = poisk_contact::mailer::audit_sender_domain(&client, &verifier, &target).await;
        let dnsbl = poisk_contact::mailer::check_dnsbl_reputation(&client, &target).await;
        let dkim_bits = if audit.has_dkim { 2048 } else { 0 };
        let report = poisk_contact::mailer::calculate_sender_health_index(
            &target,
            audit.has_mx,
            audit.has_spf,
            dkim_bits,
            &audit.dmarc_record,
            !dnsbl.listed,
            true,
        );

        println!(
            "Итоговый грейд домена: {} (Оценка: {} / 100)",
            report.grade, report.score
        );
        println!(
            "Статус готовности:     {}",
            if report.score >= 80 {
                "ОТЛИЧНО (Рекомендуется для холодных рассылок)"
            } else if report.score >= 60 {
                "УДОВЛЕТВОРИТЕЛЬНО (Требуется исправление DNS)"
            } else {
                "КРИТИЧЕСКИЙ РИСК СПАМА"
            }
        );
        println!("\nКомпоненты безопасности и доставляемости:");
        for c in &report.components {
            println!("  [+] {}", c);
        }
        if !report.critical_issues.is_empty() {
            println!("\nКритические проблемы репутации:");
            for issue in &report.critical_issues {
                println!("  [-] {}", issue);
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(target) = classify_company_target {
        println!("============================================================");
        println!("  Firmographic Size & Industry Vertical Classifier (Clearbit / Apollo Style)");
        println!("  Цель: {}", target);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        match poisk_contact::search::company_tier::classify_company_from_url(&client, &target).await
        {
            Ok(firmo) => {
                println!("Размер штата (Headcount):  {}", firmo.headcount_tier);
                println!("Отрасль (Industry):        {}", firmo.industry);
                println!("Уверенность классификации: {}%", firmo.confidence);
                if !firmo.signals.is_empty() {
                    println!("\nОбнаруженные сигналы сайта:");
                    for s in &firmo.signals {
                        println!("  * {}", s);
                    }
                }
            }
            Err(e) => eprintln!("[ERROR] Ошибка анализа сайта {}: {}", target, e),
        }
        println!("============================================================");
        return;
    }

    if let Some(raw_bounce) = classify_bounce_target {
        println!("============================================================");
        println!("  RFC 3464 SMTP DSN & Bounce Classification Taxonomy (SendGrid / Mailgun Style)");
        println!("  SMTP Ответ: \"{}\"", raw_bounce);
        println!("============================================================");
        let b = poisk_contact::search::bounce_classifier::classify_smtp_bounce(&raw_bounce);
        println!("Тип сбоя (Bounce Type):  {}", b.bounce_type);
        println!(
            "SMTP Код:                {}",
            if b.smtp_code != 0 {
                b.smtp_code.to_string()
            } else {
                "N/A".to_string()
            }
        );
        println!("Enhanced DSN Код:        {}", b.enhanced_code);
        println!(
            "Подавление (Suppress):   {}",
            if b.should_suppress {
                "ДА (Удалить из рассылки и стоп-лист)"
            } else {
                "НЕТ (Повторная отправка допустима)"
            }
        );
        println!("Рекомендуемое действие:  {}", b.recommended_action);
        println!("Диагностика сервера:     {}", b.diagnostic_message);
        println!("============================================================");
        return;
    }

    if let Some(tz_or_offset) = check_window_target {
        println!("============================================================");
        println!("  Sending Window & Business Hours Guard (Instantly / Lemlist Style)");
        println!("  Целевая таймзона / домен: {}", tz_or_offset);
        println!("============================================================");
        let offset =
            poisk_contact::search::sending_window::infer_utc_offset_from_target(&tz_or_offset);
        let now_utc = chrono::Utc::now();
        let win = poisk_contact::search::sending_window::check_sending_window(offset, now_utc);
        println!("Определенный сдвиг UTC:  UTC{:+}", offset);
        println!("Локальное время цели:    {}", win.local_time_formatted);
        println!("День недели:             {}", win.day_of_week);
        println!("Окно бизнес-часов:       {}", win.window_hours);
        println!(
            "Статус отправки:         {}",
            if win.is_active {
                "РАЗРЕШЕНО (Active Sending Window)"
            } else {
                "ЗАБЛОКИРОВАНО (Outside Business Hours)"
            }
        );
        if !win.is_active {
            println!(
                "Ожидание до открытия:    {} минут (~{:.1} часов)",
                win.minutes_until_next_window,
                win.minutes_until_next_window as f64 / 60.0
            );
        }
        println!("\nВердикт диспетчера:\n  {}", win.status_message);
        println!("============================================================");
        return;
    }

    if let Some(domain) = resolve_mx_target {
        println!("============================================================");
        println!("  Primary MX Preference Resolver & Gateway Isolation (Reacher / Truemail Style)");
        println!("  Целевой домен: {}", domain);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        match poisk_contact::search::mx_priority::resolve_and_prioritize_mx(&client, &domain).await
        {
            Ok(analysis) => {
                println!("Семейство провайдера:    {}", analysis.provider_family);
                println!(
                    "Отказоустойчивость:      {}",
                    if analysis.has_redundancy {
                        "ДА (Несколько независимых MX)"
                    } else {
                        "НЕТ (Single Point of Failure)"
                    }
                );
                if let Some(primary) = &analysis.primary_mx {
                    println!("\nПервичный авторитетный шлюз (Primary MX):");
                    println!("  [Priority {:>2}] {}", primary.preference, primary.host);
                } else {
                    println!("\n[ВНИМАНИЕ] Авторитетный MX шлюз не найден!");
                }
                if !analysis.backup_relays.is_empty() {
                    println!("\nРезервные релеи (Backup Relays):");
                    for r in &analysis.backup_relays {
                        println!("  [Priority {:>2}] {}", r.preference, r.host);
                    }
                }
                println!("\nАнализ архитектуры почты:\n  {}", analysis.recommendation);
            }
            Err(e) => eprintln!("[ERROR] DoH MX резолв не удался: {}", e),
        }
        println!("============================================================");
        return;
    }

    if audit_mime {
        println!("============================================================");
        println!("  MIME HTML-to-Text Ratio & Hidden Spam Trap Audit (Mailmeteor / GlockApps)");
        println!("============================================================");
        let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
            (
                poisk_contact::models::SmtpConfig::default(),
                poisk_contact::models::EmailTemplate::default(),
            )
        });
        println!("Тема шаблона: {}\n", template.subject);
        let rep = poisk_contact::search::mime_body_auditor::audit_mime_html_content(
            &template.body,
            &template.body,
        );
        println!("Символов в HTML:         {}", rep.html_char_count);
        println!("HTML тегов найдено:      {}", rep.tag_count);
        println!("HTML к тексту (Ratio):   {:.1}x", rep.html_to_text_ratio);
        println!(
            "Скрытый текст / 0-шрифт: {}",
            if rep.has_hidden_elements {
                "ОБНАРУЖЕН (display:none / zero-font trap)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Опасные теги (script):   {}",
            if rep.has_risky_tags {
                "ОБНАРУЖЕНЫ (<script>, <iframe>, <form>)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Inline Base64 картинки:  {}",
            if rep.has_base64_images {
                "ОБНАРУЖЕНЫ (Риск спам-штрафа за размер)"
            } else {
                "НЕТ"
            }
        );
        println!("Уровень риска инбокса:   {}", rep.deliverability_risk);
        println!("\nРекомендации аудита:");
        for a in &rep.advice {
            println!("  * {}", a);
        }
        println!("============================================================");
        return;
    }

    if let Some(target) = probe_starttls_target {
        println!("============================================================");
        println!("  Port 25 EHLO STARTTLS & Encryption Capability Probe (Mailgun Style)");
        println!("  Цель: {}", target);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let host_to_probe =
            if !target.contains('.') || target.contains('@') || !target.contains("mx") {
                let verifier = poisk_contact::search::doh::MxVerifier::new();
                let (has_mx, mx_h) = verifier.check_domain(&client, &target).await;
                if has_mx && !mx_h.is_empty() {
                    println!("Резолв Primary MX через DoH: {} -> {}", target, mx_h);
                    mx_h
                } else {
                    target.clone()
                }
            } else {
                target.clone()
            };

        println!("Подключение к {}:25 по TCP...", host_to_probe);
        let res = poisk_contact::search::starttls_probe::probe_mx_starttls(&host_to_probe, 5).await;
        println!(
            "Порт 25 доступен:        {}",
            if res.port_open {
                "ДА"
            } else {
                "НЕТ / Заблокирован"
            }
        );
        println!(
            "STARTTLS (RFC 3207):     {}",
            if res.supports_starttls {
                "ПОДДЕРЖИВАЕТСЯ (Транзитное шифрование активно)"
            } else {
                "ОТСУТСТВУЕТ (Только открытый текст)"
            }
        );
        println!("Рейтинг криптографии:    {}", res.security_rating);
        if !res.raw_banner.is_empty() {
            println!("Баннер сервера (220):    {}", res.raw_banner);
        }
        if !res.supported_extensions.is_empty() {
            println!("\nПоддерживаемые ESMTP расширения (EHLO):");
            for ext in &res.supported_extensions {
                println!("  [+] {}", ext);
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(domain) = audit_ttl_target {
        println!("============================================================");
        println!("  DNS TTL & Ephemeral Fast-Flux Risk Auditor (ZeroBounce / Spamhaus Style)");
        println!("  Домен: {}", domain);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        match poisk_contact::search::dns_ttl_risk::audit_dns_ttl_risk(&client, &domain).await {
            Ok(analysis) => {
                println!(
                    "Минимальный TTL:         {} сек (~{:.1} мин)",
                    analysis.min_ttl_seconds,
                    analysis.min_ttl_seconds as f64 / 60.0
                );
                println!(
                    "Максимальный TTL:        {} сек (~{:.1} ч)",
                    analysis.max_ttl_seconds,
                    analysis.max_ttl_seconds as f64 / 3600.0
                );
                println!(
                    "Средний TTL кэша:        {} сек",
                    analysis.average_ttl_seconds
                );
                println!(
                    "Риск Fast-Flux ботнета:  {}",
                    if analysis.is_ephemeral_risk {
                        "ВЫСОКИЙ (Ультракороткий TTL <300с)"
                    } else {
                        "НИЗКИЙ / Стабильная инфраструктура"
                    }
                );
                println!("Уровень стабильности:    {}", analysis.risk_level);
                println!(
                    "\nВердикт инфраструктуры:\n  {}",
                    analysis.stability_verdict
                );
                if !analysis.records_analyzed.is_empty() {
                    println!("\nПроанализированные DNS записи:");
                    for r in &analysis.records_analyzed {
                        println!("  * {}", r);
                    }
                }
            }
            Err(e) => eprintln!("[ERROR] Ошибка анализа TTL домена {}: {}", domain, e),
        }
        println!("============================================================");
        return;
    }

    if let Some(url) = mine_social_proof_target {
        println!("============================================================");
        println!("  Social Proof, Client Logos & B2B Certifications Miner (Clay / Apollo Style)");
        println!("  Сайт: {}", url);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        match poisk_contact::search::case_study_miner::mine_social_proof_from_url(&client, &url)
            .await
        {
            Ok(prof) => {
                println!("B2B Trust Score:         {}/100", prof.b2b_trust_score);
                println!(
                    "Секция кейсов/проектов:  {}",
                    if prof.has_case_studies {
                        "ДА"
                    } else {
                        "Не найдена"
                    }
                );
                println!(
                    "Секция отзывов:          {}",
                    if prof.has_testimonials {
                        "ДА"
                    } else {
                        "Не найдена"
                    }
                );
                if !prof.featured_clients.is_empty() {
                    println!(
                        "\nКлючевые клиенты / Логотипы:\n  {}",
                        prof.featured_clients.join(", ")
                    );
                }
                if !prof.certifications.is_empty() {
                    println!(
                        "\nГосаккредитации и лицензии:\n  {}",
                        prof.certifications.join("\n  ")
                    );
                }
                if !prof.personalized_hook.is_empty() {
                    println!(
                        "\nСгенерированный персонализированный хук для первого касания:\n  \"{}\"",
                        prof.personalized_hook
                    );
                }
            }
            Err(e) => eprintln!("[ERROR] Ошибка майнинга доверия сайта {}: {}", url, e),
        }
        println!("============================================================");
        return;
    }

    if let Some(ip) = audit_fcrdns_target {
        println!("============================================================");
        println!("  Forward-Confirmed Reverse DNS (FCrDNS) Auditor (RFC 8601)");
        println!("  IP адрес: {}", ip);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        let res = poisk_contact::search::fcrdns::audit_fcrdns(&client, &ip).await;
        println!("ARPA домен:          {}", res.arpa_domain);
        println!(
            "PTR Hostname:        {}",
            res.ptr_hostname.as_deref().unwrap_or("Не найден")
        );
        println!(
            "Forward IPs:         {}",
            if res.forward_ips.is_empty() {
                "Не разрешены".to_string()
            } else {
                res.forward_ips.join(", ")
            }
        );
        println!(
            "FCrDNS Валиден:      {}",
            if res.is_valid_fcrdns {
                "ДА (Чистый почтовый шлюз)"
            } else {
                "НЕТ (Отказ / Риск блокировки)"
            }
        );
        println!("Risk Score:          {}/100", res.risk_score);
        println!("Вердикт:             {}", res.verdict);
        println!("============================================================");
        return;
    }

    if let Some(domain) = detect_typo_target {
        println!("============================================================");
        println!("  Domain Typo & Lookalike Detector (Levenshtein Distance)");
        println!("  Домен: {}", domain);
        println!("============================================================");
        let res = poisk_contact::search::domain_typo::detect_domain_typo(&domain);
        println!(
            "Канонический провайдер: {}",
            if res.is_canonical { "ДА" } else { "НЕТ" }
        );
        println!(
            "Риск опечатки:          {}",
            if res.is_typo_risk {
                "ВЫСОКИЙ (Обнаружена ошибка ввода)"
            } else {
                "Низкий (Чистый корпоративный домен)"
            }
        );
        if let Some(sug) = &res.suggested_domain {
            println!("Предлагаемый домен:     {}", sug);
        }
        if let Some(dist) = res.edit_distance {
            println!("Расстояние Левенштейна: {}", dist);
        }
        println!("Risk Score:             {}/100", res.risk_score);
        println!("Детали:                 {}", res.reason);
        println!("============================================================");
        return;
    }

    if let Some(email) = audit_role_risk_target {
        println!("============================================================");
        println!("  Role Account & Generic Inbox Risk Tiering (NeverBounce Style)");
        println!("  Email: {}", email);
        println!("============================================================");
        let rep = poisk_contact::search::role_mailbox_risk::audit_role_mailbox_risk(&email);
        println!("Local-part:              {}", rep.local_part);
        println!("Категория ящика:         {}", rep.category);
        println!("Штраф спам-риска:        +{}/100", rep.risk_penalty);
        println!(
            "Понизить приоритет:      {}",
            if rep.should_deprioritize {
                "ДА (Низкая конверсия / Высокий риск жалоб)"
            } else {
                "НЕТ (Приоритетный контакт)"
            }
        );
        println!(
            "\nРекомендация по аутричу:\n  {}",
            rep.outreach_recommendation
        );
        println!("============================================================");
        return;
    }

    if let Some(url) = score_tech_maturity_target {
        println!("============================================================");
        println!("  Technology Stack Maturity & ICP Tier Scorer (Apollo / Clay Style)");
        println!("  Цель: {}", url);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        match poisk_contact::search::tech_maturity::evaluate_tech_maturity_from_url(&client, &url)
            .await
        {
            Ok(prof) => {
                println!("Tech Maturity Score:     {}/100", prof.score);
                println!("ICP Категория:           {}", prof.tier);
                if !prof.infra_signals.is_empty() {
                    println!(
                        "\nИнфраструктура и CDN:\n  * {}",
                        prof.infra_signals.join("\n  * ")
                    );
                }
                if !prof.frontend_signals.is_empty() {
                    println!(
                        "\nВеб-платформа и CMS:\n  * {}",
                        prof.frontend_signals.join("\n  * ")
                    );
                }
                if !prof.crm_signals.is_empty() {
                    println!("\nCRM и продажи:\n  * {}", prof.crm_signals.join("\n  * "));
                }
                if !prof.analytics_signals.is_empty() {
                    println!(
                        "\nМаркетинг и аналитика:\n  * {}",
                        prof.analytics_signals.join("\n  * ")
                    );
                }
                if !prof.ats_signals.is_empty() {
                    println!(
                        "\nРекрутинг и найм (ATS):\n  * {}",
                        prof.ats_signals.join("\n  * ")
                    );
                }
                println!(
                    "\nВердикт готовности к B2B сделке:\n  {}",
                    prof.sales_readiness_verdict
                );
            }
            Err(e) => eprintln!("[ERROR] Ошибка анализа стека {}: {}", url, e),
        }
        println!("============================================================");
        return;
    }

    if let Some(target) = audit_spf_lookups_target {
        println!("============================================================");
        println!("  RFC 7208 10-DNS-Lookup SPF Rule Auditor (GlockApps Style)");
        println!("  Цель / Домен: {}", target);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        match poisk_contact::search::spf_lookup_auditor::audit_domain_spf_lookups(&client, &target)
            .await
        {
            Ok(audit) => {
                println!("Запись SPF:              {}", audit.raw_record);
                println!(
                    "Потребление DNS Lookups: {} из {} разрешенных (RFC 7208)",
                    audit.lookup_count, audit.max_allowed
                );
                println!("Статус стандарта:        {}", audit.compliance_status);
                println!(
                    "Фатальный PermError:     {}",
                    if audit.is_permerror {
                        "ДА (Почта будет отклонена или направлена в Спам)"
                    } else {
                        "НЕТ (Соответствует спецификации)"
                    }
                );
                if !audit.lookup_mechanisms.is_empty() {
                    println!(
                        "\nМеханизмы DNS Lookups (затратные):\n  * {}",
                        audit.lookup_mechanisms.join("\n  * ")
                    );
                }
                if !audit.ip_mechanisms.is_empty() {
                    println!(
                        "\nПрямые IP диапазоны (безопасные):\n  * {}",
                        audit.ip_mechanisms.join("\n  * ")
                    );
                }
                println!("\nИнженерная рекомендация:\n  {}", audit.recommendation);
            }
            Err(e) => eprintln!("[ERROR] Ошибка аудита SPF {}: {}", target, e),
        }
        println!("============================================================");
        return;
    }

    if let Some(text_arg) = audit_reading_load_target {
        println!("============================================================");
        println!("  30-Second Skim & Cognitive Load Estimator (Lavender / Mailshake Style)");
        println!("============================================================");
        let text_to_audit = if text_arg == "template" {
            let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
                (
                    poisk_contact::models::SmtpConfig::default(),
                    poisk_contact::models::EmailTemplate::default(),
                )
            });
            template.body
        } else {
            text_arg
        };
        let rep = poisk_contact::search::reading_load::audit_reading_load(&text_to_audit);
        println!("Количество слов:         {}", rep.word_count);
        println!("Количество предложений:  {}", rep.sentence_count);
        println!("Количество абзацев:      {}", rep.paragraph_count);
        println!(
            "Время чтения (Skim):     ~{} секунд (при 200 сл/мин)",
            rep.reading_time_secs
        );
        println!(
            "Слов на предложение:     {:.1} (Идеал: <= 12-15)",
            rep.avg_words_per_sentence
        );
        println!(
            "Макс. предложений в абзаце: {} (Идеал: <= 2-3)",
            rep.max_paragraph_sentences
        );
        println!("Когнитивная нагрузка:    {}", rep.cognitive_load_tier);
        println!(
            "Адаптировано под моб.:   {}",
            if rep.mobile_skimmable {
                "ДА (Легко читается с экрана смартфона)"
            } else {
                "НЕТ (Перегружено, высокий риск архивации)"
            }
        );
        println!("\nРекомендации оптимизации:");
        for s in &rep.suggestions {
            println!("  * {}", s);
        }
        println!("============================================================");
        return;
    }

    if let Some(domain) = audit_dmarc_target {
        println!("============================================================");
        println!("  RFC 7489 DMARC & BIMI Enforcement Auditor (Google/Yahoo 2024)");
        println!("  Целевой домен: {}", domain);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let res = poisk_contact::search::dmarc_bimi::audit_dmarc_and_bimi(&client, &domain).await;
        println!(
            "Наличие DMARC:           {}",
            if res.has_dmarc {
                "ДА"
            } else {
                "НЕТ (Уязвимо для спуфинга)"
            }
        );
        if let Some(ref d) = res.dmarc_policy {
            println!(
                "Политика DMARC (p=):     {} (pct: {}%)",
                d.policy, d.percentage
            );
            println!(
                "Subdomain Policy (sp=):  {}",
                d.subdomain_policy.as_deref().unwrap_or("inherit")
            );
            println!(
                "Отчеты (RUA):            {}",
                if d.aggregate_reports.is_empty() {
                    "Не настроены".to_string()
                } else {
                    d.aggregate_reports.join(", ")
                }
            );
        }
        println!(
            "Наличие BIMI:            {}",
            if res.has_bimi {
                "ДА (Логотип подтвержден)"
            } else {
                "НЕТ"
            }
        );
        if let Some(ref b) = res.bimi_policy {
            println!(
                "SVG Логотип:             {}",
                b.logo_url.as_deref().unwrap_or("-")
            );
        }
        println!(
            "Google/Yahoo 2024:       {}",
            if res.is_esp_2024_compliant {
                "СООТВЕТСТВУЕТ (Защищено от спам-фильтров)"
            } else {
                "НЕ СООТВЕТСТВУЕТ (Риск реджекта)"
            }
        );
        println!("Грейд защиты:            {}", res.enforcement_grade);
        println!("Security Score:          {}/100", res.security_score);
        println!("\nИнженерная рекомендация:\n  {}", res.recommendation);
        println!("============================================================");
        return;
    }

    if let Some(text_arg) = audit_spam_matrix_target {
        println!("============================================================");
        println!("  Аудит матрицы спам-триггеров B2B (Lemlist / Saleshandy)");
        println!("============================================================");
        let text_to_audit = if text_arg == "template" {
            let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
                (
                    poisk_contact::models::SmtpConfig::default(),
                    poisk_contact::models::EmailTemplate::default(),
                )
            });
            template.body
        } else {
            text_arg
        };
        let rep =
            poisk_contact::search::spam_category_matrix::audit_spam_category_matrix(&text_to_audit);
        println!("Всего слов в тексте:       {}", rep.total_words);
        println!("Обнаружено спам-слов:      {}", rep.spam_words_count);
        println!(
            "Плотность спам-триггеров:  {:.1}%",
            rep.spam_word_density_percent
        );
        println!("Срабатываний по категориям:");
        println!("  - Urgency (Срочность):   {}", rep.urgency_hits);
        println!("  - Financial (Деньги):    {}", rep.financial_hits);
        println!("  - Freebies (Бесплатно):  {}", rep.freebie_hits);
        println!("  - Hype (Маркетинг/Хайп): {}", rep.hype_hits);
        println!("Штрафной балл:             {}/100", rep.penalty_score);
        println!("Вердикт доставляемости:    {}", rep.deliverability_verdict);
        if !rep.hits.is_empty() {
            println!("\nОбнаруженные спам-паттерны и B2B замены:");
            for hit in &rep.hits {
                println!(
                    "  * [{}] '{}' -> Рекомендуемая замена: '{}'",
                    hit.category, hit.word, hit.suggested_replacement
                );
            }
        }
        println!(
            "\nИнженерная рекомендация:\n  {}",
            rep.clean_b2b_recommendation
        );
        println!("============================================================");
        return;
    }

    if synthesize_patterns_flag {
        println!("============================================================");
        println!("  Hunter.io & Clearbit Email Pattern Synthesis Engine");
        println!("============================================================");
        if perm_domain.is_empty() {
            eprintln!("Ошибка: укажите целевой домен через --domain <ДОМЕН>");
            return;
        }
        let first_name = if perm_first.is_empty() {
            "Ivan"
        } else {
            &perm_first
        };
        let last_name = if perm_last.is_empty() {
            "Petrov"
        } else {
            &perm_last
        };

        let rep = poisk_contact::search::email_pattern_engine::synthesize_domain_patterns(
            first_name,
            last_name,
            &perm_domain,
            &[],
        );
        println!(
            "Целевой контакт:   {} {}",
            rep.target_first_name, rep.target_last_name
        );
        println!("Целевой домен:     {}", rep.domain);
        println!("Доминирующий паттерн: {}", rep.dominant_pattern);
        println!("Топ кандидат:      {}", rep.top_candidate);
        println!("\nСинтезированные варианты адресов:");
        for c in &rep.candidates {
            println!(
                "  [{:>3}% | {:<12}] {:<35} (Паттерн: {})",
                c.confidence_percent, c.market_prevalence, c.candidate_email, c.pattern_name
            );
        }
        println!("============================================================");
        return;
    }

    if let Some(ip) = validate_fcrdns_target {
        println!("============================================================");
        println!("  Строгая проверка Forward-Confirmed Reverse DNS (FCrDNS)");
        println!("  Целевой IP адрес: {}", ip);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let rep = poisk_contact::search::fcrdns_validator::validate_ip_fcrdns(&client, &ip).await;
        println!("In-Addr ARPA домен:        {}", rep.arpa_domain);
        println!(
            "PTR Hostname:              {}",
            rep.ptr_hostname.as_deref().unwrap_or("ОТСУТСТВУЕТ")
        );
        println!(
            "Forward A IP адреса:       {}",
            if rep.forward_ips.is_empty() {
                "-".to_string()
            } else {
                rep.forward_ips.join(", ")
            }
        );
        println!(
            "FCrDNS Roundtrip Valid:    {}",
            if rep.is_fcrdns_valid {
                "ДА (Полное соответствие RFC 8601)"
            } else {
                "НЕТ (Несоответствие PTR и A записей)"
            }
        );
        println!(
            "Динамический ISP / PBL:    {}",
            if rep.is_residential_dynamic {
                "ОБНАРУЖЕН (Блокировка Spamhaus PBL)"
            } else {
                "НЕТ (Статический Enterprise IP)"
            }
        );
        println!("Грейд репутации:           {}", rep.reputation_grade);
        println!(
            "\nВердикт доставляемости:\n  {}",
            rep.deliverability_verdict
        );
        println!("============================================================");
        return;
    }

    if let Some(text_arg) = audit_cta_friction_target {
        println!("============================================================");
        println!("  Оценка трения Call-To-Action (Woodpecker / Gong.io)");
        println!("============================================================");
        let text_to_audit = if text_arg == "template" {
            let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
                (
                    poisk_contact::models::SmtpConfig::default(),
                    poisk_contact::models::EmailTemplate::default(),
                )
            });
            template.body
        } else {
            text_arg
        };
        let rep = poisk_contact::search::cta_friction_scorer::audit_cta_friction(&text_to_audit);
        println!("Фрагмент CTA:              \"{}\"", rep.cta_excerpt);
        println!("Категория CTA:             {}", rep.cta_tier);
        println!("Коэффициент трения:        {}/100", rep.friction_score);
        println!(
            "Ссылка на календарь:       {}",
            if rep.has_booking_link {
                "ДА (Calendly/Cal.com - высокий барьер)"
            } else {
                "НЕТ (Прямой диалог)"
            }
        );
        println!("Прогноз конверсии:         {}", rep.conversion_prediction);
        println!("\nРекомендация оптимизации:\n  {}", rep.optimization_advice);
        println!("============================================================");
        return;
    }

    if let Some(msg) = audit_greylist_target {
        println!("============================================================");
        println!("  Анализатор грейлистинга и задержек SMTP (Kickbox / NeverBounce)");
        println!("============================================================");
        let rep = poisk_contact::search::greylist_detector::audit_smtp_greylisting(&msg);
        println!("Исходный ответ:            \"{}\"", rep.raw_response);
        println!("Категория задержки:        {}", rep.category);
        println!(
            "Обнаружен грейлистинг:     {}",
            if rep.is_greylisting_detected {
                "ДА (Анти-спам фильтрация первого касания)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Временная задержка (4xx):  {}",
            if rep.is_transient_delay {
                "ДА (Повторная отправка допустима)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Рекомендуемый интервал:    {} мин.",
            rep.recommended_retry_minutes
        );
        println!("Уверенность детекции:      {}%", rep.confidence_percent);
        println!(
            "\nИнженерная рекомендация:\n  {}",
            rep.handling_recommendation
        );
        println!("============================================================");
        return;
    }

    if let Some(title) = normalize_title_target {
        println!("============================================================");
        println!("  Нормализатор должностей и иерархии ЛПР (Snov.io / Lusha)");
        println!("============================================================");
        let rep = poisk_contact::search::title_normalizer::normalize_job_title(&title);
        println!("Исходная должность:        \"{}\"", rep.raw_title);
        println!("Стандартизированная:       {}", rep.clean_standard_title);
        println!("Уровень иерархии:          {}", rep.seniority_level);
        println!("Департамент:               {}", rep.department);
        println!("Вес влияния на бюджет:     {}/10", rep.influence_weight);
        println!(
            "Статус ЛПР (Decision Maker): {}",
            if rep.is_decision_maker {
                "ДА (Ключевое лицо, принимающее решения)"
            } else {
                "НЕТ (Технический эксперт / Инфлюенсер)"
            }
        );
        println!(
            "\nРекомендуемая стратегия аутрича:\n  {}",
            rep.recommended_approach
        );
        println!("============================================================");
        return;
    }

    if let Some(domain) = audit_mx_redundancy_target {
        println!("============================================================");
        println!("  Аудит отказоустойчивости MX-инфраструктуры (MXToolbox / Postmark)");
        println!("  Целевой домен: {}", domain);
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        let rep = poisk_contact::search::mx_redundancy_auditor::audit_domain_mx_redundancy(
            &client, &domain,
        )
        .await;
        println!("Всего записей MX:          {}", rep.total_mx_records);
        println!(
            "Уровней приоритета (Tiers): {}",
            rep.distinct_priority_tiers
        );
        println!(
            "Уникальных /24 подсетей:   {}",
            rep.distinct_c_class_subnets
        );
        println!(
            "Наличие резервного шлюза:  {}",
            if rep.has_failover_host {
                "ДА (Failover поддерживается)"
            } else {
                "НЕТ (Единая точка отказа)"
            }
        );
        println!(
            "Риск единой подсети (SPOF): {}",
            if rep.has_subnet_spof {
                "ОБНАРУЖЕН (Все MX в одной /24 подсети)"
            } else {
                "НЕТ (Географически распределены)"
            }
        );
        println!("Грейд отказоустойчивости:  {}", rep.redundancy_grade);
        println!(
            "\nВердикт инфраструктуры:\n  {}",
            rep.deliverability_verdict
        );
        if !rep.hosts.is_empty() {
            println!("\nОбнаруженные MX шлюзы:");
            for h in &rep.hosts {
                println!(
                    "  * [Pref: {:>2}] {:<35} (IP: {})",
                    h.preference,
                    h.hostname,
                    if h.resolved_ips.is_empty() {
                        "-".to_string()
                    } else {
                        h.resolved_ips.join(", ")
                    }
                );
            }
        }
        println!("\nИнженерная рекомендация:\n  {}", rep.recommendation);
        println!("============================================================");
        return;
    }

    if let Some(text_arg) = preflight_compliance_target {
        println!("============================================================");
        println!("  Префлайт-аудит регуляторного комплаенса (Instantly / Smartlead)");
        println!("============================================================");
        let text_to_audit = if text_arg == "template" {
            let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
                (
                    poisk_contact::models::SmtpConfig::default(),
                    poisk_contact::models::EmailTemplate::default(),
                )
            });
            template.body
        } else {
            text_arg
        };
        let rep = poisk_contact::search::preflight_compliance::audit_outbound_preflight(
            &text_to_audit,
            true,
        );
        println!("Индекс комплаенса:         {}/100", rep.compliance_score);
        println!("Статус готовности:         {}", rep.preflight_status);
        println!(
            "Заголовки RFC 8058:        {}",
            if rep.has_rfc8058_headers {
                "ДА (One-Click Unsubscribe активен)"
            } else {
                "НЕТ (Требуется добавить)"
            }
        );
        println!(
            "Текст/ссылка отписки:      {}",
            if rep.has_opt_out_link_or_text {
                "ДА (Присутствует в теле письма)"
            } else {
                "НЕТ (Нарушение CAN-SPAM)"
            }
        );
        println!(
            "Юридический/почтовый адрес: {}",
            if rep.has_physical_address {
                "ДА (Присутствует)"
            } else {
                "НЕТ (Рекомендуется добавить в подвал)"
            }
        );
        println!(
            "Идентификация отправителя: {}",
            if rep.has_sender_identity {
                "ДА (Прозрачная подпись)"
            } else {
                "НЕТ"
            }
        );
        if !rep.checklist_passed.is_empty() {
            println!("\nУспешные проверки:");
            for p in &rep.checklist_passed {
                println!("  [+] {}", p);
            }
        }
        if !rep.checklist_failed.is_empty() {
            println!("\nЗамечания / Нарушения:");
            for f in &rep.checklist_failed {
                println!("  [-] {}", f);
            }
        }
        if !rep.recommendations.is_empty() {
            println!("\nРекомендации по исправлению:");
            for r in &rep.recommendations {
                println!("  * {}", r);
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(reply_text) = classify_reply_target {
        println!("============================================================");
        println!("  Классификация ответа лида (Salesloft / Outreach Intelligence)");
        println!("============================================================");
        let rep = poisk_contact::search::sentiment_classifier::classify_sales_reply(&reply_text);
        println!("Анализируемый ответ:        \"{}\"", rep.raw_reply);
        println!("Категория ответа:           {}", rep.category);
        println!("Оценка настроения (Score):  {:+}", rep.sentiment_score);
        if let Some(ref target) = rep.extracted_referral_target {
            println!("Выявлен контакт коллеги:    {}", target);
        }
        println!("Действие для каденции:      {}", rep.cadence_action);
        println!(
            "\nИнженерная рекомендация:\n  {}",
            rep.handling_recommendation
        );
        println!("============================================================");
        return;
    }

    if let Some(spf_raw) = optimize_spf_target {
        println!("============================================================");
        println!("  Оптимизация и аудит синтаксиса SPF (Valimail / Red Sift)");
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let rep =
            poisk_contact::search::spf_syntax_optimizer::audit_and_optimize_spf(&client, &spf_raw)
                .await;
        println!("Исходная запись:            {}", rep.raw_spf);
        println!(
            "Синтаксис корректен:        {}",
            if rep.is_valid { "ДА" } else { "НЕТ" }
        );
        println!("Индекс чистоты (Hygiene):   {}/100", rep.hygiene_score);
        println!(
            "Угроза '+all' (Any IP):     {}",
            if rep.has_dangerous_plus_all {
                "ДА (КРИТИЧЕСКАЯ УЯЗВИМОСТЬ)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Устаревший 'ptr':           {}",
            if rep.has_deprecated_ptr {
                "ДА (Нарушение RFC 7208)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Механизм 'all' в середине:  {}",
            if rep.has_misplaced_all {
                "ДА (Игнорируется почтовиками)"
            } else {
                "НЕТ"
            }
        );
        println!(
            "Устранено дубликатов:       {}",
            rep.duplicate_mechanisms_count
        );
        println!("Оптимизированный SPF:       {}", rep.optimized_spf);
        if !rep.issues_found.is_empty() {
            println!("\nВыявленные дефекты:");
            for issue in &rep.issues_found {
                println!("  [-] {}", issue);
            }
        }
        if !rep.recommendations.is_empty() {
            println!("\nРекомендации:");
            for rec in &rep.recommendations {
                println!("  * {}", rec);
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(framework_url) = profile_framework_target {
        println!("============================================================");
        println!("  Профилирование веб-фреймворков и Jamstack (BuiltWith / Wappalyzer)");
        println!("============================================================");
        println!("Анализ URL: {}", framework_url);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        match poisk_contact::search::framework_profiler::profile_framework_from_url(
            &client,
            &framework_url,
        )
        .await
        {
            Ok(rep) => {
                println!("Целевой URL:                {}", rep.target_url);
                println!("Архитектурный уровень:      {}", rep.architecture_tier);
                println!(
                    "Modern Headless / Jamstack: {}",
                    if rep.is_modern_headless {
                        "ДА"
                    } else {
                        "НЕТ"
                    }
                );
                println!(
                    "Оценка готовности к аутричу: {}/100",
                    rep.sales_readiness_score
                );
                if !rep.detected_frameworks.is_empty() {
                    println!("\nОбнаруженные фронтенд-фреймворки:");
                    for f in &rep.detected_frameworks {
                        println!("  [+] {}", f);
                    }
                }
                if !rep.detected_backends.is_empty() {
                    println!("\nОбнаруженные платформы / бэкенд:");
                    for b in &rep.detected_backends {
                        println!("  [+] {}", b);
                    }
                }
                println!("\nPitch Hook для холодной рассылки:\n  {}", rep.pitch_hook);
            }
            Err(e) => {
                eprintln!("Ошибка профилирования URL {}: {}", framework_url, e);
            }
        }
        println!("============================================================");
        return;
    }

    if let Some(mailbox_email) = audit_mailbox_archetype_target {
        println!("============================================================");
        println!("  Аудит архетипа и deliverability tier ящика (ZeroBounce / TrueMail)");
        println!("============================================================");
        let rep = poisk_contact::search::mailbox_deliverability_tier::audit_mailbox_archetype(
            &mailbox_email,
        );
        println!("Анализируемый email:        {}", rep.email);
        println!("Локальная часть:            {}", rep.local_part);
        println!("Доменная часть:             {}", rep.domain);
        println!("Архетип почтового ящика:    {}", rep.archetype);
        println!(
            "Индекс доставляемости:      {}/100",
            rep.deliverability_index
        );
        println!("Риск групповых жалоб (Spam): {}", rep.group_complaint_risk);
        println!(
            "Безопасен для Cold Outreach: {}",
            if rep.is_outreach_safe {
                "ДА (Рекомендуется)"
            } else {
                "НЕТ (Высокий риск для репутации IP)"
            }
        );
        println!("\nСтратегия взаимодействия:\n  {}", rep.strategy_advice);
        println!("============================================================");
        return;
    }

    if let Some(url) = extract_firmo_url {
        println!("============================================================");
        println!(
            "  Извлечение фирмаграфики компании (JSON-LD & OpenGraph): {}",
            url
        );
        println!("============================================================");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        match client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(html) = resp.text().await {
                    let p = poisk_contact::search::firmographics::extract_firmographics(&html);
                    println!(
                        "Юридическое лицо: {}",
                        if !p.legal_name.is_empty() {
                            &p.legal_name
                        } else {
                            "Не найдено"
                        }
                    );
                    println!(
                        "Описание/Сфера:   {}",
                        if !p.description.is_empty() {
                            &p.description
                        } else {
                            "Не найдено"
                        }
                    );
                    println!(
                        "Адрес/Город:      {}",
                        if !p.address.is_empty() {
                            &p.address
                        } else {
                            "Не указан"
                        }
                    );
                    println!(
                        "Телефон:          {}",
                        if !p.phone.is_empty() {
                            &p.phone
                        } else {
                            "Не найден"
                        }
                    );
                    println!(
                        "Email:            {}",
                        if !p.email.is_empty() {
                            &p.email
                        } else {
                            "Не найден"
                        }
                    );
                    if !p.socials.is_empty() {
                        println!("Социальные сети:");
                        for s in &p.socials {
                            println!("  - {}", s);
                        }
                    }
                } else {
                    eprintln!("[ERROR] Не удалось прочитать тело страницы.");
                }
            }
            Err(e) => eprintln!("[ERROR] Ошибка запроса к {}: {}", url, e),
        }
        return;
    }

    if sanitize_db {
        println!("============================================================");
        println!("  Санитизация базы контактов (NeverBounce / ZeroBounce standard)");
        println!("  Файл: {}", output_json.display());
        println!("============================================================");
        let mut contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let total = contacts.len();
        let mut sanitized_emails = 0;
        let mut disposable_flagged = 0;
        let mut corporate_count = 0;
        let mut free_mail_count = 0;

        for c in &mut contacts {
            if !c.email.is_empty() {
                let clean = poisk_contact::search::extractor::sanitize_email(&c.email);
                if clean != c.email {
                    c.email = clean;
                    sanitized_emails += 1;
                }
                c.is_disposable = poisk_contact::search::extractor::is_disposable_email(&c.email);
                if c.is_disposable {
                    disposable_flagged += 1;
                }
                let is_free = poisk_contact::search::extractor::is_free_email(&c.email);
                if is_free {
                    free_mail_count += 1;
                } else {
                    corporate_count += 1;
                }
                c.b2b_tier = poisk_contact::search::extractor::calculate_b2b_tier(c);
            }
        }

        let _ = save_contacts_to_json(&contacts, &output_json);
        let _ = export_contacts_to_csv(&contacts, &output_csv);

        let corp_pct = if total > 0 {
            (corporate_count as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        println!("Всего записей в базе:       {}", total);
        println!("Исправлено/очищено email:   {}", sanitized_emails);
        println!("Одноразовых ящиков найдено: {}", disposable_flagged);
        println!(
            "Бесплатных ящиков (Free):   {} (gmail, yandex, mail.ru)",
            free_mail_count
        );
        println!("Корпоративных ящиков (B2B): {}", corporate_count);
        println!(
            "B2B Quality Index:          {:.1}% корпоративных контактов",
            corp_pct
        );
        println!(
            "База сохранена в {} и {}",
            output_json.display(),
            output_csv.display()
        );
        println!("============================================================");
        return;
    }

    if check_template {
        println!("============================================================");
        println!("  Анализ активного шаблона письма на спам-триггеры");
        println!("============================================================");
        let (_, template) = load_app_settings("settings.json").unwrap_or_else(|| {
            (
                poisk_contact::models::SmtpConfig::default(),
                poisk_contact::models::EmailTemplate::default(),
            )
        });
        println!("Тема: {}\nТело:\n{}\n", template.subject, template.body);
        let check = poisk_contact::mailer::analyze_spam_triggers(&template.subject, &template.body);
        println!("Индекс чистоты письма: {} / 100", check.score);
        println!(
            "Персонализация:        {}",
            if check.has_personalization {
                "Присутствует ({name}/{company})"
            } else {
                "ОТСУТСТВУЕТ"
            }
        );
        println!(
            "Агрессивный капслок:   {}",
            if check.excessive_caps {
                "ОБНАРУЖЕН"
            } else {
                "Нет"
            }
        );
        println!(
            "Множественные знаки:   {}",
            if check.excessive_punctuation {
                "ОБНАРУЖЕНЫ (!!! / ???)"
            } else {
                "Нет"
            }
        );
        if !check.trigger_words.is_empty() {
            println!("Найденные стоп-слова:  {}", check.trigger_words.join(", "));
        }
        println!("\nСоветы по улучшению:");
        for r in &check.recommendations {
            println!("  * {}", r);
        }
        return;
    }

    if check_port25 {
        println!("============================================================");
        println!("  Проверка исходящего порта 25 (Direct-to-MX)...");
        println!("============================================================");
        match poisk_contact::mailer::test_direct_mx_connectivity().await {
            Ok(msg) => {
                println!("[SUCCESS] Порт 25 ОТКРЫТ и доступен! {}", msg);
                println!("Локальная прямая отправка писем без сторонних серверов РАБОТАЕТ.");
            }
            Err(e) => {
                println!("[ERROR] Порт 25 недоступен: {}", e);
            }
        }
        return;
    }

    // Load active settings from settings.json or defaults
    let (mut config, template) = load_app_settings("settings.json").unwrap_or_else(|| {
        (
            poisk_contact::models::SmtpConfig::default(),
            poisk_contact::models::EmailTemplate::default(),
        )
    });

    if !custom_from.is_empty() {
        config.from_email = custom_from;
    }
    if !custom_reply_to.is_empty() {
        config.reply_to = custom_reply_to.clone();
    }

    if let Some(target) = relay_target {
        println!("============================================================");
        println!(
            "  Отправка письма через авторизованный SMTP релей на: {}",
            target
        );
        println!(
            "  Хост: {}:{} (User: {})",
            config.host, config.port, config.username
        );
        println!("============================================================");

        match poisk_contact::mailer::build_transport(&config) {
            Ok(transport) => {
                let subject = "Security & Engineering Collaboration Inquiry";
                let body = format!(
                    "Hello!\n\nThis message was sent via authenticated SMTP relay ({}) with full SPF/DKIM validation.\n\nBest regards,\n{}",
                    config.host, config.sender_name
                );

                match poisk_contact::mailer::send_via_relay(
                    &transport, &config, &target, subject, &body,
                ) {
                    Ok(_) => println!("[SUCCESS] Письмо успешно доставлено через SMTP релей!"),
                    Err(e) => eprintln!("[ERROR] Ошибка отправки через релей: {}", e),
                }
            }
            Err(e) => eprintln!("[ERROR] Ошибка инициализации SMTP транспорта: {}", e),
        }
        return;
    }

    if let Some(target) = direct_mx_target {
        println!("============================================================");
        println!("  Прямая отправка письма (Direct MX) на: {}", target);
        println!("============================================================");

        let subject = "Security & Engineering Collaboration Inquiry";
        let body = format!(
            "Hello!\n\nThis message was sent via direct-to-MX delivery.\n\nBest regards,\n{}",
            config.sender_name
        );

        match poisk_contact::mailer::send_direct_mx(&target, &config, subject, &body).await {
            Ok(_) => {
                println!("[SUCCESS] Письмо успешно передано на удаленный MX сервер!");
            }
            Err(e) => {
                eprintln!("[ERROR] Ошибка прямой передачи: {}", e);
            }
        }
        return;
    }

    if let Some(target) = batch_relay_target {
        println!("============================================================");
        println!(
            "  Пакетная отправка {} писем (SMTP Релей) на: {}",
            batch_count, target
        );
        println!(
            "  Хост: {}:{} | Пауза: {} мс",
            config.host, config.port, batch_delay_ms
        );
        println!("============================================================");

        let transport = match poisk_contact::mailer::build_transport(&config) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[ERROR] Ошибка SMTP релея: {}", e);
                return;
            }
        };

        let mut success = 0;
        let mut fail = 0;

        for num in 1..=batch_count {
            let unique_code = format!("{:04x}", (chrono::Utc::now().timestamp_millis() % 65535));
            let subject_raw = format!(
                "{{Security Collaboration|Application for Security Role|DevSecOps Inquiry}} [#{}/{} - {}]",
                num, batch_count, unique_code
            );
            let body_raw = format!(
                "Hi there,\n\nI am reaching out regarding security engineering opportunities.\n\nDelivery verification #{}/{} [id: {}].\n\nBest regards,\n{}",
                num, batch_count, unique_code, config.sender_name
            );

            let subject = poisk_contact::mailer::parse_spintax(&subject_raw);
            let body = poisk_contact::mailer::parse_spintax(&body_raw);

            match poisk_contact::mailer::send_via_relay(
                &transport, &config, &target, &subject, &body,
            ) {
                Ok(_) => {
                    success += 1;
                    println!(
                        "[{:>3}/{}] Успешно доставлено: \"{}\"",
                        num, batch_count, subject
                    );
                }
                Err(e) => {
                    fail += 1;
                    println!("[{:>3}/{}] Ошибка отправки: {}", num, batch_count, e);
                }
            }

            if num < batch_count && batch_delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(batch_delay_ms)).await;
            }
        }

        println!("============================================================");
        println!("  ИТОГИ ПАКЕТНОЙ ОТПРАВКИ ЧЕРЕЗ РЕЛЕЙ:");
        println!("  Успешно доставлено: {} из {}", success, batch_count);
        println!("  Ошибок:             {}", fail);
        println!("============================================================");
        return;
    }

    if sync_replies {
        println!("============================================================");
        println!("  Синхронизация входящих ответов и баунсов через IMAP...");
        println!("  Ящик: {}", config.username);
        println!("============================================================");

        let mut contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        if contacts.is_empty() {
            println!("База контактов пуста!");
            return;
        }

        let py_code = r#"
import imaplib, email, json, sys
from email.header import decode_header
sys.stdout.reconfigure(encoding='utf-8')
try:
    with open('settings.json', 'r', encoding='utf-8') as f:
        cfg = json.load(f)
    smtp = cfg.get('smtp', {})
    addr = smtp.get('username') or smtp.get('from_email')
    pw = (smtp.get('password') or '').replace(' ', '')
    if not addr or not pw:
        print('[]')
        sys.exit(0)
    m = imaplib.IMAP4_SSL('imap.gmail.com')
    m.login(addr, pw)
    m.select('INBOX', readonly=True)
    status, data = m.search(None, 'ALL')
    if status != 'OK' or not data or not data[0]:
        print('[]')
        sys.exit(0)
    ids = data[0].split()
    results = []
    for msg_id in ids[-50:]:
        s, d = m.fetch(msg_id, '(BODY.PEEK[HEADER.FIELDS (FROM SUBJECT IN-REPLY-TO REFERENCES DATE)])')
        if s == 'OK' and d and d[0] and len(d[0]) > 1:
            raw = email.message_from_bytes(d[0][1])
            def dec(h):
                val = ''
                for p, enc in decode_header(h or ''):
                    val += p.decode(enc or 'utf-8', 'replace') if isinstance(p, bytes) else str(p)
                return val.strip()
            frm = dec(raw.get('From', ''))
            subj = dec(raw.get('Subject', ''))
            in_rep = (raw.get('In-Reply-To') or '').strip()
            refs = (raw.get('References') or '').strip()
            results.append({'from': frm, 'subject': subj, 'in_reply_to': in_rep, 'references': refs})
    m.close()
    m.logout()
    print(json.dumps(results, ensure_ascii=False))
except Exception:
    print('[]')
"#;

        let output = std::process::Command::new("python")
            .arg("-c")
            .arg(py_code)
            .output();

        let mut replied_count = 0;
        let mut bounce_count = 0;

        match output {
            Ok(out) => {
                let stdout_str = String::from_utf8_lossy(&out.stdout);
                if let Ok(inbound_list) =
                    serde_json::from_str::<Vec<serde_json::Value>>(&stdout_str)
                {
                    println!(
                        "Проверено {} последних входящих писем...",
                        inbound_list.len()
                    );
                    for inbound in &inbound_list {
                        let from_header = inbound
                            .get("from")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let subject = inbound
                            .get("subject")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let in_reply_to = inbound
                            .get("in_reply_to")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let references = inbound
                            .get("references")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        let is_bounce = from_header.contains("mailer-daemon")
                            || from_header.contains("postmaster")
                            || subject.contains("delivery status notification")
                            || subject.contains("undeliverable")
                            || subject.contains("failure");

                        for c in contacts.iter_mut() {
                            let c_email = c.email.trim().to_lowercase();
                            if c_email.is_empty() {
                                continue;
                            }

                            let matches_from = from_header.contains(&c_email);
                            let matches_thread = if let Some(last_mid) = &c.last_message_id {
                                !last_mid.is_empty()
                                    && (in_reply_to.contains(last_mid)
                                        || references.contains(last_mid))
                            } else {
                                false
                            };

                            if matches_from || matches_thread {
                                if is_bounce
                                    && c.status != SendStatus::Failed("Hard Bounce".to_string())
                                {
                                    c.status = SendStatus::Failed("Hard Bounce".to_string());
                                    bounce_count += 1;
                                    println!(
                                        "  [BOUNCE DETECTED] {} ({}) -> Отмечен как Hard Bounce",
                                        c.company, c.email
                                    );
                                } else if !is_bounce && c.status != SendStatus::Replied {
                                    c.status = SendStatus::Replied;
                                    replied_count += 1;
                                    println!(
                                        "  [REPLY DETECTED] {} ({}) -> Получен ответ от рекрутера!",
                                        c.company, c.email
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[ERROR] Не удалось опросить IMAP: {}", e);
            }
        }

        if replied_count > 0 || bounce_count > 0 {
            let _ = save_contacts_to_json(&contacts, &output_json);
            println!("============================================================");
            println!("  ИТОГИ СИНХРОНИЗАЦИИ:");
            println!("  Новых ответов рекрутеров: {}", replied_count);
            println!("  Новых баунсов (недоставлено): {}", bounce_count);
            println!("  База контактов обновлена в {}", output_json.display());
            println!("============================================================");
        } else {
            println!("Новых ответов или баунсов по отправленным контактам не обнаружено.");
        }
        return;
    }

    if send_contacts {
        println!("============================================================");
        println!(
            "  Рассылка по базе контактов ({}) через SMTP Релей",
            output_json.display()
        );
        println!(
            "  Отправитель: {} <{}>",
            config.sender_name, config.from_email
        );
        println!("============================================================");

        let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
        if config.last_sent_day != today_str {
            config.sent_today = 0;
            config.last_sent_day = today_str.clone();
        }

        if config.daily_limit > 0 && config.sent_today >= config.daily_limit && !dry_run {
            println!(
                "[QUOTA LIMIT] Достигнут суточный лимит отправок: {}/{} писем за {}.",
                config.sent_today, config.daily_limit, today_str
            );
            println!(
                "Для защиты репутации домена и предотвращения блокировки рассылка приостановлена."
            );
            return;
        }

        let remaining_quota = if config.daily_limit > 0 {
            config.daily_limit.saturating_sub(config.sent_today)
        } else {
            usize::MAX
        };

        let effective_limit = batch_count.min(remaining_quota);

        let mut contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let pending: Vec<_> = contacts
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                if c.email.trim().is_empty() || c.status != SendStatus::Pending {
                    return false;
                }
                if only_personal && c.is_role_account {
                    return false;
                }
                if min_score > 0 && c.lead_score < min_score {
                    return false;
                }
                true
            })
            .map(|(i, _)| i)
            .take(effective_limit)
            .collect();

        if pending.is_empty() {
            println!(
                "Нет контактов с доступными адресами для рассылки (соответствующих фильтрам)!"
            );
            return;
        }

        let transport = if !dry_run && !config.direct_mx {
            match poisk_contact::mailer::build_transport(&config) {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("[ERROR] Ошибка инициализации релея: {}", e);
                    return;
                }
            }
        } else {
            None
        };

        println!("Найдено {} контактов для рассылки...", pending.len());
        let mut sent_count = 0;
        let mut fail_count = 0;

        for (order, &idx) in pending.iter().enumerate() {
            let contact = &mut contacts[idx];
            let subj_spinned = poisk_contact::mailer::expand_spintax(&template.subject);
            let body_spinned = poisk_contact::mailer::expand_spintax(&template.body);
            let subject =
                poisk_contact::mailer::render_template(&subj_spinned, contact, &config.sender_name);
            let body =
                poisk_contact::mailer::render_template(&body_spinned, contact, &config.sender_name);

            println!(
                "[{}/{}] Отправка на {} ({})...",
                order + 1,
                pending.len(),
                contact.email,
                contact.company
            );
            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let send_result = if dry_run {
                println!(
                    "  [DRY-RUN] Письмо сформировано (Тема: '{}'). Реальная отправка пропущена.",
                    subject
                );
                Ok("<dry_run_msg_id>".to_string())
            } else if let Some(ref t) = transport {
                poisk_contact::mailer::send_via_relay_with_headers(
                    t,
                    &config,
                    &contact.email,
                    &subject,
                    &body,
                    None,
                )
            } else {
                poisk_contact::mailer::send_direct_mx_with_headers(
                    &contact.email,
                    &config,
                    &subject,
                    &body,
                    None,
                )
                .await
            };

            match send_result {
                Ok(msg_id) => {
                    sent_count += 1;
                    if !dry_run {
                        contact.status = SendStatus::Sent;
                        contact.last_sent_at = Some(now_str);
                        contact.last_message_id = Some(msg_id);
                        contact.follow_up_count = 1;
                        config.sent_today += 1;
                    }
                    println!(
                        "  [SUCCESS] {}",
                        if dry_run {
                            "Проверено (тестовый прогон)"
                        } else {
                            "Доставлено!"
                        }
                    );
                }
                Err(e) => {
                    fail_count += 1;
                    if !dry_run {
                        contact.status = SendStatus::Failed(e.clone());
                    }
                    eprintln!("  [ERROR] {}", e);
                }
            }

            if order + 1 < pending.len() && config.delay_seconds > 0 && !dry_run {
                let delay_ms = if config.random_delay {
                    let nanos = chrono::Utc::now()
                        .timestamp_nanos_opt()
                        .unwrap_or(0)
                        .unsigned_abs();
                    let jitter = 0.75 + ((nanos % 60) as f64 / 100.0);
                    (config.delay_seconds as f64 * 1000.0 * jitter) as u64
                } else {
                    config.delay_seconds * 1000
                };
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }

        if !dry_run {
            let _ = save_contacts_to_json(&contacts, &output_json);
            let _ = poisk_contact::storage::save_app_settings(&config, &template, "settings.json");
        }
        println!("============================================================");
        println!(
            "  ИТОГИ РАССЫЛКИ ПО БАЗЕ: {} {}{}",
            sent_count,
            if dry_run {
                "смоделировано"
            } else {
                "отправлено"
            },
            if fail_count > 0 {
                format!(", {} ошибок", fail_count)
            } else {
                "".to_string()
            }
        );
        if !dry_run {
            println!("  База контактов обновлена в {}", output_json.display());
        } else {
            println!(
                "  [DRY-RUN] Изменения не сохранялись в файл {}",
                output_json.display()
            );
        }
        println!("============================================================");
        return;
    }

    if send_followups {
        let step_desc = if let Some(s) = followups_step {
            format!(
                "(Шаг #{}, порог ожидания {} дн.)",
                s,
                if s == 1 { 3 } else { 7 }
            )
        } else {
            "(все созревшие шаги)".to_string()
        };
        println!("============================================================");
        println!(
            "  Запуск Drip Follow-Up рассылки {} ({})",
            step_desc,
            output_json.display()
        );
        println!(
            "  Отправитель: {} <{}>",
            config.sender_name, config.from_email
        );
        println!("============================================================");

        let mut contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let followable: Vec<_> = contacts
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                if c.email.trim().is_empty() || c.status != SendStatus::Sent {
                    return false;
                }
                if let Some(target_step) = followups_step {
                    let min_days = if target_step == 1 { 3 } else { 7 };
                    poisk_contact::mailer::is_due_for_followup(c, target_step, min_days)
                } else {
                    c.follow_up_count >= 1 && c.follow_up_count < 3
                }
            })
            .map(|(i, _)| i)
            .collect();

        if followable.is_empty() {
            println!(
                "Нет контактов, ожидающих повторного письма (по сроку ожидания или фильтрам)!"
            );
            return;
        }

        let transport = match poisk_contact::mailer::build_transport(&config) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[ERROR] Ошибка инициализации релея: {}", e);
                return;
            }
        };

        println!(
            "Найдено {} контактов для отправки Follow-Up...",
            followable.len()
        );
        let mut sent_count = 0;
        let mut fail_count = 0;

        for (order, &idx) in followable.iter().enumerate() {
            let contact = &mut contacts[idx];
            let stage = contact.follow_up_count + 1;
            let orig_subject = poisk_contact::mailer::render_template(
                &template.subject,
                contact,
                &config.sender_name,
            );
            let subject = if orig_subject.starts_with("Re:") {
                orig_subject
            } else {
                format!("Re: {}", orig_subject)
            };

            let is_creative = template.template_name.to_lowercase().contains("fashion")
                || template.template_name.to_lowercase().contains("creative");

            let follow_body_template = if stage == 2 {
                template.follow_up_1.as_deref().unwrap_or_else(|| {
                    if is_creative {
                        "Здравствуйте, {name}!\n\nКратко напоминаю о своем письме по поводу визуального продакшна и съемок для {company}.\n\nЕсли у вас в планах обновление лукбука, кампейн или имиджевое видео под ближайший релиз — буду рад созвониться на 5 минут или прислать готовую смету под ваш формат.\n\nС уважением,\n{sender_name}"
                    } else {
                        "Hi {name},\n\nFollowing up briefly on my previous note regarding {role} / security collaboration at {company}.\n\nI understand your schedule is busy. If you have 5-10 minutes next week, I would be glad to connect or share my background summary.\n\nBest regards,\n{sender_name}"
                    }
                })
            } else {
                template.follow_up_2.as_deref().unwrap_or_else(|| {
                    if is_creative {
                        "Здравствуйте, {name}!\n\nФинально уточняю по поводу съемок для {company}. Если пока задача закрыта внутри команды — без проблем, буду рад оставаться на связи на случай будущих спецпроектов.\n\nС уважением,\n{sender_name}"
                    } else {
                        "Hi {name},\n\nWanted to check in one last time regarding potential fit for {role} at {company}. If this is not a priority right now, no worries at all.\n\nBest regards,\n{sender_name}"
                    }
                })
            };

            let body = poisk_contact::mailer::render_template(
                follow_body_template,
                contact,
                &config.sender_name,
            );
            let in_reply_to = contact.last_message_id.as_deref();

            println!(
                "[{}/{}] Отправка Follow-Up #{} на {} ({})...",
                order + 1,
                followable.len(),
                stage,
                contact.email,
                contact.company
            );
            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

            let send_result = if dry_run {
                println!(
                    "  [DRY-RUN] Follow-Up #{} сформирован (Тема: '{}'). Реальная отправка пропущена.",
                    stage, subject
                );
                Ok("<dry_run_msg_id>".to_string())
            } else {
                poisk_contact::mailer::send_via_relay_with_headers(
                    &transport,
                    &config,
                    &contact.email,
                    &subject,
                    &body,
                    in_reply_to,
                )
            };

            match send_result {
                Ok(msg_id) => {
                    sent_count += 1;
                    if !dry_run {
                        contact.follow_up_count = stage;
                        contact.last_sent_at = Some(now_str);
                        contact.last_message_id = Some(msg_id);
                        config.sent_today += 1;
                    }
                    println!(
                        "  [SUCCESS] Follow-Up #{} {}",
                        stage,
                        if dry_run {
                            "проверен (тестовый прогон)"
                        } else {
                            "доставлен!"
                        }
                    );
                }
                Err(e) => {
                    fail_count += 1;
                    eprintln!("  [ERROR] {}", e);
                }
            }

            if order + 1 < followable.len() && config.delay_seconds > 0 && !dry_run {
                let delay_ms = if config.random_delay {
                    let nanos = chrono::Utc::now()
                        .timestamp_nanos_opt()
                        .unwrap_or(0)
                        .unsigned_abs();
                    let jitter = 0.75 + ((nanos % 60) as f64 / 100.0);
                    (config.delay_seconds as f64 * 1000.0 * jitter) as u64
                } else {
                    config.delay_seconds * 1000
                };
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }

        if !dry_run {
            let _ = save_contacts_to_json(&contacts, &output_json);
            let _ = poisk_contact::storage::save_app_settings(&config, &template, "settings.json");
        }
        println!("============================================================");
        println!(
            "  ИТОГИ DRIP FOLLOW-UP: {} {}{}",
            sent_count,
            if dry_run {
                "смоделировано"
            } else {
                "отправлено"
            },
            if fail_count > 0 {
                format!(", {} ошибок", fail_count)
            } else {
                "".to_string()
            }
        );
        if !dry_run {
            println!("  База контактов обновлена в {}", output_json.display());
        } else {
            println!(
                "  [DRY-RUN] Изменения не сохранялись в файл {}",
                output_json.display()
            );
        }
        println!("============================================================");
        return;
    }

    if enrich_contacts {
        println!("============================================================");
        println!(
            "  Обогащение базы контактов ({}) корпоративными email",
            output_json.display()
        );
        println!("============================================================");
        let mut contacts = load_contacts_from_json(&output_json).unwrap_or_default();
        let initial_missing = contacts
            .iter()
            .filter(|c| c.email.trim().is_empty())
            .count();
        println!(
            "Всего записей в базе: {}, из них без email: {}",
            contacts.len(),
            initial_missing
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        let mx_verifier = poisk_contact::search::doh::MxVerifier::new();

        poisk_contact::search::permutator::enrich_contacts_with_permutator(
            &client,
            &mx_verifier,
            &mut contacts,
        )
        .await;

        println!(
            "Обогащение метаданными (провайдер, Catch-All, Role-аккаунты, Lead Score, Gravatar)..."
        );
        for c in &mut contacts {
            if !c.email.is_empty() {
                c.is_role_account = poisk_contact::search::extractor::is_role_based_email(&c.email);
                c.is_disposable = poisk_contact::search::extractor::is_disposable_email(&c.email);
                if c.mail_provider.is_empty() && !c.mx_record.is_empty() {
                    c.mail_provider =
                        poisk_contact::search::extractor::detect_mail_provider(&c.mx_record);
                }
                if !c.gravatar_found && c.avatar_url.is_empty() {
                    if let Some(av) =
                        poisk_contact::search::extractor::check_gravatar_profile(&client, &c.email)
                            .await
                    {
                        c.gravatar_found = true;
                        c.avatar_url = av;
                    }
                }
                c.lead_score = poisk_contact::search::extractor::calculate_lead_score(c);
            }
        }

        let now_with_email = contacts
            .iter()
            .filter(|c| !c.email.trim().is_empty())
            .count();
        let _ = save_contacts_to_json(&contacts, &output_json);
        let _ = export_contacts_to_csv(&contacts, &output_csv);

        println!("============================================================");
        println!(
            "  ИТОГИ ОБОГАЩЕНИЯ: теперь с валидным email: {} из {}",
            now_with_email,
            contacts.len()
        );
        println!(
            "  База сохранена в {} и {}",
            output_json.display(),
            output_csv.display()
        );
        println!("============================================================");
        return;
    }

    if let Some(target) = batch_target {
        println!("============================================================");
        println!(
            "  Пакетная отправка {} писем (Direct MX) на: {}",
            batch_count, target
        );
        println!("  Пауза между отправками: {} мс", batch_delay_ms);
        println!("============================================================");

        let mut success = 0;
        let mut fail = 0;

        for num in 1..=batch_count {
            let unique_code = format!("{:04x}", (chrono::Utc::now().timestamp_millis() % 65535));
            let subject_raw = format!(
                "{{Security Collaboration|Application for Security Role|DevSecOps Inquiry}} [#{}/{} - {}]",
                num, batch_count, unique_code
            );
            let body_raw = format!(
                "Hello,\n\nDirect MX delivery test #{}/{} [id: {}].\n\nBest regards,\n{}",
                num, batch_count, unique_code, config.sender_name
            );

            let subject = poisk_contact::mailer::parse_spintax(&subject_raw);
            let body = poisk_contact::mailer::parse_spintax(&body_raw);

            let mut sent_ok = false;
            let mut err_msg = String::new();

            for _attempt in 0..3 {
                match poisk_contact::mailer::send_direct_mx(&target, &config, &subject, &body).await
                {
                    Ok(_) => {
                        sent_ok = true;
                        break;
                    }
                    Err(e) => {
                        err_msg = e;
                        if err_msg.contains("421") || err_msg.contains("450") {
                            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                        } else {
                            break;
                        }
                    }
                }
            }

            if sent_ok {
                success += 1;
                println!(
                    "[{:>3}/{}] Успешно доставлено: \"{}\"",
                    num, batch_count, subject
                );
            } else {
                fail += 1;
                println!("[{:>3}/{}] Ошибка отправки: {}", num, batch_count, err_msg);
            }

            if num < batch_count && batch_delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(batch_delay_ms)).await;
            }
        }

        println!("============================================================");
        println!("  ИТОГИ ПАКЕТНОЙ ОТПРАВКИ:");
        println!("  Успешно доставлено: {} из {}", success, batch_count);
        println!("  Ошибок:             {}", fail);
        println!("============================================================");
        return;
    }

    println!("============================================================");
    println!("  PoiskContact Pro — CLI OSINT Search Engine");
    println!("  Локация: {}", city);
    println!("  Стек:    {}", stack);
    println!("  Лимит:   {} записей", max_items);
    println!("============================================================");

    let mut filter = SearchFilter::default();
    filter.city = city;
    filter.stack = stack;
    filter.max_items = max_items;

    let (tx, rx) = channel();

    let search_handle = tokio::spawn(async move {
        run_hybrid_search(filter, tx).await;
    });

    let mut collected = Vec::new();

    while let Ok(msg) = rx.recv() {
        match msg {
            SearchProgress::Log(level, text) => {
                let now = chrono::Local::now().format("%H:%M:%S").to_string();
                println!("[{}] [{:>7}] {}", now, level, text);
            }
            SearchProgress::Done(records) => {
                collected = records;
                break;
            }
            SearchProgress::Collected(records) => {
                collected = records;
            }
            SearchProgress::Error(err) => {
                eprintln!("[ERROR] {}", err);
                std::process::exit(1);
            }
        }
    }

    let _ = search_handle.await;

    println!("\n============================================================");
    println!(
        "  ИТОГИ СБОРА: получено {} уникальных компаний и контактов",
        collected.len()
    );
    println!("============================================================");

    for (idx, c) in collected.iter().enumerate() {
        println!(
            "{:>2}. [{}] {:<28} | {:<22} | {:<16} | {:<15} | {}",
            idx + 1,
            c.score,
            c.company,
            if c.email.is_empty() { "-" } else { &c.email },
            if c.phone.is_empty() { "-" } else { &c.phone },
            if c.telegram.is_empty() {
                "-"
            } else {
                &c.telegram
            },
            c.source
        );
    }

    println!("\nСохранение результатов...");
    if let Err(e) = save_contacts_to_json(&collected, &output_json) {
        eprintln!("Ошибка сохранения JSON: {}", e);
    } else {
        println!("JSON сохранен в: {}", output_json.display());
    }

    if let Err(e) = export_contacts_to_csv(&collected, &output_csv) {
        eprintln!("Ошибка экспорта CSV: {}", e);
    } else {
        println!("CSV экспортирован в: {}", output_csv.display());
    }

    println!("Готово!");
}
