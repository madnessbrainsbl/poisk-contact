use crate::models::{ContactRecord, EmailTemplate, SmtpConfig};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct AppConfigState {
    pub smtp: SmtpConfig,
    pub template: EmailTemplate,
}

pub fn save_contacts_to_json<P: AsRef<Path>>(
    contacts: &[ContactRecord],
    path: P,
) -> Result<(), String> {
    write_json_atomically(&contacts, path.as_ref())
}

fn write_json_atomically(value: &impl Serialize, destination: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| format!("JSON serialization failed: {}", e))?;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let temporary = destination.with_file_name(format!(
        ".{}.{}-{}.tmp",
        destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(json.as_bytes())?;
        file.sync_data()?;
        drop(file);
        fs::rename(&temporary, destination)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Could not write file {:?}: {}", destination, error));
    }
    Ok(())
}

pub fn load_contacts_from_json<P: AsRef<Path>>(path: P) -> Result<Vec<ContactRecord>, String> {
    if !path.as_ref().exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(path.as_ref())
        .map_err(|e| format!("Could not read file {:?}: {}", path.as_ref(), e))?;
    let contacts: Vec<ContactRecord> =
        serde_json::from_str(&data).map_err(|e| format!("Invalid JSON: {}", e))?;
    Ok(contacts)
}

pub fn export_contacts_to_csv<P: AsRef<Path>>(
    contacts: &[ContactRecord],
    path: P,
) -> Result<(), String> {
    let mut wtr = String::new();
    // UTF-8 BOM so Excel opens it with proper Cyrillic encoding
    wtr.push('\u{feff}');
    wtr.push_str("Email;Имя;Компания;Юрлицо;ИНН;ОГРН;VAT_ID;Описание_Компании;Адрес;B2B_Tier;Должность;Город;Стек;Телефон;Оператор_Связи;Регион_Телефона;Тип_Линии;WhatsApp_URL;Telegram_URL;GitHub_URL;Активный_Наем;Вакансии;Telegram;VK;WhatsApp;Теги;Провайдер;Role_Аккаунт;Catch_All;Gravatar;Lead_Score;Скоринг;MX_Валиден;RCPT_Проверен;Стек_Технологий;Возраст_Домена_Дней;Регистратор;Паттерн_Email;MX_Запись;Источник;Ссылка;Статус;Фоллоуап_Счетчик;Посл_Отправка;Айсбрейкер;Заметки\n");

    for c in contacts {
        let tags_str = c.tags.join(", ");
        let tech_str = c.tech_stack.join(", ");
        let roles_str = c.hiring_roles.join(", ");
        let age_str = c.domain_age_days.map(|a| a.to_string()).unwrap_or_default();
        let line = format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};\"{}\";\"{}\";\"{}\"\n",
            escape_csv(&c.email),
            escape_csv(&c.name),
            escape_csv(&c.company),
            escape_csv(&c.legal_name),
            escape_csv(&c.inn),
            escape_csv(&c.ogrn),
            escape_csv(&c.vat_id),
            escape_csv(&c.company_description),
            escape_csv(&c.address),
            escape_csv(&c.b2b_tier),
            escape_csv(&c.role),
            escape_csv(&c.city),
            escape_csv(&c.stack),
            escape_csv(&c.phone),
            escape_csv(&c.phone_carrier),
            escape_csv(&c.phone_region),
            escape_csv(&c.phone_type),
            escape_csv(&c.whatsapp_url),
            escape_csv(&c.telegram_url),
            escape_csv(&c.github_url),
            if c.hiring_active { "Yes" } else { "No" },
            escape_csv(&roles_str),
            escape_csv(&c.telegram),
            escape_csv(&c.vk_url),
            escape_csv(&c.whatsapp),
            escape_csv(&tags_str),
            escape_csv(&c.mail_provider),
            if c.is_role_account { "Yes" } else { "No" },
            if c.is_catch_all { "Yes" } else { "No" },
            if c.gravatar_found { "Yes" } else { "No" },
            c.lead_score,
            c.score,
            if c.mx_valid { "Yes" } else { "No" },
            if c.rcpt_verified { "Yes" } else { "No" },
            escape_csv(&tech_str),
            escape_csv(&age_str),
            escape_csv(&c.domain_registrar),
            escape_csv(&c.domain_pattern),
            escape_csv(&c.mx_record),
            escape_csv(&c.source),
            escape_csv(&c.url),
            c.status,
            c.follow_up_count,
            escape_csv(c.last_sent_at.as_deref().unwrap_or("")),
            escape_csv(&c.icebreaker),
            escape_csv(&c.notes)
        );
        wtr.push_str(&line);
    }

    fs::write(path.as_ref(), wtr)
        .map_err(|e| format!("Could not save CSV file {:?}: {}", path.as_ref(), e))?;
    Ok(())
}

