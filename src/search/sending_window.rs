use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendingWindowResult {
    pub is_active: bool,
    pub target_offset_hours: i32,
    pub local_time_formatted: String,
    pub day_of_week: String,
    pub window_hours: String,
    pub minutes_until_next_window: u32,
    pub status_message: String,
}

pub fn infer_utc_offset_from_target(target: &str) -> i32 {
    let lower = target.trim().to_lowercase();

    // Check direct numeric strings like "+3", "-5", "3", "-8"
    if let Ok(num) = target.trim().parse::<i32>() {
        if num >= -12 && num <= 14 {
            return num;
        }
    }
    if (lower.starts_with('+') || lower.starts_with('-')) && lower.len() >= 2 {
        if let Ok(num) = lower.parse::<i32>() {
            if num >= -12 && num <= 14 {
                return num;
            }
        }
    }

    // Common named timezones
    if lower == "msk"
        || lower.contains("moscow")
        || lower.ends_with(".ru")
        || lower.ends_with(".рф")
        || lower.ends_with(".by")
    {
        return 3;
    }
    if lower == "utc" || lower == "gmt" || lower.ends_with(".uk") || lower.contains("london") {
        return 0;
    }
    if lower == "cet"
        || lower.contains("berlin")
        || lower.contains("paris")
        || lower.ends_with(".de")
        || lower.ends_with(".fr")
    {
        return 1;
    }
    if lower == "eet"
        || lower.contains("kyiv")
        || lower.contains("helsinki")
        || lower.ends_with(".ua")
    {
        return 2;
    }
    if lower == "est" || lower.contains("new york") || lower.ends_with(".us") {
        return -5;
    }
    if lower == "cst" || lower.contains("chicago") {
        return -6;
    }
    if lower == "pst" || lower.contains("los angeles") || lower.contains("california") {
        return -8;
    }
    if lower.contains("almaty") || lower.ends_with(".kz") {
        return 5;
    }
    if lower.contains("tashkent") || lower.ends_with(".uz") {
        return 5;
    }

    // Default to MSK UTC+3 for domestic B2B operations
    3
}

pub fn check_sending_window(
    target_utc_offset_hours: i32,
    now_utc: DateTime<Utc>,
) -> SendingWindowResult {
    let local_time = now_utc + Duration::hours(target_utc_offset_hours as i64);
    let weekday = local_time.weekday();
    let hour = local_time.hour();
    let minute = local_time.minute();

    let day_name = match weekday {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    };

    let local_formatted = format!(
        "{} {:02}:{:02} (UTC{:+})",
        day_name, hour, minute, target_utc_offset_hours
    );
    let window_hours = "09:00 - 18:00 Mon-Fri".to_string();

    let is_weekday = matches!(
        weekday,
        Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri
    );
    let is_business_hour = (9..18).contains(&hour);

    if is_weekday && is_business_hour {
        return SendingWindowResult {
            is_active: true,
            target_offset_hours: target_utc_offset_hours,
            local_time_formatted: local_formatted,
            day_of_week: day_name.to_string(),
            window_hours,
            minutes_until_next_window: 0,
            status_message: format!(
                "Sending slot active: Target local time is within business hours ({}). Dispatch allowed.",
                day_name
            ),
        };
    }

    // Calculate wait time in minutes
    let minutes_until = calculate_minutes_to_next_window(weekday, hour, minute);

    let reason = if !is_weekday {
        format!(
            "Weekend blocked: Cold B2B emails sent on {} suffer severely degraded open rates and trigger anti-spam heuristics.",
            day_name
        )
    } else if hour < 9 {
        format!(
            "Early morning blocked: Local time is {:02}:{:02}. Standard business hours begin at 09:00.",
            hour, minute
        )
    } else {
        format!(
            "Evening/Night blocked: Local time is {:02}:{:02}. Standard business hours ended at 18:00.",
            hour, minute
        )
    };

    SendingWindowResult {
        is_active: false,
        target_offset_hours: target_utc_offset_hours,
        local_time_formatted: local_formatted,
        day_of_week: day_name.to_string(),
        window_hours,
        minutes_until_next_window: minutes_until,
        status_message: format!(
            "{} Next sending window opens in {} minutes.",
            reason, minutes_until
        ),
    }
}

fn calculate_minutes_to_next_window(weekday: Weekday, hour: u32, minute: u32) -> u32 {
    let current_day_minute = hour * 60 + minute;
    let target_start_minute: u32 = 9 * 60; // 09:00 = 540 min

    match weekday {
        Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu => {
            if hour < 9 {
                target_start_minute.saturating_sub(current_day_minute)
            } else {
                // Next day at 09:00
                (24 * 60 - current_day_minute) + target_start_minute
            }
        }
        Weekday::Fri => {
            if hour < 9 {
                target_start_minute.saturating_sub(current_day_minute)
            } else {
                // Monday at 09:00 (remaining Fri + Sat 24h + Sun 24h + Mon 09:00)
                (24 * 60 - current_day_minute) + (48 * 60) + target_start_minute
            }
        }
        Weekday::Sat => {
            // Remaining Sat + Sun 24h + Mon 09:00
            (24 * 60 - current_day_minute) + (24 * 60) + target_start_minute
        }
        Weekday::Sun => {
            // Remaining Sun + Mon 09:00
            (24 * 60 - current_day_minute) + target_start_minute
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_active_window_tuesday_afternoon() {
        // Tuesday 2026-10-13 11:00 UTC. Offset +3 (MSK) -> 14:00 Tuesday (Within window)
        let utc_dt = Utc.with_ymd_and_hms(2026, 10, 13, 11, 0, 0).unwrap();
        let res = check_sending_window(3, utc_dt);
        assert!(res.is_active);
        assert_eq!(res.minutes_until_next_window, 0);
        assert_eq!(res.day_of_week, "Tuesday");
        assert!(res.status_message.contains("Dispatch allowed"));
    }

    #[test]
    fn test_blocked_window_sunday_night() {
        // Sunday 2026-10-11 20:00 UTC. Offset +3 (MSK) -> 23:00 Sunday (Weekend)
        let utc_dt = Utc.with_ymd_and_hms(2026, 10, 11, 20, 0, 0).unwrap();
        let res = check_sending_window(3, utc_dt);
        assert!(!res.is_active);
        assert_eq!(res.day_of_week, "Sunday");
        assert!(res.minutes_until_next_window > 0);
        assert!(res.status_message.contains("Weekend blocked"));
    }

    #[test]
    fn test_blocked_window_early_morning() {
        // Wednesday 2026-10-14 03:00 UTC. Offset +3 (MSK) -> 06:00 Wednesday (Early morning)
        let utc_dt = Utc.with_ymd_and_hms(2026, 10, 14, 3, 0, 0).unwrap();
        let res = check_sending_window(3, utc_dt);
        assert!(!res.is_active);
        assert_eq!(res.day_of_week, "Wednesday");
        // 06:00 to 09:00 is 180 minutes
        assert_eq!(res.minutes_until_next_window, 180);
        assert!(res.status_message.contains("Early morning blocked"));
    }

    #[test]
    fn test_offset_inference() {
        assert_eq!(infer_utc_offset_from_target("company.ru"), 3);
        assert_eq!(infer_utc_offset_from_target("msk"), 3);
        assert_eq!(infer_utc_offset_from_target("berlin.de"), 1);
        assert_eq!(infer_utc_offset_from_target("service.us"), -5);
        assert_eq!(infer_utc_offset_from_target("+4"), 4);
        assert_eq!(infer_utc_offset_from_target("-8"), -8);
    }
}
