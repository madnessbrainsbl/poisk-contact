use regex::Regex;
use std::sync::LazyLock;

static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)[a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,}").unwrap());

static MAILTO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)mailto:([a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,})").unwrap());

static TG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:t\.me/|telegram\.me/|(?:telegram|телеграм)\s*[:\-]?\s*@)([a-zA-Z0-9_]{4,32})",
    )
    .unwrap()
});

static PHONE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\+?\d[\d ()-]*\d").unwrap());
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").unwrap());

const BAD_PARTS: &[&str] = &[
    "example.",
    "example@",
    "noreply",
    "no-reply",
    "donotreply",
    ".png",
    ".jpg",
    ".jpeg",
    ".webp",
    ".gif",
    ".svg",
    ".css",
    ".js",
    ".json",
    ".woff",
    "sentry",
    "wixpress",
    "godaddy",
    "cloudflare",
    "schema.org",
    "w3.org",
    "test.com",
    "localhost",
    "domain.com",
    "yourdomain",
    "privacy@",
    "dpo@",
    "dataprotection@",
    "abuse@",
    "postmaster@",
    "hostmaster@",
    "security@",
    "support@sentry",
    "satoshi@",
    "test@",
    "dummy@",
    "sample@",
    "john.doe@",
    "yourname@",
    "username@",
    "email.com",
    "sample.com",
    "dummy.com",
    "yourcompany.com",
];

const TG_IGNORED: &[&str] = &[
    "share",
    "joinchat",
    "channel",
    "group",
    "contact",
    "contacts",
    "telegram",
    "support",
    "help",
    "admin",
    "yandex",
    "google",
    "mail",
    "media",
    "keyframes",
    "context",
    "function",
    "import",
    "charset",
    "namespace",
    "supports",
    "document",
    "page",
    "font-face",
    "sentry",
    "adobe",
    "emotion",
    "babel",
    "react",
    "vue",
    "angular",
    "webpack",
    "typescript",
    "types",
    "rollup",
    "postcss",
    "eslint",
    "prettier",
    "vite",
];

pub fn unescape_html_entities(input: &str) -> String {
    let mut text = input
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&#039;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&#8212;", "—")
        .replace("&#8211;", "–")
        .replace("&#64;", "@")
        .replace("&#x40;", "@")
        .replace("%40", "@")
        .replace("&#46;", ".")
        .replace("&#x2e;", ".")
        .replace("%2e", ".");

    let at_re = Regex::new(r"(?i)\[\s*at\s*\]|\(\s*at\s*\)|\{\s*at\s*\}").unwrap();
    let dot_re = Regex::new(r"(?i)\[\s*dot\s*\]|\(\s*dot\s*\)|\{\s*dot\s*\}").unwrap();
    text = at_re.replace_all(&text, "@").to_string();
    text = dot_re.replace_all(&text, ".").to_string();
    text
}

pub fn clean_html_and_entities(input: &str) -> String {
    unescape_html_entities(input)
}

pub fn strip_html_boilerplate(html: &str) -> String {
    static SCRIPT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap());
    static STYLE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap());
    static SVG_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<svg[^>]*>.*?</svg>").unwrap());
    static NOSCRIPT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<noscript[^>]*>.*?</noscript>").unwrap());
    static COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<!--.*?-->").unwrap());

    let s = SCRIPT_RE.replace_all(html, " ");
    let s = STYLE_RE.replace_all(&s, " ");
    let s = SVG_RE.replace_all(&s, " ");
    let s = NOSCRIPT_RE.replace_all(&s, " ");
    let s = COMMENT_RE.replace_all(&s, " ");
    s.into_owned()
}

pub fn extract_emails(raw_text: &str) -> Vec<String> {
    if raw_text.is_empty() {
        return Vec::new();
    }
    let stripped = strip_html_boilerplate(raw_text);
    let cleaned = clean_html_and_entities(&stripped);
    let mut found = Vec::new();

    for cap in MAILTO_RE.captures_iter(&cleaned) {
        if let Some(m) = cap.get(1) {
            let email = m.as_str().trim_matches('.').to_lowercase();
            if is_valid_email(&email) && !found.contains(&email) {
                found.push(email);
            }
        }
    }

    let visible = clean_html_and_entities(&TAG_RE.replace_all(&stripped, " "));
    for cap in EMAIL_RE.captures_iter(&visible) {
        if let Some(m) = cap.get(0) {
            let email = m.as_str().trim_matches('.').to_lowercase();
            if is_valid_email(&email) && !found.contains(&email) {
                found.push(email);
            }
        }
    }

    found
}

