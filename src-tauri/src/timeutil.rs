use std::time::{SystemTime, UNIX_EPOCH};

pub const DAY: i64 = 86_400;

pub fn now() -> i64 {
    unix(SystemTime::now())
}

pub fn unix(t: SystemTime) -> i64 {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => 0,
    }
}

pub fn unix_res(t: std::io::Result<SystemTime>) -> i64 {
    t.map(unix).unwrap_or(0)
}

/// Local wall-clock (year, month, day, hour).
pub fn local_now() -> (u16, u16, u16, u16) {
    let st = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    (st.wYear, st.wMonth, st.wDay, st.wHour)
}

pub fn month_stamp() -> String {
    let (y, m, _, _) = local_now();
    format!("{y:04}-{m:02}")
}

pub fn day_stamp() -> String {
    let (y, m, d, _) = local_now();
    format!("{y:04}-{m:02}-{d:02}")
}

/// "3 days", "5 weeks", "4 months", "over a year".
pub fn human_age(days: i64) -> String {
    match days {
        i64::MIN..=0 => "today".into(),
        1 => "a day".into(),
        2..=13 => format!("{days} days"),
        14..=59 => format!("{} weeks", days / 7),
        60..=364 => format!("{} months", days / 30),
        365..=729 => "over a year".into(),
        _ => format!("{} years", days / 365),
    }
}

pub fn human_bytes(b: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else if v >= 100.0 {
        format!("{v:.0} {}", U[i])
    } else {
        format!("{v:.1} {}", U[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ages() {
        assert_eq!(human_age(3), "3 days");
        assert_eq!(human_age(21), "3 weeks");
        assert_eq!(human_age(150), "5 months");
        assert_eq!(human_age(800), "2 years");
    }

    #[test]
    fn bytes() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