pub fn export_contacts_to_amocrm_csv<P: AsRef<Path>>(
    contacts: &[ContactRecord],
    path: P,
) -> Result<(), String> {
    let mut wtr = String::new();
    wtr.push('\u{feff}');
    wtr.push_str("Название сделки/компании;Контакт;Должность;Рабочий email;Рабочий телефон;Город;Сайт;Теги;Telegram;WhatsApp;ИНН;Примечание\n");

    for c in contacts {
        let comp_name = if !c.company.is_empty() {
            &c.company
        } else if !c.legal_name.is_empty() {
            &c.legal_name
        } else {
            "No MX record"
        };

        let mut all_tags = c.tags.clone();
        if !c.b2b_tier.is_empty() {
            all_tags.push(c.b2b_tier.clone());
        }
        if c.hiring_active {
            all_tags.push("Hiring".to_string());
        }
        let tags_str = all_tags.join(", ");

        let notes_combined = format!(
            "{}{}{}",
            c.notes,
            if !c.icebreaker.is_empty() {
                format!(" | Tags: {}", c.icebreaker)
            } else {
                "".to_string()
            },
            if !c.company_description.is_empty() {
                format!(" | Stack: {}", c.company_description)
            } else {
                "".to_string()
            }
        );

        let line = format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\"\n",
            escape_csv(comp_name),
            escape_csv(&c.name),
            escape_csv(&c.role),
            escape_csv(&c.email),
            escape_csv(&c.phone),
            escape_csv(&c.city),
            escape_csv(&c.url),
            escape_csv(&tags_str),
            escape_csv(&c.telegram),
            escape_csv(&c.whatsapp),
            escape_csv(&c.inn),
            escape_csv(&notes_combined)
        );
        wtr.push_str(&line);
    }

    fs::write(path.as_ref(), wtr)
        .map_err(|e| format!("Could not save AmoCRM CSV file {:?}: {}", path.as_ref(), e))?;
    Ok(())
}

pub fn export_contacts_to_bitrix24_csv<P: AsRef<Path>>(
    contacts: &[ContactRecord],
    path: P,
) -> Result<(), String> {
    let mut wtr = String::new();
    wtr.push('\u{feff}');
    wtr.push_str("NAME;LAST_NAME;COMPANY_TITLE;POST;EMAIL_WORK;PHONE_WORK;WEB_WORK;ADDRESS_CITY;COMMENTS;UF_CRM_INN;UF_CRM_TELEGRAM\n");

    for c in contacts {
        let (first_name, last_name) = if let Some((f, l)) = c.name.split_once(' ') {
            (f.trim(), l.trim())
        } else {
            (c.name.as_str(), "")
        };

        let comp_name = if !c.company.is_empty() {
            &c.company
        } else {
            &c.legal_name
        };

        let comment = format!(
            "{}{}{}",
            c.notes,
            if !c.icebreaker.is_empty() {
                format!(" | Socials: {}", c.icebreaker)
            } else {
                "".to_string()
            },
            if !c.stack.is_empty() {
                format!(" | Score: {}", c.stack)
            } else {
                "".to_string()
            }
        );

        let line = format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\"\n",
            escape_csv(first_name),
            escape_csv(last_name),
            escape_csv(comp_name),
            escape_csv(&c.role),
            escape_csv(&c.email),
            escape_csv(&c.phone),
            escape_csv(&c.url),
            escape_csv(&c.city),
            escape_csv(&comment),
            escape_csv(&c.inn),
            escape_csv(&c.telegram)
        );
        wtr.push_str(&line);
    }

    fs::write(path.as_ref(), wtr).map_err(|e| {
        format!(
            "Could not save Bitrix24 CSV file {:?}: {}",
            path.as_ref(),
            e
        )
    })?;
    Ok(())
}