pub fn is_valid_email(email: &str) -> bool {
    if email.len() < 6 || email.len() > 120 {
        return false;
    }
    if !email.contains('@') {
        return false;
    }
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (user, domain) = (parts[0], parts[1]);
    if user.is_empty() || domain.is_empty() || !domain.contains('.') {
        return false;
    }
    for bad in BAD_PARTS {
        if email.contains(bad) {
            return false;
        }
    }
    true
}

pub fn normalize_phone_number(phone: &str) -> String {
    static OPTIONAL_TRUNK_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(\+\d{1,3}\s*)\(0\)").unwrap());
    let phone = phone.trim();
    let phone = if phone.starts_with("+39") {
        std::borrow::Cow::Borrowed(phone)
    } else {
        OPTIONAL_TRUNK_RE.replace(phone, "$1")
    };
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    if phone.starts_with('+') && !digits.is_empty() {
        if digits.len() == 12 && digits.starts_with("610") {
            return format!("+61{}", &digits[3..]);
        }
        return format!("+{digits}");
    }
    if digits.len() == 11 {
        if digits.starts_with('8') || digits.starts_with('7') {
            format!("+7{}", &digits[1..])
        } else if phone.trim().starts_with('+') {
            format!("+{}", digits)
        } else {
            digits
        }
    } else if !digits.is_empty() {
        if phone.starts_with('+') {
            format!("+{}", digits)
        } else {
            digits
        }
    } else {
        phone.trim().to_string()
    }
}

pub fn extract_telegrams(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    let stripped = strip_html_boilerplate(text);
    for cap in TG_RE.captures_iter(&stripped) {
        if let Some(m) = cap.get(1) {
            let handle = m.as_str().to_lowercase();
            if !TG_IGNORED.contains(&handle.as_str()) && !results.contains(&format!("@{}", handle))
            {
                results.push(format!("@{}", handle));
            }
        }
    }
    results
}

static TEL_HREF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)href=["']tel:([^"']+)["']"#).unwrap());

pub(super) fn is_plausible_phone(phone: &str) -> bool {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    (7..=15).contains(&digits.len())
        && !digits.ends_with("123456789")
        && !digits.ends_with("1234567890")
        && !digits
            .as_bytes()
            .iter()
            .all(|digit| *digit == digits.as_bytes()[0])
}

pub fn extract_phones(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    let stripped = strip_html_boilerplate(text);

    // 1. First priority: explicit href="tel:..."
    for cap in TEL_HREF_RE.captures_iter(&stripped) {
        if let Some(m) = cap.get(1) {
            let raw = m.as_str().trim();
            let norm = normalize_phone_number(raw);
            if is_plausible_phone(&norm) && !results.contains(&norm) {
                results.push(norm);
            }
        }
    }

    // 2. Second priority: formatted numbers in stripped body
    let visible = unescape_html_entities(&TAG_RE.replace_all(&stripped, " "));
    for cap in PHONE_RE.captures_iter(&visible) {
        if let Some(m) = cap.get(0) {
            let phone = m.as_str().trim();
            // Discard pure numeric strings without phone separators unless starting with +
            let has_separators = phone.contains(' ')
                || phone.contains('-')
                || phone.contains('(')
                || phone.starts_with('+');
            if !has_separators {
                continue;
            }
            let digits_count = phone.chars().filter(|c| c.is_ascii_digit()).count();
            if (10..=15).contains(&digits_count) {
                let norm = normalize_phone_number(&phone);
                if is_plausible_phone(&norm) && !results.contains(&norm) {
                    results.push(norm);
                }
            }
        }
    }
    results
}

