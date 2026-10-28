use super::location::Location;
use crate::models::{ContactEvidence, ContactRecord, LeadScore};
use crate::search::extractor::{
    detect_industry_tags, extract_emails, extract_phones, extract_telegrams, extract_vk_links,
    extract_whatsapp_links,
};
use reqwest::Client;
use std::{sync::LazyLock, time::Duration};
use url::Url;

static HTML_TAG_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"<[^>]*>").unwrap());
static PHONE_PARAGRAPH_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?is)<p\b[^>]*>.*?</p\s*>").unwrap());
static SELECT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?is)<select\b[^>]*>.*?</select\s*>").unwrap());
static H1_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?is)<h1\b[^>]*>(.*?)</h1\s*>").unwrap());
static LOCATION_LINK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?is)<a\b[^>]*href\s*=\s*["']([^"']+)["'][^>]*>(.*?)</a\s*>"#).unwrap()
});

fn location_text(body: &str, domain: &str, location: &Location) -> String {
    let body = crate::search::extractor::strip_html_boilerplate(body);
    let body = SELECT_RE.replace_all(&body, " ");
    let body = LOCATION_LINK_RE.replace_all(&body, |cap: &regex::Captures<'_>| {
        let text = crate::search::extractor::clean_html_and_entities(
            &HTML_TAG_RE.replace_all(&cap[2], " "),
        );
        let other_site = Url::parse(&crate::search::extractor::unescape_html_entities(&cap[1]))
            .ok()
            .is_some_and(|url| url.host_str().is_some_and(|host| host != domain));
        if other_site && location.is_place_name(&text) {
            " ".to_string()
        } else {
            cap[0].to_string()
        }
    });
    crate::search::extractor::clean_html_and_entities(&HTML_TAG_RE.replace_all(&body, " "))
}

const CONTACT_PATHS: &[&str] = &[
    "/",
    "/contacts",
    "/contact",
    "/contact-us",
    "/kontakty",
    "/kontakt",
    "/about",
    "/team",
    "/careers",
];

pub(crate) fn is_company_website(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    matches!(parsed.scheme(), "http" | "https") && !excluded_listing(&parsed)
}

fn excluded_listing(parsed: &Url) -> bool {
    let domain = parsed.host_str().unwrap_or("");
    let parts: Vec<_> = parsed
        .path()
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.first() == Some(&"companies") && parts.len() >= 4 {
        return true;
    }
    parsed
        .path()
        .split('/')
        .any(|part| part == "b2b-database" || part == "find-a-vet" || part.starts_with("list-of-"))
        || domain.is_empty()
        || [
            "google.com",
            "yandex.ru",
            "yandex.com",
            "vk.com",
            "vk.ru",
            "t.me",
            "telegram.me",
            "facebook.com",
            "instagram.com",
            "linkedin.com",
            "2gis.ru",
            "zoon.ru",
            "jsprav.ru",
            "yp.ru",
            "2base.ru",
            "sprav.co",
            "krspages.ru",
            "spravker.ru",
            "doctu.ru",
            "baza-inform.ru",
            "spr.ru",
            "sibdom.ru",
            "veterinarnye-kliniki.com",
            "dverprof.com",
            "infrus.ru",
            "aion-portent.ru",
            "poisk-firm.ru",
            "ouuo.ru",
            "candy-factory.ru",
            "rhotel.site",
            "orgpage.ru",
            "flamp.ru",
            "yell.ru",
            "whatclinic.com",
            "studiogo.ru",
            "workin.space",
            "doctena.de",
            "allaboutberlin.com",
            "cybo.com",
            "poidata.io",
            "photo-firmika.ru",
            "on-mend.com",
            "rubrikator.org",
            "placidway.com",
            "thestorefront.com",
            "socialcompare.com",
            "wikipedia.org",
            "slideshare.net",
            "cataloxy.ru",
            "infobel.com",
            "asktel.ru",
            "nsregion.ru",
            "gorko.ru",
            "redmap.ru",
        ]
        .iter()
        .any(|host| domain == *host || domain.ends_with(&format!(".{host}")))
}

pub async fn crawl_website_for_contacts(
    client: &Client,
    start_url: &str,
    city: &str,
    stack: &str,
) -> Result<Vec<ContactRecord>, String> {
    if !is_company_website(start_url) {
        return Ok(Vec::new());
    }
    let location = Location::resolve(client, city)
        .await
        .unwrap_or_else(|_| Location::raw(city));
    crawl_with_location(client, start_url, city, stack, &location).await
}