pub fn export_contacts_to_instantly_csv<P: AsRef<Path>>(
    contacts: &[ContactRecord],
    path: P,
) -> Result<(), String> {
    let mut wtr = String::new();
    wtr.push('\u{feff}');
    wtr.push_str("Email,First Name,Last Name,Company Name,Personalization,Phone,Website,Title,City,Custom Variable 1,Custom Variable 2,Custom Variable 3\n");

    for c in contacts {
        if c.email.trim().is_empty() {
            continue;
        }
        let (first_name, last_name) = if let Some((f, l)) = c.name.split_once(' ') {
            (f.trim(), l.trim())
        } else {
            (c.name.as_str(), "")
        };

        let comp_name = if !c.company.is_empty() {
            &c.company
        } else {
            &c.legal_name
        };

        let tech_var = c.tech_stack.join(", ");
        let hiring_var = if c.hiring_active {
            format!("Hiring: {}", c.hiring_roles.join(", "))
        } else {
            String::new()
        };

        let line = format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            escape_csv(&c.email),
            escape_csv(first_name),
            escape_csv(last_name),
            escape_csv(comp_name),
            escape_csv(&c.icebreaker),
            escape_csv(&c.phone),
            escape_csv(&c.url),
            escape_csv(&c.role),
            escape_csv(&c.city),
            escape_csv(&tech_var),
            escape_csv(&hiring_var),
            escape_csv(&c.inn)
        );
        wtr.push_str(&line);
    }

    fs::write(path.as_ref(), wtr).map_err(|e| {
        format!(
            "Could not save Instantly CSV file {:?}: {}",
            path.as_ref(),
            e
        )
    })?;
    Ok(())
}

pub fn export_contacts_to_lemlist_csv<P: AsRef<Path>>(
    contacts: &[ContactRecord],
    path: P,
) -> Result<(), String> {
    let mut wtr = String::new();
    wtr.push('\u{feff}');
    wtr.push_str("email,firstName,lastName,companyName,icebreaker,phone,website,jobTitle,city,techStack,customDomain\n");

    for c in contacts {
        if c.email.trim().is_empty() {
            continue;
        }
        let (first_name, last_name) = if let Some((f, l)) = c.name.split_once(' ') {
            (f.trim(), l.trim())
        } else {
            (c.name.as_str(), "")
        };

        let comp_name = if !c.company.is_empty() {
            &c.company
        } else {
            &c.legal_name
        };

        let domain = c.email.split_once('@').map(|(_, d)| d).unwrap_or("");
        let tech_var = c.tech_stack.join(", ");

        let line = format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            escape_csv(&c.email),
            escape_csv(first_name),
            escape_csv(last_name),
            escape_csv(comp_name),
            escape_csv(&c.icebreaker),
            escape_csv(&c.phone),
            escape_csv(&c.url),
            escape_csv(&c.role),
            escape_csv(&c.city),
            escape_csv(&tech_var),
            escape_csv(domain)
        );
        wtr.push_str(&line);
    }

    fs::write(path.as_ref(), wtr)
        .map_err(|e| format!("Could not save Lemlist CSV file {:?}: {}", path.as_ref(), e))?;
    Ok(())
}