pub(super) fn phones_for_country(text: &str, country: &str) -> Vec<String> {
    let mut results = extract_phones(text);
    if country == "RU" {
        results.retain(|phone| {
            phone.starts_with('+') || phone.len() != 10 || phone.starts_with(['3', '4', '8', '9'])
        });
        for phone in &mut results {
            if phone.len() == 10 && phone.chars().all(|c| c.is_ascii_digit()) {
                *phone = format!("+7{phone}");
            }
        }
        return results;
    }
    if !matches!(country, "CA" | "US") {
        return results;
    }
    let stripped = strip_html_boilerplate(text);
    // Some sites put a national NANP number after '+' in tel:, while the label correctly
    // prints the national number. Preserve the printed number instead of inventing +90.
    static TEL_LABEL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?is)<a\b[^>]*href=["']tel:\+(\d{10})["'][^>]*>(.*?)</a>"#).unwrap()
    });
    for cap in TEL_LABEL_RE.captures_iter(&stripped) {
        let label = unescape_html_entities(&TAG_RE.replace_all(&cap[2], " "));
        let national: String = label.chars().filter(|c| c.is_ascii_digit()).collect();
        if national == cap[1] && !label.trim().starts_with('+') {
            for phone in &mut results {
                if *phone == format!("+{national}") {
                    *phone = national.clone();
                }
            }
        }
    }

    results
}

static VK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:vk\.com|vkontakte\.ru)/([a-zA-Z0-9_\.]{3,40})").unwrap());

const VK_IGNORED: &[&str] = &[
    "share",
    "share.php",
    "away",
    "away.php",
    "feed",
    "im",
    "wall",
    "video",
    "market",
    "app",
    "dev",
    "support",
    "help",
    "privacy",
    "terms",
    "widget",
    "al_im",
];

pub fn extract_vk_links(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    for cap in VK_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let slug = m.as_str().trim_matches('/').to_lowercase();
            if !VK_IGNORED.contains(&slug.as_str()) && !results.contains(&slug) {
                results.push(format!("https://vk.com/{}", slug));
            }
        }
    }
    results
}

static WA_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:wa\.me/|api\.whatsapp\.com/send\?(?:[^&]*&)?phone=)(\+?\d{10,15})").unwrap()
});

pub fn extract_whatsapp_links(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    for cap in WA_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let digits: String = m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                let link = format!("https://wa.me/{}", digits);
                if !results.contains(&link) {
                    results.push(link);
                }
            }
        }
    }
    results
}

pub fn detect_industry_tags(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tags = Vec::new();

    if lower.contains("шоурум")
        || lower.contains("бренд")
        || lower.contains("одежд")
        || lower.contains("fashion")
        || lower.contains("лукбук")
        || lower.contains("коллекци")
    {
        tags.push("Fashion / Бренд".to_string());
    }
    if lower.contains("ресторан")
        || lower.contains("кафе")
        || lower.contains("бар")
        || lower.contains("гастро")
        || lower.contains("меню")
        || lower.contains("блюд")
    {
        tags.push("HoReCa / Меню".to_string());
    }
    if lower.contains("фотостуд")
        || lower.contains("локаци")
        || lower.contains("аренда студии")
        || lower.contains("съемочн")
    {
        tags.push("Фотостудия".to_string());
    }
    if lower.contains("модел") || lower.contains("кастинг") || lower.contains("agency")
    {
        tags.push("Модельное агентство".to_string());
    }
    if lower.contains("продакшн")
        || lower.contains("видеоролик")
        || lower.contains("кампейн")
        || lower.contains("реклам")
    {
        tags.push("Creative Production".to_string());
    }

    tags
}