pub(super) async fn crawl_with_location(
    client: &Client,
    start_url: &str,
    city: &str,
    stack: &str,
    location: &Location,
) -> Result<Vec<ContactRecord>, String> {
    let parsed = Url::parse(start_url).map_err(|e| format!("Invalid website URL: {}", e))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Website URL must use HTTP or HTTPS".into());
    }
    // Directory and vendor details do not belong to the businesses they list.
    if excluded_listing(&parsed) {
        return Ok(Vec::new());
    }
    let domain = parsed.host_str().unwrap_or("").to_string();

    let is_domain = matches!(parsed.host(), Some(url::Host::Domain(_)));
    let base_origin = parsed.origin().ascii_serialization();
    let mut discovered_records = Vec::new();
    let mut company_name = extract_company_name_from_domain(&domain);
    let mut checked_emails = Vec::new();
    let mut best_phone = String::new();
    let mut best_tg = String::new();
    let mut best_vk = String::new();
    let mut best_wa = String::new();
    let mut best_legal_name = String::new();
    let mut best_legal_explicit = false;
    let mut best_description = String::new();
    let mut best_address = String::new();
    let mut detected_tags = Vec::new();
    let mut detected_tech = Vec::new();
    let mut detected_analytics = Vec::new();
    let mut best_inn = String::new();
    let mut best_ogrn = String::new();
    let mut best_vat = String::new();
    let mut is_hiring_active = false;
    let mut hiring_roles = Vec::new();

    let mut pages = vec![parsed.to_string()];
    for path in CONTACT_PATHS {
        let url = format!("{base_origin}{path}");
        if !pages.contains(&url) {
            pages.push(url);
        }
    }
    let mut published_contact_url = String::new();
    let mut location_found = location.matches("", "");
    let mut location_evidence_url = String::new();
    let mut office_scoped = false;
    let mut readable_pages = 0;
    // The start page's error explains a failure; later paths mostly 404.
    let mut first_error = String::new();
    let mut evidence = Vec::new();
    let mut browser_attempted = false;
    // Search may already have found the contact page; keep its path and port.
    for page_url in pages {
        let resp = match client
            .get(&page_url)
            .timeout(Duration::from_secs(8))
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
            .send()
            .await
        {
            Ok(r) => r,
            Err(error) => { if first_error.is_empty() { first_error = error.to_string(); } continue; },
        };

        if !resp.status().is_success() {
            if first_error.is_empty() {
                first_error = format!("HTTP {} at {page_url}", resp.status().as_u16());
            }
            continue;
        }

        if excluded_listing(resp.url()) {
            return Ok(Vec::new());
        }

        let headers_str = format!("{:?}", resp.headers());
        let mut body = match resp.text().await {
            Ok(b) => b,
            Err(error) => {
                if first_error.is_empty() {
                    first_error = error.to_string();
                }
                continue;
            }
        };
        if !browser_attempted
            && super::scrapling::python_path().is_some()
            && super::scrapling::needs_browser(&body)
        {
            browser_attempted = true;
            match super::scrapling::render(&page_url).await {
                Ok(rendered) => {
                    if excluded_listing(&rendered.url)
                        || rendered.url.host_str() != parsed.host_str()
                    {
                        return Ok(Vec::new());
                    }
                    body = rendered.html;
                }
                Err(error) => {
                    if first_error.is_empty() {
                        first_error = format!("Scrapling: {error}");
                    }
                }
            }
        }
        if is_directory_page(&body) {
            return Ok(Vec::new());
        }
        if H1_RE.captures_iter(&body).any(|cap| {
            let text = crate::search::extractor::clean_html_and_entities(
                &HTML_TAG_RE.replace_all(&cap[1], " "),
            )
            .trim()
            .to_lowercase();
            matches!(
                text.as_str(),
                "404" | "page not found" | "404 page not found" | "not found"
            )
        }) {
            if first_error.is_empty() {
                first_error = format!("Page not found at {page_url} (HTTP 200)");
            }
            continue;
        }
        readable_pages += 1;

        // Firmographics extraction (JSON-LD Schema.org & OpenGraph)
        let mut firmo = crate::search::firmographics::extract_firmographics(&body);
        let (local_emails, local_phones) = super::contact_blocks::local_contacts(&body, location);
        let has_local = !local_emails.is_empty() || !local_phones.is_empty();
        if !location.country_matches(&firmo.country) || location.locality_conflicts(&firmo.locality)
        {
            let office_heading = regex::Regex::new(r"(?is)<h[1-6]\b[^>]*>(.*?)</h[1-6]\s*>")
                .unwrap()
                .captures_iter(&body)
                .any(|cap| {
                    location.is_place_name(&crate::search::extractor::clean_html_and_entities(
                        &HTML_TAG_RE.replace_all(&cap[1], " "),
                    ))
                });
            if !has_local || !office_heading {
                return Ok(Vec::new());
            }
            firmo.address.clear();
            firmo.country.clear();
            firmo.locality.clear();
            firmo.phone.clear();
            firmo.email.clear();
        }
        let found_here = has_local
            || location.matches(
                &format!(
                    "{} {}",
                    location_text(&body, &domain, location),
                    firmo.address
                ),
                &firmo.country,
            );
        if best_address.is_empty() && found_here && location.country_code == "RU" {
            best_address = russian_street_address(&location_text(&body, &domain, location))
                .unwrap_or_default();
        }
        location_found |= found_here;
        if found_here && location_evidence_url.is_empty() {
            location_evidence_url = page_url.clone();
        }
        if has_local {
            if !office_scoped {
                discovered_records.clear();
                checked_emails.clear();
                office_scoped = true;
            }
            best_phone = local_phones.first().cloned().unwrap_or_default();
        }
        if best_legal_name.is_empty() && !firmo.legal_name.is_empty() {
            best_legal_explicit = firmo.legal_name_explicit;
            best_legal_name = firmo.legal_name;
        }
        if best_description.is_empty() && !firmo.description.is_empty() {
            best_description = firmo.description;
        }
        if best_address.is_empty() && !firmo.address.is_empty() {
            best_address = firmo.address;
        }
        if best_phone.is_empty() && !has_local {
            // Prefer a phone beside the requested office over numbers from a global footer/schema.
            best_phone = PHONE_PARAGRAPH_RE
                .find_iter(&body)
                .filter(|paragraph| {
                    location.matches(
                        &crate::search::extractor::clean_html_and_entities(
                            &HTML_TAG_RE.replace_all(paragraph.as_str(), " "),
                        ),
                        &firmo.country,
                    )
                })
                .flat_map(|paragraph| {
                    super::extractor::phones_for_country(paragraph.as_str(), &location.country_code)
                })
                .next()
                .unwrap_or_default();
        }
        if best_phone.is_empty() && !has_local && !firmo.phone.is_empty() {
            let phone = crate::search::extractor::normalize_phone_number(&firmo.phone);
            if crate::search::extractor::is_plausible_phone(&phone) {
                best_phone = phone;
            }
        }
        for s in &firmo.socials {
            if best_vk.is_empty() && s.contains("vk.com") {
                best_vk = s.clone();
            } else if best_tg.is_empty() && (s.contains("t.me") || s.contains("telegram.me")) {
                best_tg = s.clone();
            } else if best_wa.is_empty() && (s.contains("wa.me") || s.contains("whatsapp.com")) {
                best_wa = s.clone();
            }
        }

        // Tax & Corporate ID extraction (Rusprofile / VIES style)
        let tax_prof = crate::search::tax_id::extract_tax_identifiers(&body);
        if best_inn.is_empty() && !tax_prof.inn.is_empty() {
            best_inn = tax_prof.inn;
        }
        if best_ogrn.is_empty() && !tax_prof.ogrn.is_empty() {
            best_ogrn = tax_prof.ogrn;
        }
        if best_vat.is_empty() && !tax_prof.vat_id.is_empty() {
            best_vat = tax_prof.vat_id;
        }

        // Hiring intent & Career signals (Apollo / Clay style)
        let career = crate::search::career_signals::detect_career_signals(&body, &page_url);
        if career.is_hiring {
            is_hiring_active = true;
            for r in career.detected_roles {
                if !hiring_roles.contains(&r) {
                    hiring_roles.push(r);
                }
            }
        }

        // Tech stack fingerprinting (Wappalyzer / Clearbit style)
        let tech_res = crate::search::tech_detect::detect_tech_stack(&body, &headers_str);
        for tag in tech_res.all_tags {
            if !detected_tech.contains(&tag) {
                detected_tech.push(tag);
            }
        }
        for an in tech_res.analytics {
            if !detected_analytics.contains(&an) {
                detected_analytics.push(an);
            }
        }

        // Contact pages can also carry stale template names in structured metadata.
        let domain_brand = extract_company_name_from_domain(&domain);
        let page_brand = extract_clean_company_name(&body, &domain_brand, &best_legal_name);
        let page_brand = if location.is_place_name(&page_brand) {
            domain_brand.clone()
        } else {
            page_brand
        };
        if page_url == format!("{base_origin}/") || page_brand.eq_ignore_ascii_case(&domain_brand) {
            company_name = page_brand;
            if !best_legal_explicit
                && company_name.eq_ignore_ascii_case(&extract_company_name_from_domain(&domain))
                && !best_legal_name
                    .to_lowercase()
                    .contains(&company_name.to_lowercase())
            {
                best_legal_name.clear();
            }
        }

        let mut emails = if has_local {
            local_emails
        } else {
            extract_emails(&body)
        };
        for email in if has_local {
            Vec::new()
        } else {
            extract_emails(&firmo.email)
        } {
            if !emails.contains(&email) {
                emails.push(email);
            }
        }
        let phones = if has_local {
            local_phones
        } else {
            super::extractor::phones_for_country(&body, &location.country_code)
        };
        let telegrams = extract_telegrams(&body);
        let vks = extract_vk_links(&body);
        let was = extract_whatsapp_links(&body);
        let tags = detect_industry_tags(&body);
        let mut published_phones = phones.clone();
        if !firmo.phone.is_empty() {
            published_phones.extend(super::extractor::phones_for_country(
                &format!("<a href=\"tel:{}\">Call</a>", firmo.phone),
                &location.country_code,
            ));
        }
        evidence.push(ContactEvidence {
            url: page_url.clone(),
            checked_at: chrono::Utc::now().to_rfc3339(),
            emails: emails.clone(),
            phones: published_phones,
            telegrams: telegrams.clone(),
            vk_urls: vks.clone(),
            location_matches: found_here,
        });

        for t in tags {
            if !detected_tags.contains(&t) {
                detected_tags.push(t);
            }
        }

        if best_phone.is_empty() && !phones.is_empty() {
            best_phone = phones[0].clone();
        }
        if best_tg.is_empty() && !telegrams.is_empty() {
            best_tg = telegrams[0].clone();
        }
        if best_vk.is_empty() && !vks.is_empty() {
            best_vk = vks[0].clone();
        }
        if best_wa.is_empty() && !was.is_empty() {
            best_wa = was[0].clone();
        }

        if published_contact_url.is_empty()
            && (!best_phone.is_empty()
                || !best_tg.is_empty()
                || !best_vk.is_empty()
                || !best_wa.is_empty())
        {
            published_contact_url = page_url.clone();
        }
        for email in emails {
            if !checked_emails.contains(&email) {
                checked_emails.push(email.clone());

                let role = if email.starts_with("hr")
                    || email.starts_with("career")
                    || email.starts_with("job")
                    || email.starts_with("hiring")
                {
                    "HR / Recruiter".to_string()
                } else if email.starts_with("ceo")
                    || email.starts_with("founder")
                    || email.starts_with("director")
                {
                    "Founder / Director".to_string()
                } else {
                    "General contact".to_string()
                };

                let score = if email.starts_with("hr")
                    || email.starts_with("career")
                    || email.starts_with("ceo")
                    || tech_res.ats.is_some()
                {
                    LeadScore::Hot
                } else {
                    LeadScore::Warm
                };

                let contact_name = format!("{} team", company_name);

                let mut record = ContactRecord::new(
                    &contact_name,
                    &email,
                    &company_name,
                    &role,
                    city,
                    stack,
                    &best_phone,
                    &best_tg,
                    "Company website",
                    &page_url,
                    score,
                    false, // will be verified by DoH in coordinator
                    "",
                );
                record.vk_url = best_vk.clone();
                record.whatsapp = best_wa.clone();
                record.tags = detected_tags.clone();
                record.tech_stack = detected_tech.clone();
                record.analytics_ids = detected_analytics.clone();
                record.legal_name = best_legal_name.clone();
                record.company_description = best_description.clone();
                record.address = best_address.clone();
                record.suggested_email_fix =
                    crate::search::domain_pattern::detect_domain_typo(&email);
                record.is_disposable = crate::search::extractor::is_disposable_email(&email);
                record.is_role_account = crate::search::extractor::is_role_based_email(&email);
                record.mail_provider = crate::search::extractor::detect_mail_provider(&email);
                record.lead_score = crate::search::extractor::calculate_lead_score(&record);
                record.b2b_tier = crate::search::extractor::calculate_b2b_tier(&record);
                if let Some((ref v, ref s)) = tech_res.ats {
                    record.notes = format!("Detected ATS: {} ({})", v, s);
                    record.score = LeadScore::Hot;
                    record.lead_score = 90;
                }
                discovered_records.push(record);
            }
        }

        // If we found emails on this domain, stop probing remaining paths to save time
        if !discovered_records.is_empty() && location_found {
            break;
        }
    }

    // A search ranking is not evidence of the company's location.
    if readable_pages == 0 {
        return Err(format!(
            "No readable company pages at {start_url}: {first_error}"
        ));
    }
    if !location_found {
        return Ok(Vec::new());
    }

    // Deep discovery via sitemap.xml & robots.txt (Screaming Frog / Katana style)
    if discovered_records.is_empty() && published_contact_url.is_empty() && is_domain {
        let sitemap_urls =
            crate::search::sitemap_crawler::discover_contact_pages_via_sitemap(client, &domain)
                .await;
        for sm_page in sitemap_urls {
            if let Ok(resp) = client
                .get(&sm_page)
                .timeout(Duration::from_secs(5))
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                )
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(body) = resp.text().await {
                        let emails = extract_emails(&body);
                        let phones = extract_phones(&body);
                        if best_phone.is_empty() && !phones.is_empty() {
                            best_phone = phones[0].clone();
                        }
                        for email in emails {
                            if !checked_emails.contains(&email) {
                                checked_emails.push(email.clone());
                                let contact_name = format!("{} team", company_name);
                                let mut record = ContactRecord::new(
                                    &contact_name,
                                    &email,
                                    &company_name,
                                    "Sitemap contact",
                                    city,
                                    stack,
                                    &best_phone,
                                    &best_tg,
                                    "sitemap.xml",
                                    &sm_page,
                                    LeadScore::Warm,
                                    false,
                                    "",
                                );
                                record.vk_url = best_vk.clone();
                                record.whatsapp = best_wa.clone();
                                record.tags = detected_tags.clone();
                                record.tech_stack = detected_tech.clone();
                                record.analytics_ids = detected_analytics.clone();
                                record.legal_name = best_legal_name.clone();
                                record.company_description = best_description.clone();
                                record.address = best_address.clone();
                                record.suggested_email_fix =
                                    crate::search::domain_pattern::detect_domain_typo(&email);
                                record.is_disposable =
                                    crate::search::extractor::is_disposable_email(&email);
                                record.is_role_account =
                                    crate::search::extractor::is_role_based_email(&email);
                                record.mail_provider =
                                    crate::search::extractor::detect_mail_provider(&email);
                                record.lead_score =
                                    crate::search::extractor::calculate_lead_score(&record);
                                record.b2b_tier =
                                    crate::search::extractor::calculate_b2b_tier(&record);
                                discovered_records.push(record);
                            }
                        }
                    }
                }
            }
            if !discovered_records.is_empty() {
                break;
            }
        }
    }

    // Try RFC 9116 security.txt & humans.txt OSINT discovery if standard paths yielded nothing
    if discovered_records.is_empty() && published_contact_url.is_empty() && is_domain {
        let recon = crate::search::recon::run_domain_recon(client, &domain).await;
        for sec_email in recon.security_contacts {
            if !checked_emails.contains(&sec_email) {
                checked_emails.push(sec_email.clone());
                let mut record = ContactRecord::new(
                    &format!("{} team", company_name),
                    &sec_email,
                    &company_name,
                    "Security contact (security.txt)",
                    city,
                    stack,
                    &best_phone,
                    &best_tg,
                    "security.txt (RFC 9116)",
                    &format!("https://{}/.well-known/security.txt", domain),
                    LeadScore::Hot,
                    false,
                    "",
                );
                record.vk_url = best_vk.clone();
                record.whatsapp = best_wa.clone();
                record.tags = detected_tags.clone();
                record.tech_stack = detected_tech.clone();
                record.analytics_ids = detected_analytics.clone();
                record.legal_name = best_legal_name.clone();
                record.company_description = best_description.clone();
                record.address = best_address.clone();
                record.b2b_tier = crate::search::extractor::calculate_b2b_tier(&record);
                record.lead_score = crate::search::extractor::calculate_lead_score(&record);
                discovered_records.push(record);
            }
        }
        for hum_email in recon.human_contacts {
            if !checked_emails.contains(&hum_email) {
                checked_emails.push(hum_email.clone());
                let mut record = ContactRecord::new(
                    &format!("{} developer", company_name),
                    &hum_email,
                    &company_name,
                    "Developer / Author (humans.txt)",
                    city,
                    stack,
                    &best_phone,
                    &best_tg,
                    "humans.txt",
                    &format!("https://{}/humans.txt", domain),
                    LeadScore::Hot,
                    false,
                    "",
                );
                record.vk_url = best_vk.clone();
                record.whatsapp = best_wa.clone();
                record.tags = detected_tags.clone();
                record.tech_stack = detected_tech.clone();
                record.analytics_ids = detected_analytics.clone();
                record.legal_name = best_legal_name.clone();
                record.company_description = best_description.clone();
                record.address = best_address.clone();
                record.b2b_tier = crate::search::extractor::calculate_b2b_tier(&record);
                record.lead_score = crate::search::extractor::calculate_lead_score(&record);
                discovered_records.push(record);
            }
        }
    }

    // Keep published phone/social contacts without inventing an email address.
    if discovered_records.is_empty() && !published_contact_url.is_empty() {
        let mut record = ContactRecord::new(
            &format!("{} team", company_name),
            "",
            &company_name,
            "General contact",
            city,
            stack,
            &best_phone,
            &best_tg,
            "Company website",
            &published_contact_url,
            LeadScore::Cold,
            false,
            "",
        );
        record.vk_url = best_vk;
        record.whatsapp = best_wa;
        record.tags = detected_tags;
        record.tech_stack = detected_tech;
        record.analytics_ids = detected_analytics;
        record.legal_name = best_legal_name;
        record.company_description = best_description;
        record.address = best_address;
        record.b2b_tier = crate::search::extractor::calculate_b2b_tier(&record);
        record.lead_score = crate::search::extractor::calculate_lead_score(&record);
        discovered_records.push(record);
    }

    // Post-enrich all discovered records with Phone Intel, Tax IDs, Hiring Signals, and Icebreaker
    for rec in &mut discovered_records {
        if !location_evidence_url.is_empty() && rec.url != location_evidence_url {
            if !rec.notes.is_empty() {
                rec.notes.push('\n');
            }
            rec.notes
                .push_str(&format!("Location evidence: {location_evidence_url}"));
        }
        if !best_inn.is_empty() && rec.inn.is_empty() {
            rec.inn = best_inn.clone();
            if !rec.tags.iter().any(|t| t.starts_with("ИНН:")) {
                rec.tags.push(format!("ИНН: {}", best_inn));
            }
        }
        if !best_ogrn.is_empty() && rec.ogrn.is_empty() {
            rec.ogrn = best_ogrn.clone();
        }
        if !best_vat.is_empty() && rec.vat_id.is_empty() {
            rec.vat_id = best_vat.clone();
        }
        if is_hiring_active {
            rec.hiring_active = true;
            for r in &hiring_roles {
                if !rec.hiring_roles.contains(r) {
                    rec.hiring_roles.push(r.clone());
                }
            }
            if !rec.tags.iter().any(|t| t == "Активно нанимает") {
                rec.tags.push("Активно нанимает".to_string());
            }
            rec.lead_score = (rec.lead_score + 15).min(100);
        }
        if !rec.phone.is_empty() {
            if matches!(location.country_code.as_str(), "CA" | "US" | "RU")
                && !rec.phone.starts_with('+')
                && rec.phone.len() == 10
                && rec.phone.chars().all(|c| c.is_ascii_digit())
            {
                rec.phone = format!(
                    "+{}{}",
                    if location.country_code == "RU" {
                        "7"
                    } else {
                        "1"
                    },
                    rec.phone
                );
            }
            let p_info = crate::search::phone_intel::parse_and_enrich_phone(&rec.phone);
            if p_info.is_valid {
                rec.phone = p_info.formatted;
                rec.phone_carrier = p_info.carrier;
                rec.phone_region = p_info.region;
                rec.phone_type = p_info.line_type;
                rec.whatsapp_url = p_info.whatsapp_url.unwrap_or_default();
                rec.telegram_url = p_info.telegram_url.unwrap_or_default();
            }
        }
        rec.icebreaker = crate::search::icebreaker::generate_dynamic_icebreaker(rec);
    }

    for record in &mut discovered_records {
        record.evidence = evidence.clone();
    }
    Ok(discovered_records)
}