fn escape_csv(s: &str) -> String {
    s.replace('"', "\"\"").replace('\n', " ").replace('\r', "")
}

pub fn export_contacts_to_vcf<P: AsRef<Path>>(
    contacts: &[ContactRecord],
    path: P,
) -> Result<(), String> {
    let mut vcf = String::new();
    for c in contacts {
        if c.company.trim().is_empty() && c.email.trim().is_empty() && c.phone.trim().is_empty() {
            continue;
        }
        vcf.push_str("BEGIN:VCARD\r\n");
        vcf.push_str("VERSION:3.0\r\n");

        let fn_name = if !c.company.is_empty() {
            format!(
                "{} ({})",
                c.company,
                if !c.name.is_empty() { &c.name } else { &c.city }
            )
        } else if !c.name.is_empty() {
            c.name.clone()
        } else {
            "From PoiskContact".to_string()
        };
        vcf.push_str(&format!("FN:{}\r\n", escape_vcard(&fn_name)));

        if !c.company.is_empty() {
            vcf.push_str(&format!("ORG:{}\r\n", escape_vcard(&c.company)));
        }
        if !c.role.is_empty() {
            vcf.push_str(&format!("TITLE:{}\r\n", escape_vcard(&c.role)));
        }
        if !c.phone.is_empty() {
            let clean_phone = c.phone.replace([' ', '-', '(', ')'], "");
            vcf.push_str(&format!("TEL;TYPE=CELL,VOICE:{}\r\n", clean_phone));
        }
        if !c.email.is_empty() {
            vcf.push_str(&format!("EMAIL;TYPE=WORK:{}\r\n", c.email));
        }
        if !c.url.is_empty() {
            vcf.push_str(&format!("URL:{}\r\n", c.url));
        }
        if !c.vk_url.is_empty() {
            vcf.push_str(&format!("X-SOCIALPROFILE;type=vkontakte:{}\r\n", c.vk_url));
        }
        if !c.whatsapp.is_empty() {
            vcf.push_str(&format!("X-SOCIALPROFILE;type=whatsapp:{}\r\n", c.whatsapp));
        }
        if !c.whatsapp_url.is_empty() {
            vcf.push_str(&format!("X-WHATSAPP:{}\r\n", c.whatsapp_url));
        }
        if !c.telegram_url.is_empty() {
            vcf.push_str(&format!("X-TELEGRAM:{}\r\n", c.telegram_url));
        }
        if !c.github_url.is_empty() {
            vcf.push_str(&format!("X-GITHUB:{}\r\n", c.github_url));
        }
        if !c.avatar_url.is_empty() {
            vcf.push_str(&format!("PHOTO;VALUE=uri:{}\r\n", c.avatar_url));
        }

        if !c.address.is_empty() {
            vcf.push_str(&format!(
                "ADR;TYPE=WORK:;;{};;;;\r\n",
                escape_vcard(&c.address)
            ));
        }

        let mut note_parts = Vec::new();
        if !c.legal_name.is_empty() {
            note_parts.push(format!("Status: {}", c.legal_name));
        }
        if !c.inn.is_empty() {
            note_parts.push(format!("Role: {}", c.inn));
        }
        if !c.ogrn.is_empty() {
            note_parts.push(format!("Industry: {}", c.ogrn));
        }
        if !c.vat_id.is_empty() {
            note_parts.push(format!("VAT: {}", c.vat_id));
        }
        if !c.phone_carrier.is_empty() {
            note_parts.push(format!("Source: {} ({})", c.phone_carrier, c.phone_region));
        }
        if c.hiring_active {
            note_parts.push(format!("Email pattern: {}", c.hiring_roles.join(", ")));
        }
        if !c.company_description.is_empty() {
            note_parts.push(format!("Confidence: {}", c.company_description));
        }
        if !c.b2b_tier.is_empty() {
            note_parts.push(format!("B2B Tier: {}", c.b2b_tier));
        }
        if !c.stack.is_empty() {
            note_parts.push(format!("Age: {}", c.stack));
        }
        if !c.city.is_empty() {
            note_parts.push(format!("City: {}", c.city));
        }
        if !c.telegram.is_empty() {
            note_parts.push(format!("Telegram: {}", c.telegram));
        }
        if !c.vk_url.is_empty() {
            note_parts.push(format!("VK: {}", c.vk_url));
        }
        if !c.tags.is_empty() {
            note_parts.push(format!("Tags: {}", c.tags.join(", ")));
        }
        if !c.tech_stack.is_empty() {
            note_parts.push(format!("CMS: {}", c.tech_stack.join(", ")));
        }
        if !c.icebreaker.is_empty() {
            note_parts.push(format!("CRM: {}", c.icebreaker));
        }
        if let Some(age) = c.domain_age_days {
            note_parts.push(format!("Linked pages: {}", age));
        }
        if !c.domain_pattern.is_empty() {
            note_parts.push(format!("Email typo: {}", c.domain_pattern));
        }
        if !c.notes.is_empty() {
            note_parts.push(format!("Icebreaker: {}", c.notes));
        }
        note_parts.push(format!("Open rate: {} | Replied: {}", c.score, c.status));
        vcf.push_str(&format!(
            "NOTE:{}\r\n",
            escape_vcard(&note_parts.join(" | "))
        ));

        vcf.push_str("END:VCARD\r\n");
    }

    fs::write(path.as_ref(), vcf)
        .map_err(|e| format!("Could not save vCard file {:?}: {}", path.as_ref(), e))?;
    Ok(())
}