const DISPOSABLE_DOMAINS: &[&str] = &[
    "temp-mail.org",
    "tempmail.com",
    "10minutemail.com",
    "mailinator.com",
    "guerrillamail.com",
    "sharklasers.com",
    "yopmail.com",
    "trashmail.com",
    "dispostable.com",
    "getairmail.com",
    "mohmal.com",
    "crazymailing.com",
    "fakeinbox.com",
    "generator.email",
    "dropmail.me",
    "fakemailgenerator.com",
    "throwawaymail.com",
    "burnermail.io",
    "maildrop.cc",
    "inboxkitten.com",
    "emailondeck.com",
    "mintemail.com",
    "mytemp.email",
    "tempail.com",
    "nada.ltd",
    "getnada.com",
    "inboxbear.com",
    "mailsac.com",
    "tempm.com",
    "mohmal.in",
    "incognitomail.org",
    "temporary-mail.net",
    "trashmail.net",
    "trashmail.me",
    "disposablemail.com",
    "mailhero.io",
    "crazymail.net",
    "jetable.org",
    "trash-mail.com",
    "mytempmail.com",
    "filzmail.com",
    "fastmail.fm",
    "spamgourmet.com",
    "tempmailaddress.com",
    "burnermail.com",
    "harakirimail.com",
    "sharklasers.net",
    "guerrillamail.biz",
    "guerrillamail.net",
    "guerrillamail.org",
    "grr.la",
    "guerrillamailblock.com",
    "pokemail.net",
    "spam4.me",
    "bccto.me",
    "chacuo.net",
    "0-mail.com",
    "10mail.org",
    "20minutemail.com",
    "anonbox.net",
    "anonymbox.com",
    "antichef.com",
    "antichef.net",
    "binkmail.com",
    "bobmail.info",
    "buffemail.com",
    "bugmenot.com",
    "bupmail.com",
    "buyusedlibrarybooks.org",
    "byom.de",
    "cmail.club",
    "cool.fr.nf",
    "courrieltemporaire.com",
    "cust.in",
    "dacoolest.com",
    "dandikmail.com",
    "dayrep.com",
    "deadaddress.com",
    "deadfake.com",
    "despam.it",
    "discard.email",
    "discardmail.com",
    "discardmail.de",
    "disposable.com",
    "dispostable.net",
    "dodgit.com",
    "drdrb.net",
    "dump-email.info",
    "e4ward.com",
    "email-temporaire.fr",
    "emaildienst.de",
    "emailfake.com",
    "emailgenerator.net",
    "emailmiser.com",
    "emailsensei.com",
    "emailtemporario.com.br",
    "emkei.cz",
    "evopoint.net",
    "explodemail.com",
    "eyepaste.com",
    "fakebox.org",
    "fakemail.net",
    "fakemailgenerator.net",
    "fastcheetah.com",
    "fmail.pw",
];

pub const FREE_MAIL_DOMAINS: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "yandex.ru",
    "yandex.com",
    "ya.ru",
    "mail.ru",
    "bk.ru",
    "inbox.ru",
    "list.ru",
    "internet.ru",
    "rambler.ru",
    "lenta.ru",
    "autorambler.ru",
    "myrambler.ru",
    "ro.ru",
    "yahoo.com",
    "yahoo.co.uk",
    "outlook.com",
    "hotmail.com",
    "live.com",
    "msn.com",
    "icloud.com",
    "me.com",
    "mac.com",
    "proton.me",
    "protonmail.com",
    "zoho.com",
    "aol.com",
    "gmx.com",
    "gmx.de",
    "web.de",
    "mail.com",
];

pub fn is_disposable_email(email: &str) -> bool {
    if let Some((_, domain)) = email.split_once('@') {
        let clean = domain.trim().to_lowercase();
        DISPOSABLE_DOMAINS
            .iter()
            .any(|d| clean == *d || clean.ends_with(&format!(".{}", d)))
    } else {
        false
    }
}

pub fn is_free_mail_domain(domain: &str) -> bool {
    let clean = domain.trim().to_lowercase();
    FREE_MAIL_DOMAINS
        .iter()
        .any(|d| clean == *d || clean.ends_with(&format!(".{}", d)))
}

pub fn is_free_email(email: &str) -> bool {
    if let Some((_, domain)) = email.split_once('@') {
        is_free_mail_domain(domain)
    } else {
        false
    }
}

pub fn sanitize_email(email: &str) -> String {
    let clean = email.trim().to_lowercase();
    if let Some((user, domain)) = clean.split_once('@') {
        let normalized_domain = domain.trim();
        // Remove subaddressing plus-tags (+tag)
        let normalized_user = if let Some((base_user, _)) = user.split_once('+') {
            base_user.trim()
        } else {
            user.trim()
        };
        format!("{}@{}", normalized_user, normalized_domain)
    } else {
        clean
    }
}

