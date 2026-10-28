use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimeAuditReport {
    pub html_char_count: usize,
    pub text_char_count: usize,
    pub html_to_text_ratio: f64,
    pub tag_count: usize,
    pub has_hidden_elements: bool,
    pub has_risky_tags: bool,
    pub has_base64_images: bool,
    pub deliverability_risk: String,
    pub advice: Vec<String>,
}

pub fn audit_mime_html_content(html: &str, plaintext: &str) -> MimeAuditReport {
    let html_lower = html.to_lowercase();
    let html_char_count = html.chars().count();
    let text_char_count = plaintext.chars().count();

    // 1. Tag count
    let re_tag = Regex::new(r"<[^>]+>").ok();
    let tag_count = re_tag
        .as_ref()
        .map(|r| r.find_iter(&html_lower).count())
        .unwrap_or(0);

    // 2. HTML to Text Ratio
    let ratio = if text_char_count > 0 {
        html_char_count as f64 / text_char_count as f64
    } else if html_char_count > 0 {
        10.0
    } else {
        1.0
    };

    // 3. Risky HTML Tags
    let risky_patterns = [
        "<script",
        "<iframe",
        "<form",
        "<object",
        "<embed",
        "<input",
        "<textarea",
    ];
    let mut has_risky_tags = false;
    let mut advice = Vec::new();

    for tag in &risky_patterns {
        if html_lower.contains(tag) {
            has_risky_tags = true;
            advice.push(format!("Risky HTML element detected: {}. Mail servers aggressively block interactive or embedded executable tags.", tag));
            break;
        }
    }

    // 4. Hidden text & Zero-font Spam Traps
    let hidden_patterns = [
        "display:none",
        "display: none",
        "visibility:hidden",
        "visibility: hidden",
        "font-size:0",
        "font-size: 0",
        "font-size:0px",
        "font-size: 0px",
        "font-size:0pt",
        "opacity:0",
        "opacity: 0",
        "color:transparent",
        "color: transparent",
    ];
    let mut has_hidden_elements = false;
    for pat in &hidden_patterns {
        if html_lower.contains(pat) {
            has_hidden_elements = true;
            advice.push(format!("Hidden text or zero-font pattern detected ('{}'). Spam filters (SpamBrain/Defender) treat hidden text as high-risk spam evasion.", pat));
            break;
        }
    }

    // 5. Heavy Base64 Images
    let has_base64_images = html_lower.contains("data:image/");
    if has_base64_images {
        advice.push("Inline base64 image detected. Embedded images significantly inflate MIME payload size and frequently trigger promotions tab or spam filters.".to_string());
    }

    // 6. Ratio Advice
    if ratio > 3.5 && html_char_count > 500 {
        advice.push(format!("High HTML overhead: HTML-to-text ratio is {:.1}x. B2B outreach delivers best with minimal styling and a ratio below 2.0x.", ratio));
    }

    // 7. Overall Risk Assessment
    let deliverability_risk = if has_risky_tags || has_hidden_elements {
        "High (Spam Filters Alert: Immediate Quarantine Risk)".to_string()
    } else if has_base64_images || ratio > 4.0 {
        "Moderate (Promotions Tab / HTML Overhead Risk)".to_string()
    } else {
        "Low (Clean & Balanced: Optimal Inbox Placement)".to_string()
    };

    if advice.is_empty() {
        advice.push("MIME structure is well-balanced: clean HTML markup without obfuscated styling or suspicious tags.".to_string());
    }

    MimeAuditReport {
        html_char_count,
        text_char_count,
        html_to_text_ratio: (ratio * 10.0).round() / 10.0,
        tag_count,
        has_hidden_elements,
        has_risky_tags,
        has_base64_images,
        deliverability_risk,
        advice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_balanced_email() {
        let html = "<p>Здравствуйте, Алексей!</p><p>Хотел обсудить проект безопасности.</p>";
        let text = "Здравствуйте, Алексей!\n\nХотел обсудить проект безопасности.";
        let rep = audit_mime_html_content(html, text);
        assert!(!rep.has_hidden_elements);
        assert!(!rep.has_risky_tags);
        assert!(!rep.has_base64_images);
        assert_eq!(
            rep.deliverability_risk,
            "Low (Clean & Balanced: Optimal Inbox Placement)"
        );
    }

    #[test]
    fn test_hidden_zero_font_spam_trap() {
        let html = r#"<div>Привет! <span style="font-size:0px; display:none;">spam keywords click here</span> Как дела?</div>"#;
        let text = "Привет! Как дела?";
        let rep = audit_mime_html_content(html, text);
        assert!(rep.has_hidden_elements);
        assert!(rep.deliverability_risk.contains("High"));
    }

    #[test]
    fn test_risky_script_and_iframe_tags() {
        let html = r#"<div>Тест<script>alert(1)</script></div>"#;
        let text = "Тест";
        let rep = audit_mime_html_content(html, text);
        assert!(rep.has_risky_tags);
        assert!(rep.deliverability_risk.contains("High"));
    }

    #[test]
    fn test_base64_heavy_image_alert() {
        let html = r#"<p>Привет</p><img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=" />"#;
        let text = "Привет";
        let rep = audit_mime_html_content(html, text);
        assert!(rep.has_base64_images);
        assert!(rep.deliverability_risk.contains("Moderate"));
    }
}
