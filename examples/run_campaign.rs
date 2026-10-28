use poisk_contact::{
    mailer::{self, render_template},
    models::SendStatus,
    storage::{
        export_contacts_to_csv, load_app_settings, load_contacts_from_json, save_contacts_to_json,
    },
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let limit_override: Option<usize> = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|pos| args.get(pos + 1))
        .and_then(|val| val.parse().ok());

    let (mut config, template) =
        load_app_settings("settings.json").ok_or("Failed to load settings.json")?;
    let mut contacts = match load_contacts_from_json("contacts.json") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] Не удалось загрузить contacts.json: {}", e);
            return Err(e.into());
        }
    };

    let pending_indices: Vec<usize> = contacts
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.email.trim().is_empty() && c.status == SendStatus::Pending)
        .map(|(i, _)| i)
        .collect();

    let total_pending = pending_indices.len();
    if total_pending == 0 {
        println!("[INFO] Все контакты с email уже отправлены или база пуста.");
        return Ok(());
    }

    let batch_size = if let Some(lim) = limit_override {
        lim.min(total_pending)
    } else if config.daily_limit > 0 {
        (config.daily_limit as usize)
            .saturating_sub(config.sent_today as usize)
            .min(total_pending)
    } else {
        total_pending
    };

    println!("==================================================");
    println!("  POISK_CONTACT — ЗАПУСК КАМПАНИИ АУТРИЧА");
    println!("  Режим: Direct MX (порт 25, TLS + DKIM)");
    println!(
        "  Отправитель: {} <{}>",
        config.sender_name, config.from_email
    );
    println!("  Reply-To: {}", config.reply_to);
    println!("  Доступно для отправки: {} контактов", total_pending);
    println!("  План на эту сессию: {} контактов", batch_size);
    println!(
        "  Задержка между письмами: {} сек (random: {})",
        config.delay_seconds, config.random_delay
    );
    println!("==================================================");

    let mut success_count = 0;
    let mut fail_count = 0;

    for (order, &idx) in pending_indices.iter().take(batch_size).enumerate() {
        let contact = &contacts[idx];
        let email = contact.email.trim().to_string();
        let company = contact.company.clone();
        let name = contact.name.clone();

        let subject = render_template(&template.subject, contact, &config.sender_name);
        let body = render_template(&template.body, contact, &config.sender_name);

        println!(
            "[{}/{}] Отправка на {} (Компания: {}, Имя: {})...",
            order + 1,
            batch_size,
            email,
            company,
            name
        );
        println!("       Тема: {}", subject);

        let send_result = mailer::send_direct_mx_with_headers(
            &email,
            &config,
            &subject,
            &body,
            contact.last_message_id.as_deref(),
        )
        .await;

        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        match send_result {
            Ok(msg_id) => {
                success_count += 1;
                config.sent_today += 1;
                let c_mut = &mut contacts[idx];
                c_mut.status = SendStatus::Sent;
                c_mut.last_sent_at = Some(now_str);
                c_mut.last_message_id = Some(msg_id.clone());
                c_mut.follow_up_count = 1;

                println!("       -> [УСПЕХ] Доставлено на сервер получателя (250 OK)");
                println!("       -> ID: {}", msg_id);
            }
            Err(e) => {
                fail_count += 1;
                let c_mut = &mut contacts[idx];
                c_mut.status = SendStatus::Failed(e.clone());
                println!("       -> [ОШИБКА] Не удалось отправить: {}", e);
            }
        }

        // Сохраняем после каждого письма, чтобы прогресс не терялся
        let _ = save_contacts_to_json(&contacts, "contacts.json");
        let _ = export_contacts_to_csv(&contacts, "contacts.csv");

        // Пауза перед следующим письмом
        if order + 1 < batch_size {
            let delay_sec = if config.random_delay && config.delay_seconds > 1 {
                let jitter = (order % 4) as u64;
                (config.delay_seconds as u64).saturating_add(jitter)
            } else {
                config.delay_seconds.max(2) as u64
            };
            println!(
                "       Ожидание {} сек перед следующим письмом...",
                delay_sec
            );
            sleep(Duration::from_secs(delay_sec)).await;
        }
    }

    println!("==================================================");
    println!("  КАМПАНИЯ ЗАВЕРШЕНА");
    println!("  Успешно отправлено: {}", success_count);
    println!("  Ошибок: {}", fail_count);
    println!("  База контактов обновлена в contacts.json и contacts.csv");
    println!("==================================================");

    Ok(())
}
