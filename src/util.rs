//! Время и даты.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SECS_PER_DAY: u64 = 86_400;

/// Смещение местного времени от UTC в секундах. Пока не задано — UTC, поэтому тесты не зависят от
/// часового пояса машины; приложение подставляет настоящее значение при запуске.
static UTC_OFFSET_SECS: AtomicI64 = AtomicI64::new(0);

/// Текущее время, секунды с Unix-эпохи.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Смещение часового пояса системы от UTC, секунды.
pub fn system_utc_offset() -> i64 {
    i64::from(chrono::Local::now().offset().local_minus_utc())
}

/// С этого момента «сутки» — календарные сутки пользователя. Пояс читается один раз при запуске;
/// перевод часов посреди сеанса учтётся при следующем запуске.
pub fn use_local_timezone() {
    UTC_OFFSET_SECS.store(system_utc_offset(), Ordering::Relaxed);
}

/// Номер дня с 1970-01-01 для момента `secs` при заданном смещении от UTC.
pub fn day_at(secs: u64, utc_offset: i64) -> u64 {
    let local = i64::try_from(secs)
        .unwrap_or(i64::MAX)
        .saturating_add(utc_offset);
    u64::try_from(local.div_euclid(SECS_PER_DAY as i64)).unwrap_or(0)
}

/// Номер календарного дня (по местному времени) с 1970-01-01.
pub fn local_day(secs: u64) -> u64 {
    day_at(secs, UTC_OFFSET_SECS.load(Ordering::Relaxed))
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

/// `ГГГГ-ММ-ДД ЧЧ:ММ:СС` (UTC) для момента с Unix-эпохи.
pub fn format_datetime(secs: u64) -> String {
    let rest = secs % SECS_PER_DAY;
    format!(
        "{} {:02}:{:02}:{:02}",
        format_day(secs / SECS_PER_DAY),
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
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
    fn datetime_is_utc_with_seconds() {
        assert_eq!(format_datetime(0), "1970-01-01 00:00:00");
        assert_eq!(format_datetime(1_000_000_000), "2001-09-09 01:46:40");
        assert_eq!(format_datetime(86_399), "1970-01-01 23:59:59");
    }

    #[test]
    fn duration_formats_minutes_and_hours() {
        assert_eq!(format_duration(0), "0 мин");
        assert_eq!(format_duration(59 * 60 + 59), "59 мин");
        assert_eq!(format_duration(5_400), "1.5 ч");
    }

    #[test]
    fn day_boundary_follows_the_utc_offset() {
        let midnight_utc = 20_725 * SECS_PER_DAY;
        assert_eq!(day_at(midnight_utc, 0), 20_725);
        assert_eq!(day_at(midnight_utc - 1, 0), 20_724);
        // Москва (UTC+3): 23:30 по UTC — уже следующие сутки
        assert_eq!(day_at(midnight_utc - 1_800, 3 * 3_600), 20_725);
        // Нью-Йорк (UTC−4): 01:00 по UTC — ещё вчера
        assert_eq!(day_at(midnight_utc + 3_600, -4 * 3_600), 20_724);
        assert_eq!(day_at(0, -12 * 3_600), 0, "до эпохи не уходим");
        assert_eq!(
            day_at(u64::MAX, 14 * 3_600),
            (i64::MAX as u64) / SECS_PER_DAY
        );
    }

    #[test]
    fn system_offset_is_a_real_timezone() {
        // Только читаем пояс (глобальное смещение не трогаем: остальные тесты считают сутки по UTC).
        assert!((-14 * 3_600..=14 * 3_600).contains(&system_utc_offset()));
    }
}