pub fn load_suppression_list<P: AsRef<Path>>(path: P) -> crate::models::SuppressionList {
    if let Ok(content) = fs::read_to_string(path.as_ref()) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        crate::models::SuppressionList::default()
    }
}

pub fn save_suppression_list<P: AsRef<Path>>(
    list: &crate::models::SuppressionList,
    path: P,
) -> Result<(), String> {
    let data = serde_json::to_string_pretty(list)
        .map_err(|e| format!("Settings serialization failed: {}", e))?;
    fs::write(path.as_ref(), data)
        .map_err(|e| format!("Could not save settings {:?}: {}", path.as_ref(), e))
}

fn escape_vcard(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', " ")
        .replace('\r', "")
}

pub fn save_app_settings<P: AsRef<Path>>(
    smtp: &SmtpConfig,
    template: &EmailTemplate,
    path: P,
) -> Result<(), String> {
    let state = AppConfigState {
        smtp: smtp.clone(),
        template: template.clone(),
    };
    write_json_atomically(&state, path.as_ref())
}

pub fn save_email_template(
    smtp: &SmtpConfig,
    template: &EmailTemplate,
    path: &Path,
) -> Result<(), String> {
    let saved_smtp = if path.exists() {
        let data = fs::read_to_string(path)
            .map_err(|error| format!("Could not read existing settings: {error}"))?;
        let state: AppConfigState = serde_json::from_str(&data)
            .map_err(|error| format!("Existing settings are invalid; file preserved: {error}"))?;
        state.smtp
    } else {
        smtp.clone()
    };
    save_app_settings(&saved_smtp, template, path)
}

pub fn load_email_template(path: &Path) -> Result<EmailTemplate, String> {
    let data = fs::read_to_string(path)
        .map_err(|error| format!("Could not open saved template: {error}"))?;
    let state: AppConfigState = serde_json::from_str(&data)
        .map_err(|error| format!("Could not read saved template: {error}"))?;
    Ok(state.template)
}

