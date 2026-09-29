//! Время и даты без внешних зависимостей.

use std::time::{SystemTime, UNIX_EPOCH};

pub const SECS_PER_DAY: u64 = 86_400;

/// Текущее время, секунды с Unix-эпохи.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Номер дня (UTC) с Unix-эпохи.
pub fn unix_day(secs: u64) -> u64 {
    secs / SECS_PER_DAY
}

/// Дата (год, месяц, день) по числу дней с 1970-01-01, григорианский календарь.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// `ГГГГ-ММ-ДД` для номера дня с эпохи.
pub fn format_day(day: u64) -> String {
    let (y, m, d) = civil_from_days(day as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// «42 мин» или «1.5 ч».
pub fn format_duration(secs: u64) -> String {
    if secs >= 3600 {
        format!("{:.1} ч", secs as f32 / 3600.0)
    } else {
        format!("{} мин", secs / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_match_known_days() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29)); // високосный день
        assert_eq!(civil_from_days(20_725), (2026, 9, 29));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(format_day(20_725), "2026-09-29");
    }

    #[test]
    fn duration_formats_minutes_and_hours() {
        assert_eq!(format_duration(0), "0 мин");
        assert_eq!(format_duration(59 * 60 + 59), "59 мин");
        assert_eq!(format_duration(5_400), "1.5 ч");
    }
}