pub fn calculate_b2b_tier(contact: &crate::models::ContactRecord) -> String {
    let is_free = is_free_email(&contact.email);
    let has_corp_mx = contact.mx_valid && !is_free;
    let has_rich_tech = !contact.tech_stack.is_empty();
    let has_mature_domain = contact.domain_age_days.map(|d| d > 365).unwrap_or(false);

    if has_corp_mx && (has_rich_tech || has_mature_domain || contact.lead_score >= 80) {
        "Tier 1".to_string()
    } else if !contact.email.is_empty() && (!is_free || !contact.url.is_empty()) {
        "Tier 2".to_string()
    } else {
        "Tier 3".to_string()
    }
}

pub fn detect_mail_provider(mx_record: &str) -> String {
    let lower = mx_record.to_lowercase();
    if lower.contains("yandex.net") || lower.contains("yandex.ru") || lower.contains("mx.yandex") {
        "Яндекс 360 / Корп. Почта".to_string()
    } else if lower.contains("mail.ru")
        || lower.contains("mxs.mail.ru")
        || lower.contains("biz.mail.ru")
    {
        "VK WorkMail / Mail.ru для бизнеса".to_string()
    } else if lower.contains("google.com")
        || lower.contains("googlemail.com")
        || lower.contains("l.google.com")
    {
        "Google Workspace".to_string()
    } else if lower.contains("outlook.com")
        || lower.contains("protection.outlook")
        || lower.contains("office365")
    {
        "Microsoft 365 / Exchange".to_string()
    } else if lower.contains("timeweb.ru") {
        "Timeweb Почта".to_string()
    } else if lower.contains("beget.com") || lower.contains("beget.ru") {
        "Beget Почта".to_string()
    } else if lower.contains("reg.ru") {
        "Reg.ru Почта".to_string()
    } else if lower.contains("nic.ru") {
        "RU-CENTER Почта".to_string()
    } else if lower.contains("zoho") {
        "Zoho Mail".to_string()
    } else if lower.contains("protonmail") || lower.contains("proton.me") {
        "ProtonMail Business".to_string()
    } else if !lower.is_empty() {
        "Собственный MX сервер".to_string()
    } else {
        "—".to_string()
    }
}

pub fn md5_hex(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xefcdab89;
    let mut h2: u32 = 0x98badcfe;
    let mut h3: u32 = 0x10325476;

    let k: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2d7d83b1,
        0xeb86d391,
    ];

    let s: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    let bit_len = (bytes.len() as u64) * 8;
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_le_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut m = [0u32; 16];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            m[i] = u32::from_le_bytes(word.try_into().unwrap());
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | ((!b) & d), i),
                16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | (!d)), (7 * i) % 16),
            };

            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                (a.wrapping_add(f).wrapping_add(k[i]).wrapping_add(m[g])).rotate_left(s[i]),
            );
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
    }

    format!(
        "{:08x}{:08x}{:08x}{:08x}",
        h0.to_le(),
        h1.to_le(),
        h2.to_le(),
        h3.to_le()
    )
}

pub async fn check_gravatar_profile(client: &reqwest::Client, email: &str) -> Option<String> {
    let clean = email.trim().to_lowercase();
    if clean.is_empty() || !clean.contains('@') {
        return None;
    }
    let hash = md5_hex(&clean);
    let avatar_url = format!("https://www.gravatar.com/avatar/{}?d=404", hash);
    let res = client
        .head(&avatar_url)
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await;

    if let Ok(resp) = res {
        if resp.status().is_success() {
            return Some(format!("https://www.gravatar.com/avatar/{}?s=200", hash));
        }
    }
    None
}

pub const ROLE_PREFIXES: &[&str] = &[
    "admin",
    "administrator",
    "support",
    "help",
    "info",
    "information",
    "sales",
    "contact",
    "contacts",
    "hello",
    "hi",
    "press",
    "media",
    "marketing",
    "team",
    "jobs",
    "careers",
    "career",
    "hr",
    "work",
    "office",
    "mail",
    "postmaster",
    "hostmaster",
    "abuse",
    "security",
    "privacy",
    "legal",
    "billing",
    "invoice",
    "finance",
    "accounting",
    "general",
    "service",
    "inquiry",
    "inquiries",
    "feedback",
    "dev",
    "development",
    "tech",
    "staff",
    "reception",
    "manager",
    "request",
];