fn is_directory_page(body: &str) -> bool {
    static TITLE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?is)<title\b[^>]*>(.*?)</title>").unwrap());
    let title = TITLE
        .captures(body)
        .map(|cap| crate::search::extractor::clean_html_and_entities(&cap[1]).to_lowercase())
        .unwrap_or_default();
    [
        "справочник",
        "каталог организаций",
        "каталог компаний",
        "рейтинг компаний",
        "список компаний",
        "business directory",
        "company directory",
        "yellow pages",
    ]
    .iter()
    .any(|term| title.contains(term))
}

fn russian_street_address(text: &str) -> Option<String> {
    static ADDRESS: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?iu)(?:ул(?:ица|\.)?|пр(?:оспект|\.)?|пер(?:еулок|\.)?)\s+([\p{L}\s-]{2,50}),?\s*(?:д(?:ом|\.)?\s*)?(\d+[а-яa-z]?)(?:\s*,?\s*(?:стр(?:оение|\.)?|ст\.?)\s*(\d+))?").unwrap()
    });
    let cap = ADDRESS.captures(text)?;
    let mut address = format!("ул. {}, д. {}", cap[1].trim(), &cap[2]);
    if let Some(building) = cap.get(3) {
        address.push_str(&format!(", стр. {}", building.as_str()));
    }
    Some(address)
}