pub fn load_app_settings<P: AsRef<Path>>(path: P) -> Option<(SmtpConfig, EmailTemplate)> {
    if !path.as_ref().exists() {
        return None;
    }
    let data = fs::read_to_string(path.as_ref()).ok()?;
    let state: AppConfigState = serde_json::from_str(&data).ok()?;
    Some((state.smtp, state.template))
}

pub fn parse_csv_line(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                current.push('"');
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
        } else if ch == delimiter && !in_quotes {
            fields.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    fields.push(current.trim().to_string());
    fields
}

pub fn import_external_contacts_file<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<ContactRecord>, String> {
    let path_ref = path.as_ref();
    if !path_ref.exists() {
        return Err(format!("Файл не найден: {:?}", path_ref));
    }

    let ext = path_ref
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let raw_data = fs::read_to_string(path_ref)
        .map_err(|e| format!("Ошибка чтения файла {:?}: {}", path_ref, e))?;

    if ext == "json" {
        if let Ok(records) = serde_json::from_str::<Vec<ContactRecord>>(&raw_data) {
            return Ok(records);
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw_data) {
            if let Some(arr) = val.as_array() {
                let mut results = Vec::new();
                for item in arr {
                    let company = item
                        .get("company")
                        .or_else(|| item.get("name"))
                        .or_else(|| item.get("название"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let email = item
                        .get("email")
                        .or_else(|| item.get("почта"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let phone = item
                        .get("phone")
                        .or_else(|| item.get("телефон"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let site = item
                        .get("url")
                        .or_else(|| item.get("site"))
                        .or_else(|| item.get("сайт"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let city = item
                        .get("city")
                        .or_else(|| item.get("город"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Красноярск");
                    let stack = item
                        .get("stack")
                        .or_else(|| item.get("category"))
                        .or_else(|| item.get("рубрика"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("2GIS / Каталог");

                    if !email.is_empty() || !company.is_empty() {
                        results.push(ContactRecord::new(
                            company,
                            email,
                            company,
                            "Руководитель / Контактное лицо",
                            city,
                            stack,
                            phone,
                            "",
                            "Импорт 2GIS/JSON",
                            site,
                            crate::models::LeadScore::Warm,
                            !email.is_empty(),
                            "",
                        ));
                    }
                }
                if !results.is_empty() {
                    return Ok(results);
                }
            }
        }
    }

    // CSV parser (stripping UTF-8 BOM if present)
    let content = raw_data.strip_prefix('\u{feff}').unwrap_or(&raw_data);
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header_line = match lines.next() {
        Some(h) => h,
        None => return Ok(Vec::new()),
    };

    let delimiter = if header_line.contains(';') { ';' } else { ',' };
    let headers = parse_csv_line(header_line, delimiter);

    let mut idx_email = None;
    let mut idx_name = None;
    let mut idx_company = None;
    let mut idx_role = None;
    let mut idx_phone = None;
    let mut idx_tg = None;
    let mut idx_city = None;
    let mut idx_stack = None;
    let mut idx_url = None;

    for (i, h) in headers.iter().enumerate() {
        let hl = h.to_lowercase();
        if hl.contains("email") || hl.contains("почта") || hl.contains("mail") {
            if idx_email.is_none() {
                idx_email = Some(i);
            }
        } else if hl.contains("компания")
            || hl.contains("наименование")
            || hl.contains("организация")
            || hl == "company"
        {
            if idx_company.is_none() {
                idx_company = Some(i);
            }
        } else if hl.contains("имя") || hl.contains("фио") || hl == "name" {
            if idx_name.is_none() {
                idx_name = Some(i);
            }
        } else if hl.contains("должность") || hl == "role" {
            if idx_role.is_none() {
                idx_role = Some(i);
            }
        } else if hl.contains("телефон") || hl.contains("тел") || hl == "phone" {
            if idx_phone.is_none() {
                idx_phone = Some(i);
            }
        } else if hl.contains("telegram") || hl.contains("телеграм") || hl == "tg" {
            if idx_tg.is_none() {
                idx_tg = Some(i);
            }
        } else if hl.contains("город") || hl == "city" {
            if idx_city.is_none() {
                idx_city = Some(i);
            }
        } else if hl.contains("стек")
            || hl.contains("рубрика")
            || hl.contains("категория")
            || hl == "stack"
        {
            if idx_stack.is_none() {
                idx_stack = Some(i);
            }
        } else if hl.contains("сайт")
            || hl.contains("url")
            || hl.contains("ссылка")
            || hl == "website"
        {
            if idx_url.is_none() {
                idx_url = Some(i);
            }
        }
    }

    let mut records = Vec::new();
    let re_email = regex::Regex::new(r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+").unwrap();

    for line in lines {
        let fields = parse_csv_line(line, delimiter);
        let mut email = idx_email
            .and_then(|idx| fields.get(idx))
            .map(|s| s.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if email.is_empty() {
            // Check if any field contains an email address
            for f in &fields {
                if let Some(m) = re_email.find(f) {
                    email = m.as_str().to_lowercase();
                    break;
                }
            }
        }

        let company = idx_company
            .and_then(|idx| fields.get(idx))
            .map(|s| s.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let name = idx_name
            .and_then(|idx| fields.get(idx))
            .map(|s| s.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let role = idx_role
            .and_then(|idx| fields.get(idx))
            .map(|s| s.as_str())
            .unwrap_or("Руководитель")
            .trim()
            .to_string();
        let phone = idx_phone
            .and_then(|idx| fields.get(idx))
            .map(|s| s.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let tg = idx_tg
            .and_then(|idx| fields.get(idx))
            .map(|s| s.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let city = idx_city
            .and_then(|idx| fields.get(idx))
            .map(|s| s.as_str())
            .unwrap_or("Красноярск")
            .trim()
            .to_string();
        let stack = idx_stack
            .and_then(|idx| fields.get(idx))
            .map(|s| s.as_str())
            .unwrap_or("Импорт CSV")
            .trim()
            .to_string();
        let url = idx_url
            .and_then(|idx| fields.get(idx))
            .map(|s| s.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if !company.is_empty() || !email.is_empty() {
            let final_name = if !name.is_empty() {
                name
            } else if !company.is_empty() {
                company.clone()
            } else {
                "Контактное лицо".to_string()
            };
            records.push(ContactRecord::new(
                &final_name,
                &email,
                &company,
                &role,
                &city,
                &stack,
                &phone,
                &tg,
                "Импорт CSV/2GIS",
                &url,
                crate::models::LeadScore::Warm,
                !email.is_empty(),
                "",
            ));
        }
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_save_roundtrip_preserves_smtp_and_rejects_corrupt_settings() {
        let directory = std::env::temp_dir().join(format!(
            "poisk-template-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("settings.json");
        let smtp = SmtpConfig {
            username: "saved@example.org".into(),
            password: "synthetic-secret".into(),
            ..Default::default()
        };
        save_app_settings(&smtp, &EmailTemplate::default(), &path).unwrap();
        let edited_smtp = SmtpConfig {
            username: "unsaved@example.org".into(),
            ..Default::default()
        };
        let template = EmailTemplate {
            subject: "Съёмка для {company}".into(),
            body: "Здравствуйте, {name}!\nНовая строка и кириллица.".repeat(150),
            ..Default::default()
        };
        save_email_template(&edited_smtp, &template, &path).unwrap();
        let loaded = load_email_template(&path).unwrap();
        assert_eq!(loaded.subject, template.subject);
        assert_eq!(loaded.body, template.body);
        let (saved_smtp, _) = load_app_settings(&path).unwrap();
        assert_eq!(
            serde_json::to_value(saved_smtp).unwrap(),
            serde_json::to_value(smtp).unwrap()
        );
        fs::write(&path, "broken JSON").unwrap();
        assert!(save_email_template(&edited_smtp, &template, &path).is_err());
        assert!(load_email_template(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "broken JSON");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn settings_save_reports_missing_destination_directory() {
        let missing = std::env::temp_dir()
            .join(format!(
                "poisk-settings-missing-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap()
            ))
            .join("settings.json");
        let result = save_app_settings(&SmtpConfig::default(), &EmailTemplate::default(), missing);
        assert!(result.is_err());
    }

    #[test]
    fn test_amocrm_and_bitrix24_export() {
        let temp_dir = std::env::temp_dir();
        let amocrm_path = temp_dir.join("test_amocrm.csv");
        let bitrix_path = temp_dir.join("test_bitrix24.csv");

        let mut c = ContactRecord::new(
            "Иван Петров",
            "ivan@corp.ru",
            "ООО Корпорация",
            "CTO",
            "Москва",
            "Rust, Docker",
            "+1 202 555 0100",
            "@ivan_cto",
            "Сайт",
            "https://corp.ru",
            crate::models::LeadScore::Hot,
            true,
            "mx.corp.ru",
        );
        c.inn = "7707083893".to_string();
        c.icebreaker = "Заметил использование Rust".to_string();

        let contacts = vec![c];

        assert!(export_contacts_to_amocrm_csv(&contacts, &amocrm_path).is_ok());
        let amocrm_content = fs::read_to_string(&amocrm_path).unwrap();
        assert!(amocrm_content.contains("Название сделки/компании"));
        assert!(amocrm_content.contains("ООО Корпорация"));
        assert!(amocrm_content.contains("7707083893"));
        let _ = fs::remove_file(amocrm_path);

        assert!(export_contacts_to_bitrix24_csv(&contacts, &bitrix_path).is_ok());
        let bitrix_content = fs::read_to_string(&bitrix_path).unwrap();
        assert!(bitrix_content.contains("COMPANY_TITLE"));
        assert!(bitrix_content.contains("Иван"));
        assert!(bitrix_content.contains("Петров"));
        assert!(bitrix_content.contains("UF_CRM_INN"));
        let _ = fs::remove_file(bitrix_path);
    }

    #[test]
    fn test_instantly_and_lemlist_export() {
        let temp_dir = std::env::temp_dir();
        let instantly_path = temp_dir.join("test_instantly.csv");
        let lemlist_path = temp_dir.join("test_lemlist.csv");

        let mut c = ContactRecord::new(
            "Павел Дуров",
            "pavel@telegram.org",
            "Telegram FZ-LLC",
            "Founder",
            "Dubai",
            "C++, Rust",
            "+971501234567",
            "@durov",
            "Telegram",
            "https://telegram.org",
            crate::models::LeadScore::Hot,
            true,
            "mx.telegram.org",
        );
        c.icebreaker = "Impressive privacy commitment".to_string();
        c.tech_stack = vec!["C++".to_string(), "PostgreSQL".to_string()];

        let contacts = vec![c];

        assert!(export_contacts_to_instantly_csv(&contacts, &instantly_path).is_ok());
        let inst_content = fs::read_to_string(&instantly_path).unwrap();
        assert!(inst_content.contains("Personalization"));
        assert!(inst_content.contains("pavel@telegram.org"));
        assert!(inst_content.contains("Павел"));
        assert!(inst_content.contains("Дуров"));
        assert!(inst_content.contains("Impressive privacy commitment"));
        let _ = fs::remove_file(instantly_path);

        assert!(export_contacts_to_lemlist_csv(&contacts, &lemlist_path).is_ok());
        let lem_content = fs::read_to_string(&lemlist_path).unwrap();
        assert!(lem_content.contains("customDomain"));
        assert!(lem_content.contains("pavel@telegram.org"));
        assert!(lem_content.contains("telegram.org"));
        let _ = fs::remove_file(lemlist_path);
    }
}
