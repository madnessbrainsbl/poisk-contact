use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SendStatus {
    #[default]
    #[serde(alias = "Ready")]
    Pending,
    Sent,
    Failed(String),
    Skipped,
    Replied,
}

fn default_status() -> SendStatus {
    SendStatus::Pending
}

fn default_score() -> LeadScore {
    LeadScore::Warm
}

impl std::fmt::Display for SendStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendStatus::Pending => write!(f, "Pending"),
            SendStatus::Sent => write!(f, "Sent"),
            SendStatus::Failed(err) => write!(f, "Failed: {}", err),
            SendStatus::Skipped => write!(f, "Skipped"),
            SendStatus::Replied => write!(f, "Replied"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum LeadScore {
    Hot, // Direct HR/Founder email from vacancy or explicit job post
    #[default]
    Warm, // Contact from /contacts or verified site page with MX
    Cold, // General address (info@, contact@) or hypothesis
}

impl std::fmt::Display for LeadScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeadScore::Hot => write!(f, "HOT"),
            LeadScore::Warm => write!(f, "WARM"),
            LeadScore::Cold => write!(f, "COLD"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ContactEvidence {
    pub url: String,
    pub checked_at: String,
    pub emails: Vec<String>,
    pub phones: Vec<String>,
    pub telegrams: Vec<String>,
    pub vk_urls: Vec<String>,
    pub location_matches: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContactRecord {
    #[serde(default)]
    pub evidence: Vec<ContactEvidence>,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub selected: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub stack: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub telegram: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub url: String,
    #[serde(default = "default_status")]
    pub status: SendStatus,
    #[serde(default = "default_score")]
    pub score: LeadScore,
    #[serde(default)]
    pub mx_valid: bool,
    #[serde(default)]
    pub mx_record: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub follow_up_count: u32,
    #[serde(default)]
    pub last_sent_at: Option<String>,
    #[serde(default)]
    pub last_message_id: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub vk_url: String,
    #[serde(default)]
    pub whatsapp: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub rcpt_verified: bool,
    #[serde(default)]
    pub is_catch_all: bool,
    #[serde(default)]
    pub is_disposable: bool,
    #[serde(default)]
    pub gravatar_found: bool,
    #[serde(default)]
    pub avatar_url: String,
    #[serde(default)]
    pub mail_provider: String,
    #[serde(default)]
    pub is_role_account: bool,
    #[serde(default)]
    pub lead_score: u8,
    #[serde(default)]
    pub tech_stack: Vec<String>,
    #[serde(default)]
    pub analytics_ids: Vec<String>,
    #[serde(default)]
    pub domain_age_days: Option<i64>,
    #[serde(default)]
    pub domain_registrar: String,
    #[serde(default)]
    pub domain_pattern: String,
    #[serde(default)]
    pub suggested_email_fix: Option<String>,
    #[serde(default)]
    pub company_description: String,
    #[serde(default)]
    pub legal_name: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub b2b_tier: String,
    #[serde(default)]
    pub phone_carrier: String,
    #[serde(default)]
    pub phone_region: String,
    #[serde(default)]
    pub phone_type: String,
    #[serde(default)]
    pub whatsapp_url: String,
    #[serde(default)]
    pub telegram_url: String,
    #[serde(default)]
    pub github_url: String,
    #[serde(default)]
    pub github_bio: String,
    #[serde(default)]
    pub inn: String,
    #[serde(default)]
    pub ogrn: String,
    #[serde(default)]
    pub vat_id: String,
    #[serde(default)]
    pub hiring_active: bool,
    #[serde(default)]
    pub hiring_roles: Vec<String>,
    #[serde(default)]
    pub icebreaker: String,
    #[serde(default)]
    pub compliance_score: u8,
    #[serde(default)]
    pub compliance_status: String,
    #[serde(default)]
    pub privacy_policy_url: String,
    #[serde(default)]
    pub mailbox_tier: String,
    #[serde(default)]
    pub seniority: String,
    #[serde(default)]
    pub department: String,
    #[serde(default)]
    pub phone_line_category: String,
    #[serde(default)]
    pub cloud_provider: String,
    #[serde(default)]
    pub estimated_headcount: String,
    #[serde(default)]
    pub industry_vertical: String,
    #[serde(default)]
    pub social_proof_clients: Vec<String>,
    #[serde(default)]
    pub certifications: Vec<String>,
    #[serde(default)]
    pub role_risk_tier: String,
    #[serde(default)]
    pub tech_maturity_tier: String,
    #[serde(default)]
    pub cta_friction_tier: String,
    #[serde(default)]
    pub spam_matrix_penalty: u8,
    #[serde(default)]
    pub title_seniority_tier: String,
    #[serde(default)]
    pub mx_redundancy_grade: String,
    #[serde(default)]
    pub web_framework_tier: String,
    #[serde(default)]
    pub mailbox_archetype: String,
}

impl ContactRecord {
    pub fn new(
        name: &str,
        email: &str,
        company: &str,
        role: &str,
        city: &str,
        stack: &str,
        phone: &str,
        telegram: &str,
        source: &str,
        url: &str,
        score: LeadScore,
        mx_valid: bool,
        mx_record: &str,
    ) -> Self {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = format!(
            "{}_{}_{}",
            email.replace('@', "_at_"),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
            SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        Self {
            evidence: Vec::new(),
            id,
            selected: true,
            name: name.trim().to_string(),
            email: email.trim().to_lowercase(),
            company: company.trim().to_string(),
            role: role.trim().to_string(),
            city: city.trim().to_string(),
            stack: stack.trim().to_string(),
            phone: phone.trim().to_string(),
            telegram: telegram.trim().to_string(),
            source: source.trim().to_string(),
            url: url.trim().to_string(),
            status: SendStatus::Pending,
            score,
            mx_valid,
            mx_record: mx_record.trim().to_string(),
            created_at: now,
            follow_up_count: 0,
            last_sent_at: None,
            last_message_id: None,
            notes: "".to_string(),
            vk_url: "".to_string(),
            whatsapp: "".to_string(),
            tags: Vec::new(),
            rcpt_verified: false,
            is_catch_all: false,
            is_disposable: false,
            gravatar_found: false,
            avatar_url: "".to_string(),
            mail_provider: "".to_string(),
            is_role_account: false,
            lead_score: 50,
            tech_stack: Vec::new(),
            analytics_ids: Vec::new(),
            domain_age_days: None,
            domain_registrar: "".to_string(),
            domain_pattern: "".to_string(),
            suggested_email_fix: None,
            company_description: "".to_string(),
            legal_name: "".to_string(),
            address: "".to_string(),
            b2b_tier: "Tier 2".to_string(),
            phone_carrier: "".to_string(),
            phone_region: "".to_string(),
            phone_type: "".to_string(),
            whatsapp_url: "".to_string(),
            telegram_url: "".to_string(),
            github_url: "".to_string(),
            github_bio: "".to_string(),
            inn: "".to_string(),
            ogrn: "".to_string(),
            vat_id: "".to_string(),
            hiring_active: false,
            hiring_roles: Vec::new(),
            icebreaker: "".to_string(),
            compliance_score: 0,
            compliance_status: "".to_string(),
            privacy_policy_url: "".to_string(),
            mailbox_tier: "Unknown".to_string(),
            seniority: "Unknown".to_string(),
            department: "General".to_string(),
            phone_line_category: "Unknown".to_string(),
            cloud_provider: "".to_string(),
            estimated_headcount: "Unknown".to_string(),
            industry_vertical: "General".to_string(),
            social_proof_clients: Vec::new(),
            certifications: Vec::new(),
            role_risk_tier: "Unknown".to_string(),
            tech_maturity_tier: "Unknown".to_string(),
            cta_friction_tier: "Unknown".to_string(),
            spam_matrix_penalty: 0,
            title_seniority_tier: "Unknown".to_string(),
            mx_redundancy_grade: "Unknown".to_string(),
            web_framework_tier: "Unknown".to_string(),
            mailbox_archetype: "Unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DnsblResult {
    pub domain_or_ip: String,
    pub listed: bool,
    pub blacklists_checked: usize,
    pub detections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainAuditResult {
    pub domain: String,
    pub has_mx: bool,
    pub mx_record: String,
    pub has_spf: bool,
    pub spf_record: String,
    pub has_dmarc: bool,
    pub dmarc_record: String,
    pub score: u32,
    #[serde(default)]
    pub grade: String,
    #[serde(default)]
    pub has_dkim: bool,
    #[serde(default)]
    pub dkim_selector: String,
    #[serde(default)]
    pub dkim_bits: u32,
    #[serde(default)]
    pub dkim_strength: String,
    #[serde(default)]
    pub has_bimi: bool,
    #[serde(default)]
    pub bimi_svg_url: String,
    #[serde(default)]
    pub bimi_vmc_cert: String,
    #[serde(default)]
    pub has_mta_sts: bool,
    #[serde(default)]
    pub mta_sts_mode: String,
    #[serde(default)]
    pub has_tls_rpt: bool,
    #[serde(default)]
    pub tls_rpt_rua: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityPostureReport {
    pub url: String,
    pub score: u8,
    pub grade: String,
    pub has_hsts: bool,
    pub hsts_max_age: u64,
    pub has_csp: bool,
    pub has_xfo: bool,
    pub has_nosniff: bool,
    pub has_referrer_policy: bool,
    pub is_enterprise_ready: bool,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BounceRiskForecast {
    pub total_contacts: usize,
    pub predicted_bounces: usize,
    pub predicted_bounce_rate: f32,
    pub safety_verdict: String,
    pub high_risk_count: usize,
    pub moderate_risk_count: usize,
    pub safe_count: usize,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WarmupDaySchedule {
    pub day: usize,
    pub daily_limit: usize,
    pub cumulative_sent: usize,
    pub delay_between_emails_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutreachPsychologyReport {
    pub word_count: usize,
    pub reading_time_sec: u32,
    pub sentence_count: usize,
    pub question_count: usize,
    pub you_to_i_ratio: f32,
    pub subject_words: usize,
    pub readability_score: u8,
    pub grade: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MxFingerprint {
    pub host: String,
    pub provider_name: String,
    pub gateway_type: String,
    pub strictness_level: String,
    pub deliverability_advice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MailboxTier {
    #[default]
    CorporateB2b,
    FreeWebmail,
    Disposable,
}

impl std::fmt::Display for MailboxTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailboxTier::CorporateB2b => write!(f, "Corporate B2B"),
            MailboxTier::FreeWebmail => write!(f, "Free webmail"),
            MailboxTier::Disposable => write!(f, "Disposable / risk"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplianceProfile {
    pub url: String,
    pub has_privacy_policy: bool,
    pub privacy_policy_url: String,
    pub law_152_fz_compliant: bool,
    pub gdpr_compliant: bool,
    pub has_cookie_notice: bool,
    pub compliance_score: u8,
    pub compliance_status: String,
    pub detected_markers: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MtaStsAudit {
    pub domain: String,
    pub dns_txt_valid: bool,
    pub txt_record: String,
    pub policy_fetched: bool,
    pub mode: String,
    pub mx_patterns: Vec<String>,
    pub max_age: u64,
    pub has_tls_rpt: bool,
    pub tls_rpt_record: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BimiAudit {
    pub domain: String,
    pub valid_bimi: bool,
    pub raw_record: String,
    pub svg_url: String,
    pub vmc_cert_url: String,
    pub has_vmc: bool,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpamCheckResult {
    pub score: u32,
    pub trigger_words: Vec<String>,
    pub excessive_caps: bool,
    pub excessive_punctuation: bool,
    pub has_personalization: bool,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SuppressionList {
    #[serde(default)]
    pub emails: Vec<String>,
    #[serde(default)]
    pub domains: Vec<String>,
}

impl SuppressionList {
    pub fn is_suppressed(&self, email: &str) -> bool {
        let clean = email.trim().to_lowercase();
        if self.emails.iter().any(|e| e.to_lowercase() == clean) {
            return true;
        }
        if let Some((_, domain)) = clean.split_once('@') {
            if self.domains.iter().any(|d| d.to_lowercase() == domain) {
                return true;
            }
        }
        false
    }

    pub fn add_email(&mut self, email: &str) {
        let clean = email.trim().to_lowercase();
        if !clean.is_empty() && !self.emails.contains(&clean) {
            self.emails.push(clean);
        }
    }

    pub fn add_domain(&mut self, domain: &str) {
        let clean = domain.trim().to_lowercase();
        if !clean.is_empty() && !self.domains.contains(&clean) {
            self.domains.push(clean);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    pub city: String,
    pub stack: String,
    pub role_filter: String,
    pub search_linkedin: bool,
    pub search_remotive: bool,
    pub search_arbeitnow: bool,
    pub search_remoteok: bool,
    pub search_jobicy: bool,
    pub search_hn: bool,
    pub search_telegram: bool,
    pub search_google_maps: bool,
    pub search_web: bool,
    #[serde(default = "default_true")]
    pub search_gis: bool,
    pub crawl_websites: bool,
    pub verify_mx: bool,
    pub max_items: usize,
}

fn default_true() -> bool {
    true
}

impl Default for SearchFilter {
    fn default() -> Self {
        Self {
            city: "Krasnoyarsk".to_string(),
            stack: "Photography".to_string(),
            role_filter: "All".to_string(),
            search_linkedin: false,
            search_remotive: false,
            search_arbeitnow: false,
            search_remoteok: false,
            search_jobicy: false,
            search_hn: false,
            search_telegram: false,
            search_google_maps: false,
            search_web: true,
            search_gis: true,
            crawl_websites: true,
            verify_mx: true,
            max_items: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub direct_mx: bool,
    #[serde(default)]
    pub helo_domain: String,
    #[serde(default)]
    pub dkim_selector: String,
    #[serde(default)]
    pub dkim_key_path: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub from_email: String,
    pub sender_name: String,
    #[serde(default)]
    pub reply_to: String,
    pub use_ssl: bool,
    pub delay_seconds: u64,
    #[serde(default = "default_true")]
    pub random_delay: bool,
    #[serde(default = "default_daily_limit")]
    pub daily_limit: usize,
    #[serde(default)]
    pub sent_today: usize,
    #[serde(default)]
    pub last_sent_day: String,
    #[serde(default = "default_true")]
    // Legacy setting name; outbound mail uses an RFC 2369 mailto unsubscribe header.
    pub enable_rfc8058: bool,
    #[serde(default)]
    pub warmup_mode: bool,
    #[serde(default = "default_warmup_step")]
    pub warmup_step: usize,
    #[serde(default)]
    pub request_read_receipt: bool,
}

fn default_warmup_step() -> usize {
    5
}

fn default_daily_limit() -> usize {
    50
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            direct_mx: false,
            helo_domain: "".to_string(),
            dkim_selector: String::new(),
            dkim_key_path: String::new(),
            host: "smtp.gmail.com".to_string(),
            port: 587,
            username: String::new(),
            password: "".to_string(),
            from_email: String::new(),
            sender_name: String::new(),
            reply_to: String::new(),
            use_ssl: false,
            delay_seconds: 5,
            random_delay: true,
            daily_limit: 50,
            sent_today: 0,
            last_sent_day: "".to_string(),
            enable_rfc8058: true,
            warmup_mode: false,
            warmup_step: 5,
            request_read_receipt: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    pub template_name: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub follow_up_1: Option<String>,
    #[serde(default)]
    pub follow_up_2: Option<String>,
}

impl Default for EmailTemplate {
    fn default() -> Self {
        Self {
            template_name: "Business introduction".to_string(),
            subject: "Question for {company}".to_string(),
            body: "Hi {name},\n\nI would like to discuss a possible collaboration with {company}. Would a short introduction be useful?\n\nBest regards,\n{sender_name}".to_string(),
            follow_up_1: None,
            follow_up_2: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: &str, message: &str) -> Self {
        Self {
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
            level: level.to_string(),
            message: message.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpamCheckReport {
    pub spam_score: u8,
    pub trigger_words_found: Vec<String>,
    pub caps_ratio: f32,
    pub exclamation_count: usize,
    pub is_high_risk: bool,
    pub advice: Vec<String>,
    pub mailru_deliverability: String,
    pub gmail_risk_level: String,
    pub gmail_warning_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EcommerceStackProfile {
    pub payment_gateways: Vec<String>,
    pub live_chats: Vec<String>,
    pub store_engines: Vec<String>,
    pub is_commercial: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketingIntentProfile {
    pub ad_pixels: Vec<String>,
    pub automation_platforms: Vec<String>,
    pub intent_score: u8,
    pub is_high_intent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MailInfraSubdomain {
    pub subdomain: String,
    pub resolved_record: String,
    pub service_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinkAuditReport {
    pub link_count: usize,
    pub link_density_pct: f32,
    pub has_suspicious_redirects: bool,
    pub is_safe_for_inbox: bool,
    pub advice: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoipClassification {
    pub phone_number: String,
    pub is_voip_cloud_pbx: bool,
    pub is_real_mobile: bool,
    pub carrier_category: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloudInfraProfile {
    pub provider: String,
    pub is_cloud_hosted: bool,
    pub detected_markers: Vec<String>,
    pub cdn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubjectOptimizationReport {
    pub original_subject: String,
    pub char_count: usize,
    pub word_count: usize,
    pub is_mobile_friendly: bool,
    pub has_spam_words: bool,
    pub lowercase_variant: String,
    pub advice: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpamTrapAuditResult {
    pub email: String,
    pub is_spam_trap: bool,
    pub trap_category: String,
    pub risk_level: String,
    pub action_recommended: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SenderHealthReport {
    pub domain: String,
    pub score: u8,
    pub grade: String,
    pub components: Vec<String>,
    pub critical_issues: Vec<String>,
}