#[cfg(test)]
fn location_matches(text: &str, location: &str) -> bool {
    Location::raw(location).matches(text, "")
}

fn extract_clean_company_name(html: &str, domain_brand: &str, best_legal: &str) -> String {
    let lower = html.to_lowercase();
    let start_tag = "<title>";
    let end_tag = "</title>";
    let mut title_tokens = Vec::new();

    if let Some(start_idx) = lower.find(start_tag) {
        let start = start_idx + start_tag.len();
        if let Some(end_rel) = lower[start..].find(end_tag) {
            let end = start + end_rel;
            let raw_title = &html[start..end];
            let unescaped = crate::search::extractor::unescape_html_entities(raw_title);

            for tok in unescaped.split(|c| {
                c == '|' || c == '—' || c == '–' || c == '-' || c == '•' || c == ':' || c == '»'
            }) {
                let trimmed = tok.trim();
                if !trimmed.is_empty() {
                    title_tokens.push(trimmed.to_string());
                }
            }
        }
    }

    let brand_lower = domain_brand.to_lowercase();

    // 1. Check if any title token matches or contains the domain brand name (e.g. "Kraken", "OKX", "Cloudera")
    for tok in &title_tokens {
        let tok_lower = tok.to_lowercase();
        if tok_lower == brand_lower {
            return tok.clone();
        }
        if tok_lower.split_whitespace().any(|word| word == brand_lower) && tok.chars().count() > 30
        {
            return domain_brand.to_string();
        }
        if tok_lower.starts_with(&brand_lower) || tok_lower.ends_with(&brand_lower) {
            if tok.len() <= 30 {
                return tok.clone();
            }
            if tok_lower.starts_with(&format!("{brand_lower} ")) {
                return domain_brand.to_string();
            }
        }
    }

    // 2. If legal name is present and clean, prefer it
    if !best_legal.is_empty()
        && best_legal.len() <= 40
        && !best_legal.contains("http")
        && !matches!(
            best_legal.trim().to_lowercase().as_str(),
            "wordpress" | "joomla" | "tilda" | "wix"
        )
    {
        return best_legal.to_string();
    }

    // 3. Check for a concise, clean brand token in title that isn't a long marketing pitch
    for tok in &title_tokens {
        let tok_lower = tok.to_lowercase();
        if matches!(
            tok_lower.as_str(),
            "главная"
                | "контакты"
                | "о компании"
                | "наши контакты"
                | "contact"
                | "contacts"
                | "about us"
                | "home"
        ) {
            continue;
        }
        let is_slogan = tok_lower.contains("buy ")
            || tok_lower.contains("best ")
            || tok_lower.contains("leading ")
            || tok_lower.contains("home")
            || tok_lower.contains("official")
            || tok_lower.contains("platform")
            || tok_lower.contains("institutional")
            || tok_lower.contains("welcome")
            || tok_lower.contains("service")
            || tok_lower.contains("solutions")
            || tok_lower.contains("crypto")
            || tok_lower.contains("bitcoin");
        if !is_slogan && tok.len() <= 30 && !tok.contains("...") {
            return tok.clone();
        }
    }

    // 4. Default to solid domain brand
    domain_brand.to_string()
}