pub fn is_role_based_email(email: &str) -> bool {
    if let Some((local, _)) = email.split_once('@') {
        let clean_local = local.trim().to_lowercase();
        let stem = clean_local
            .split(['.', '-', '_', '+'])
            .next()
            .unwrap_or(&clean_local);
        ROLE_PREFIXES.contains(&clean_local.as_str()) || ROLE_PREFIXES.contains(&stem)
    } else {
        false
    }
}

pub fn transliterate_to_latin(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch.to_lowercase().next().unwrap_or(ch) {
            'а' => result.push('a'),
            'б' => result.push('b'),
            'в' => result.push('v'),
            'г' => result.push('g'),
            'д' => result.push('d'),
            'е' | 'ё' | 'э' => result.push('e'),
            'ж' => result.push_str("zh"),
            'з' => result.push('z'),
            'и' => result.push('i'),
            'й' | 'ы' => result.push('y'),
            'к' => result.push('k'),
            'л' => result.push('l'),
            'м' => result.push('m'),
            'н' => result.push('n'),
            'о' => result.push('o'),
            'п' => result.push('p'),
            'р' => result.push('r'),
            'с' => result.push('s'),
            'т' => result.push('t'),
            'у' => result.push('u'),
            'ф' => result.push('f'),
            'х' => result.push_str("kh"),
            'ц' => result.push_str("ts"),
            'ч' => result.push_str("ch"),
            'ш' => result.push_str("sh"),
            'щ' => result.push_str("shch"),
            'ю' => result.push_str("yu"),
            'я' => result.push_str("ya"),
            other if other.is_ascii_alphanumeric() => result.push(other),
            _ => {}
        }
    }
    result
}

pub fn generate_email_permutations(first: &str, last: &str, domain: &str) -> Vec<String> {
    let clean_domain = domain
        .trim()
        .trim_start_matches('@')
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_lowercase();
    if clean_domain.is_empty() {
        return Vec::new();
    }

    let f = transliterate_to_latin(first.trim());
    let l = transliterate_to_latin(last.trim());

    if f.is_empty() && l.is_empty() {
        return Vec::new();
    }

    let mut permutations = Vec::new();

    if !f.is_empty() && !l.is_empty() {
        let f_init = f.chars().next().map(|c| c.to_string()).unwrap_or_default();
        let l_init = l.chars().next().map(|c| c.to_string()).unwrap_or_default();

        permutations.push(format!("{}.{}@{}", f, l, clean_domain));
        permutations.push(format!("{}@{}", f, clean_domain));
        permutations.push(format!("{}.{}@{}", f_init, l, clean_domain));
        permutations.push(format!("{}{}@{}", f, l, clean_domain));
        permutations.push(format!("{}{}@{}", f_init, l, clean_domain));
        permutations.push(format!("{}{}@{}", f, l_init, clean_domain));
        permutations.push(format!("{}_{}@{}", f, l, clean_domain));
        permutations.push(format!("{}.{}@{}", l, f, clean_domain));
        permutations.push(format!("{}@{}", l, clean_domain));
        permutations.push(format!("{}.{}@{}", l, f_init, clean_domain));
    } else if !f.is_empty() {
        permutations.push(format!("{}@{}", f, clean_domain));
    } else {
        permutations.push(format!("{}@{}", l, clean_domain));
    }

    let mut unique = Vec::new();
    for p in permutations {
        if !unique.contains(&p) {
            unique.push(p);
        }
    }
    unique
}