fn extract_company_name_from_domain(domain: &str) -> String {
    let clean = domain.trim_start_matches("www.");
    let parts: Vec<&str> = clean.split('.').collect();
    if let Some(first) = parts.first() {
        let mut chars = first.chars();
        match chars.next() {
            None => clean.to_string(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    } else {
        clean.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn detects_unknown_directories_and_normalizes_actual_street_addresses() {
        assert!(is_directory_page("<title>Справочник Красноярска</title>"));
        assert!(is_directory_page("<title>Local Business Directory</title>"));
        assert!(!is_directory_page(
            "<title>Типография — каталог продукции</title>"
        ));
        assert_eq!(
            russian_street_address("Красноярск, ул. Маерчака, д.50, стр.6"),
            russian_street_address("г.Красноярск ул. Маерчака, д. 50, ст.6")
        );
        assert!(
            russian_street_address("Красноярск, ул. Маерчака, д.50, стр.6")
                .unwrap()
                .contains("50")
        );
    }

    #[tokio::test]
    async fn rejects_inferred_template_metadata_on_a_discovered_contact_page() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            socket.read(&mut request).await.unwrap();
            let body = r#"<title>Nakatomi LLC offices in Poland and around the World</title><script type="application/ld+json">{"@type":"Organization","name":"Codiqa"}</script><section><h2>Warsaw</h2><a href="mailto:biuro@agency.org">Email</a></section>"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let client = Client::builder()
            .no_proxy()
            .resolve("nakatomi.local", addr)
            .build()
            .unwrap();
        let records = crawl_fixture(
            &client,
            &format!("http://nakatomi.local:{}/contact", addr.port()),
            "Warsaw",
            "marketing agency",
        )
        .await
        .unwrap();
        server.abort();
        assert_eq!(records[0].company, "Nakatomi");
        assert!(
            records[0].legal_name.is_empty(),
            "Template metadata was saved as a legal entity"
        );
    }

    #[test]
    fn excludes_krasnoyarsk_business_directories() {
        for site in [
            "https://krasnoyarsk.yp.ru/",
            "https://2base.ru/catalog/krasnoiarskii-krai/krasnoiarsk",
            "https://krasnoyarsk.sprav.co/",
            "https://krspages.ru/",
            "https://krasnoyarsk.spravker.ru/veterinarnye-kliniki/",
            "https://doctu.ru/krasnoyarsk/clinic/stomatologija-akula",
            "https://baza-inform.ru/contacts",
            "https://sfo.spr.ru/krasnoyarsk/krasnoyarsk.html",
            "https://www.sibdom.ru/organization/view/30925/",
            "https://veterinarnye-kliniki.com/krasnoyarsk",
        ] {
            assert!(!is_company_website(site), "Directory accepted: {site}");
        }
        assert!(is_company_website("https://business-krsk.ru/kontakty"));
    }

    #[test]
    fn does_not_name_a_company_after_homepage_navigation() {
        assert_eq!(
            extract_clean_company_name("<title>Главная</title>", "Akkord24", ""),
            "Akkord24"
        );
        assert_eq!(
            extract_clean_company_name("<title>Контакты</title>", "Tkkts", ""),
            "Tkkts"
        );
        assert_eq!(
            extract_clean_company_name(
                "<title>Клининг Красноярск</title>",
                "Kroshkichistoty",
                "WordPress"
            ),
            "Kroshkichistoty"
        );
    }

    #[test]
    fn excludes_a_company_database_category_page_from_website_crawling() {
        assert!(!is_company_website(
            "https://revenuebase.ai/companies/architecture-firms/united-kingdom/london"
        ));
        assert!(is_company_website(
            "https://architect.org/our-offices/london/"
        ));
    }

    #[tokio::test]
    async fn rejects_a_neighboring_city_postal_address_despite_a_target_city_in_the_title() {
        let html = "<title>Accounting Toronto</title><p>625 Cochrane Drive, Suite 900<br/>Markham, Ontario<br/>L3R9R9, Canada</p>";
        let profile = super::super::firmographics::extract_firmographics(html);
        let photon = serde_json::json!({"features":[{"properties":{"osm_type":"R","osm_id":2,"type":"city","name":"Toronto","country":"Canada","countrycode":"CA"}}]});
        let location = Location::from_photon("Toronto, Canada", &photon, &photon);
        assert_eq!(profile.locality, "Markham");
        assert!(location.locality_conflicts(&profile.locality));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/contact", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            socket.read(&mut request).await.unwrap();
            let body = format!(
                "{html}<div>Toronto accounting <a href='mailto:enquiries@firm.org'>Email</a></div>"
            );
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        assert!(
            crawl_with_location(
                &Client::builder().no_proxy().build().unwrap(),
                &url,
                "Toronto, Canada",
                "accounting",
                &location
            )
            .await
            .unwrap()
            .is_empty()
        );
        server.abort();
    }

    #[test]
    fn uses_the_printed_national_number_when_a_tel_link_has_a_spurious_plus() {
        assert_eq!(
            super::super::extractor::phones_for_country(
                "<a href='tel:+9059461300'>905-946-1300</a>",
                "CA"
            )[0],
            "9059461300"
        );
        assert_eq!(
            extract_phones("<a href='tel:+9059461300'>+90 594 613 00</a>")[0],
            "+9059461300"
        );
        assert_eq!(
            super::super::extractor::phones_for_country(
                "<a href='tel:+9059461300'>905-946-1300</a>",
                "TR"
            )[0],
            "+9059461300"
        );
    }

    #[test]
    fn prefers_the_actual_brand_in_a_long_title_over_template_metadata() {
        assert_eq!(
            extract_clean_company_name(
                "<title>Nakatomi LLC offices in Poland and around the World</title>",
                "Nakatomi",
                "Codiqa"
            ),
            "Nakatomi"
        );
        assert_eq!(
            extract_clean_company_name(
                "<title>Agencja Marketingowa Warszawa - Najlepsza Agencja Marketingowa w Polsce Nakatomi LLC</title>",
                "Nakatomi",
                "Codiqa"
            ),
            "Nakatomi"
        );
    }

    #[tokio::test]
    async fn keeps_email_and_phone_from_the_requested_office_on_a_multi_office_page() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/contact", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            socket.read(&mut request).await.unwrap();
            let body = r#"<main><section><div><h2>Los Angeles</h2><div><a href="mailto:losangeles@agency.org">Email</a></div><div><a href="tel:+12132463806">Telephone</a></div></div></section>
            <section><div><h2>Warsaw</h2><div><a href="mailto:biuro@agency.org">Email</a></div><div><a href="tel:+48223071131">Telephone</a></div></div></section></main>"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let records = crawl_fixture(
            &Client::builder().no_proxy().build().unwrap(),
            &url,
            "Warsaw",
            "marketing agency",
        )
        .await
        .unwrap();
        server.abort();
        assert_eq!(
            records.len(),
            1,
            "Foreign-office contacts were included: {records:?}"
        );
        assert_eq!(records[0].email, "biuro@agency.org");
        assert_eq!(records[0].phone, "+48223071131");
        assert!(
            records[0]
                .evidence
                .iter()
                .any(|proof| proof.location_matches
                    && proof.emails.contains(&"biuro@agency.org".to_string())
                    && !proof.emails.contains(&"losangeles@agency.org".to_string()))
        );
    }

    #[tokio::test]
    async fn skips_soft_404_pages_before_collecting_a_real_contact_page() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                let n = socket.read(&mut request).await.unwrap();
                let body = if String::from_utf8_lossy(&request[..n]).starts_with("GET /contact ") {
                    r#"<h1>Contact</h1><p>Toronto</p><a href="mailto:office@accountant.org">Email</a>"#
                } else {
                    r#"<h1>Page not found</h1><footer>Toronto <a href="mailto:footer@errorpage.org">Footer</a></footer>"#
                };
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
        });
        let records = crawl_fixture(
            &Client::builder().no_proxy().build().unwrap(),
            &url,
            "Toronto",
            "accounting firm",
        )
        .await
        .unwrap();
        server.abort();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].email, "office@accountant.org");
        assert!(records[0].url.ends_with("/contact"));
    }

    async fn crawl_fixture(
        client: &Client,
        url: &str,
        city: &str,
        stack: &str,
    ) -> Result<Vec<ContactRecord>, String> {
        crawl_with_location(client, url, city, stack, &Location::raw(city)).await
    }

    #[tokio::test]
    #[ignore = "Requires installed Scrapling and Chromium; accesses only a local synthetic page"]
    async fn scrapling_collects_contact_from_javascript_page() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut request = [0; 2048];
                    let _ = socket.read(&mut request).await;
                    let body = r#"<html><body><div id="root"></div><script>document.getElementById('root').innerHTML='<h1>Studio</h1><p>Toronto</p><a href="mailto:hello@studio.org">hello@studio.org</a>';</script></body></html>"#;
                    let _ = socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
                });
            }
        });
        let rendered = super::super::scrapling::render(&url)
            .await
            .expect("browser rendering");
        assert!(rendered.html.contains("Toronto"), "{}", rendered.html);
        let result = crawl_fixture(
            &Client::builder().no_proxy().build().unwrap(),
            &url,
            "Toronto",
            "",
        )
        .await;
        server.abort();
        let records = result.unwrap();
        assert!(
            records
                .iter()
                .any(|r| r.email == "hello@studio.org" && r.evidence.iter().any(|e| e.url == url)),
            "{records:?}"
        );
    }

    #[test]
    fn matches_visible_place_names_without_substrings_or_losing_unicode() {
        assert!(location_matches(
            "Address: Красноярск, ул. Ленина",
            "Krasnoyarsk"
        ));
        assert!(location_matches("Clinic in São Paulo", "São Paulo"));
        assert!(location_matches(
            "Office in Berlin, Germany",
            "Berlin, Germany"
        ));
        assert!(!location_matches("Berlin clinic", "Berlin, Canada"));
        assert!(!location_matches("Burlington clinic", "London"));
        assert!(!location_matches("Londonville clinic", "London"));
        assert!(!location_matches("Unrelated studio", ", ,"));
        assert!(location_matches("Unrelated studio", "Remote Worldwide"));
    }

    #[tokio::test]
    async fn does_not_collect_platform_support_as_local_business_contacts() {
        let client = Client::builder().no_proxy().build().unwrap();
        for url in [
            "https://vk.ru/studio",
            "https://krasnoyarsk.jsprav.ru/fotostudii/",
            "https://zoon.ru/krasnoyarsk/",
            "https://krs.gorko.ru/%D1%84%D0%BE%D1%82%D0%BE%D1%81%D1%82%D1%83%D0%B4%D0%B8%D0%B8/",
            "https://2gis.ru/krasnoyarsk/",
            "https://www.instagram.com/studio/",
            "https://www.whatclinic.com/dentists/uk/london",
            "https://studiogo.ru/krasnoyarsk/studios",
            "https://workin.space/en/france",
            "https://es.cybo.com/RU-biz/photo-studio-plasticine",
            "https://www.doctena.de/en/dentist/berlin",
            "https://www.poidata.io/report/photography-studio/russia/krasnoyarsk-krai",
            "https://krasnoyarsk.photo-firmika.ru/",
            "https://allaboutberlin.com/guides/dentists",
            "https://on-mend.com/providers/germany/berlin/berlin/dentist",
            "https://rubrikator.org/russia/krasnoyarsk/mads-photo-studio",
            "https://www.placidway.com/view-review/323360/Clinic-Berlin",
            "https://www.thestorefront.com/spaces/france/paris/60823",
            "https://socialcompare.com/fr/comparison/coworking-paris",
            "https://en.wikipedia.org/wiki/Tour_First",
            "https://www.slideshare.net/StudiosSinguliers",
        ] {
            assert!(
                crawl_fixture(&client, url, "", "")
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[tokio::test]
    async fn keeps_the_discovered_contact_path_and_port() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}/en/contact-us?office=Berlin",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            let n = socket.read(&mut request).await.unwrap();
            assert!(
                String::from_utf8_lossy(&request[..n])
                    .starts_with("GET /en/contact-us?office=Berlin ")
            );
            let body = "<p>Berlin</p><a href='mailto:bookings@fixture-clinic.org'>Email</a>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let client = Client::builder().no_proxy().build().unwrap();
        let records = tokio::time::timeout(
            Duration::from_secs(5),
            crawl_fixture(&client, &url, "Berlin", "Dentist"),
        )
        .await
        .unwrap()
        .unwrap();
        server.await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].email, "bookings@fixture-clinic.org");
        assert_eq!(records[0].url, url);
        assert_eq!(records[0].source, "Company website");
    }

    #[tokio::test]
    async fn does_not_invent_an_email_for_a_site_without_contacts() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 17\r\nConnection: close\r\n\r\nNo public contact").await.unwrap();
            }
        });
        let client = Client::builder().no_proxy().build().unwrap();
        let records =
            tokio::time::timeout(Duration::from_secs(5), crawl_fixture(&client, &url, "", ""))
                .await;
        server.abort();
        assert!(records.unwrap().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_contact_from_a_different_city() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/contact", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                let body = "<script>var searchCity = 'Krasnoyarsk';</script><p>Photo studio in Moscow</p><a href='/Krasnoyarsk'>Book</a><a href='mailto:studio@fixture.org'>Email</a>";
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let result = crawl_fixture(
            &Client::builder().no_proxy().build().unwrap(),
            &url,
            "Krasnoyarsk",
            "Photo studio",
        )
        .await;
        server.abort();
        assert!(
            result.unwrap().is_empty(),
            "An unrelated city must not be assigned to a contact"
        );
    }

    #[tokio::test]
    async fn reports_unreadable_websites_as_errors_instead_of_empty_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/contact", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                socket
                    .write_all(
                        b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await
                    .unwrap();
            }
        });
        let result =
            crawl_fixture(&Client::builder().no_proxy().build().unwrap(), &url, "", "").await;
        server.abort();
        assert!(
            result.is_err(),
            "All pages were blocked, but the crawler returned {result:?}"
        );
    }

    #[tokio::test]
    async fn keeps_published_structured_contact_email_after_attribute_filtering() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/contact", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            socket.read(&mut request).await.unwrap();
            let body = r#"<script type="application/ld+json">{"@type":"Dentist","name":"Berlin Clinic","email":"bookings@fixture-clinic.org","address":{"addressLocality":"Berlin","addressCountry":"DE"}}</script><input placeholder="name@email.ru">"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let records = crawl_fixture(
            &Client::builder().no_proxy().build().unwrap(),
            &url,
            "Berlin",
            "dentist",
        )
        .await
        .unwrap();
        server.await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].email, "bookings@fixture-clinic.org");
    }

    #[tokio::test]
    async fn prefers_the_requested_office_phone_over_a_schema_example_or_other_offices() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            socket.read(&mut request).await.unwrap();
            let body = r#"<script type="application/ld+json">{"@type":"LocalBusiness","telephone":"+33123456789","email":"info@workways.org","address":{"addressLocality":"Paris","addressCountry":"FR"}}</script>
            <p>Brussels office <a href="tel:+3228913900">+32 2 891 39 00</a></p>
            <p><strong>Paris office</strong><br><a href="tel:+33173096000">+33 1 73 09 60 00</a></p>
            <p>Dublin office <a href="tel:+35319630600">+353 1 963 0600</a></p>"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let records = crawl_fixture(
            &Client::builder().no_proxy().build().unwrap(),
            &url,
            "Paris",
            "coworking",
        )
        .await
        .unwrap();
        server.await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].phone, "+33173096000");
    }

    #[tokio::test]
    async fn ignores_city_switchers_when_checking_a_company_branch_location() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                let body = r#"<title>Studio Kazan</title><a href="https://berlin.studio.org/"><span>Berlin</span></a><select><option>Berlin</option></select><p>Kazan office</p><a href="mailto:bookings@studio.org">Email</a>"#;
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
        });
        let records = crawl_fixture(
            &Client::builder().no_proxy().build().unwrap(),
            &url,
            "Berlin",
            "photo studio",
        )
        .await
        .unwrap();
        server.abort();
        assert!(
            records.is_empty(),
            "A city selector is not a company address: {records:?}"
        );
    }

    #[tokio::test]
    async fn does_not_attribute_a_database_vendors_contact_to_a_listed_business() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}/b2b-database/state/gb/list-of-architecture-firms-in-london",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                let body = r#"<title>List of architecture firms in London</title><h1>Buy a verified London business database</h1><a href="mailto:sales@data-vendor.org">Contact the data vendor</a><a href="tel:+919725145188">Sales</a>"#;
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
        });
        let records = crawl_fixture(
            &Client::builder().no_proxy().build().unwrap(),
            &url,
            "London",
            "architecture firm",
        )
        .await
        .unwrap();
        server.abort();
        assert!(
            records.is_empty(),
            "Contacts belong to the list vendor: {records:?}"
        );
    }
}