pub fn calculate_lead_score(contact: &crate::models::ContactRecord) -> u8 {
    let mut score = 0u8;

    if contact.email.contains('@') && contact.email.contains('.') {
        score += 15;
    }
    if contact.mx_valid {
        score += 20;
    }
    if !contact.is_disposable {
        score += 15;
    }
    if !contact.is_catch_all {
        score += 15;
    }
    if !contact.is_role_account {
        score += 15;
    }
    if contact.gravatar_found || !contact.avatar_url.is_empty() {
        score += 10;
    }
    if !contact.phone.is_empty() || !contact.telegram.is_empty() {
        score += 10;
    }

    score.min(100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_domains_are_not_telegram_accounts() {
        assert_eq!(
            extract_telegrams(
                "info@berlin-smile.de mail@unicorn.studio Instagram: @instagram_contact Telegram: @real_contact https://t.me/clinic_team"
            ),
            vec!["@real_contact", "@clinic_team"]
        );
    }

    #[test]
    fn preserves_international_and_national_phone_prefixes() {
        assert_eq!(normalize_phone_number("020 7221 1680"), "02072211680");
        assert_eq!(normalize_phone_number("+44 20 7380 0034"), "+442073800034");
        assert_eq!(normalize_phone_number("030 8876 2090"), "03088762090");
        assert_eq!(normalize_phone_number("8 (391) 200-11-22"), "+73912001122");
        assert_eq!(
            extract_phones("Call 020 7631 3276 or +49 30 959999410"),
            vec!["02076313276", "+4930959999410"]
        );
        assert!(extract_phones(r#"<div data-id="391-200-11-22"></div><script><a href="tel:02076313276">Hidden</a></script>"#).is_empty());
    }

    #[test]
    fn removes_optional_domestic_zero_only_in_international_notation() {
        assert_eq!(normalize_phone_number("+49 (0)30 3029695"), "+49303029695");
        assert_eq!(
            normalize_phone_number("+44 (0)20 7608 0806"),
            "+442076080806"
        );
        assert_eq!(normalize_phone_number("020 7608 0806"), "02076080806");
        assert_eq!(normalize_phone_number("+39 06 12345678"), "+390612345678");
        assert_eq!(normalize_phone_number("+39 (0)6 12345678"), "+390612345678");
        assert_eq!(normalize_phone_number("+81 3 1234 5678"), "+81312345678");
        assert_eq!(normalize_phone_number("(903) 123-4567"), "9031234567");
    }

    #[test]
    fn excludes_privacy_mailboxes_from_general_contact_collection() {
        assert!(
            extract_emails("dpo@morning.fr dataprotection@clinic.org contact@clinic.org")
                .iter()
                .all(|email| email == "contact@clinic.org")
        );
        assert!(!is_valid_email("dpo@morning.fr"));
    }

    #[test]
    fn rejects_international_example_numbers_even_in_tel_links() {
        let phones = extract_phones(
            r#"<a href="tel:+33123456789">Call</a><p>Example: +44 1234567890</p><a href="tel:+33173096000">Paris</a>"#,
        );
        assert_eq!(phones, vec!["+33173096000"]);
    }

    #[test]
    fn removes_the_australian_domestic_zero_from_international_numbers() {
        assert_eq!(normalize_phone_number("+61 0476 225 575"), "+61476225575");
        assert_eq!(
            normalize_phone_number("+61 (0) 476 225 575"),
            "+61476225575"
        );
        assert_eq!(normalize_phone_number("+61 2 1234 5678"), "+61212345678");
        assert_eq!(normalize_phone_number("0476 225 575"), "0476225575");
        assert_eq!(normalize_phone_number("+39 06 6982"), "+39066982");
    }

    #[test]
    fn ignores_form_placeholders_but_keeps_published_text_and_mailto() {
        let html = r#"<input placeholder="name@email.ru"><div data-example="sample@template.org">Contact: &lt;bookings@studio.org&gt;</div><a href="mailto:hello@clinic.org">Email us</a>"#;
        let emails = extract_emails(html);
        assert_eq!(
            emails.len(),
            2,
            "HTML attributes were treated as published contacts: {emails:?}"
        );
        assert!(emails.contains(&"bookings@studio.org".to_string()));
        assert!(emails.contains(&"hello@clinic.org".to_string()));
    }

    #[test]
    fn test_role_account_detection() {
        assert!(is_role_based_email("info@company.com"));
        assert!(is_role_based_email("support@tech.ru"));
        assert!(is_role_based_email("billing-dept@acme.org"));
        assert!(!is_role_based_email("pavel.durov@telegram.org"));
        assert!(!is_role_based_email("alexander@startup.io"));
    }

    #[test]
    fn test_email_permutator() {
        let perms = generate_email_permutations("Иван", "Петров", "yandex.ru");
        assert!(perms.contains(&"ivan.petrov@yandex.ru".to_string()));
        assert!(perms.contains(&"i.petrov@yandex.ru".to_string()));
        assert!(perms.contains(&"ivan@yandex.ru".to_string()));
    }
}
