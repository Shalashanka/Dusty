//! Turns loose questions into search filters:
//! "find me a pdf with invoice in its name downloaded roughly 5 months ago"
//! -> terms [invoice], ext {pdf}, modified 3.75..6.25 months ago.

use crate::timeutil::{DAY, now};
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub terms: Vec<String>,
    pub exts: Vec<String>,
    pub dirs_only: bool,
    pub files_only: bool,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub after: Option<i64>,
    pub before: Option<i64>,
    /// Folder names that must appear in the path ("downloads", "c:").
    pub location: Vec<String>,
    /// Human-readable chips explaining how the query was understood.
    pub explain: Vec<String>,
}

const STOP: &[&str] = &[
    "find",
    "me",
    "a",
    "an",
    "the",
    "file",
    "files",
    "with",
    "something",
    "like",
    "named",
    "called",
    "that",
    "which",
    "is",
    "was",
    "has",
    "its",
    "it's",
    "it",
    "name",
    "roughly",
    "about",
    "around",
    "approximately",
    "i",
    "my",
    "some",
    "of",
    "and",
    "or",
    "for",
    "show",
    "where",
    "search",
    "look",
    "get",
    "open",
    "please",
    "dusty",
    "downloaded",
    "saved",
    "created",
    "modified",
    "made",
    "got",
    "had",
    "in",
    "on",
    "from",
    "inside",
    "under",
    "ago",
    "where's",
    "whats",
    "what's",
    "thing",
    "stuff",
    "contains",
    "containing",
];

fn type_exts(word: &str) -> Option<(&'static str, &'static [&'static str])> {
    Some(match word {
        "pdf" | "pdfs" => ("PDFs", &["pdf"]),
        "doc" | "docs" | "word" | "document" | "documents" => (
            "documents",
            &["doc", "docx", "odt", "rtf", "txt", "md", "pdf"],
        ),
        "excel" | "spreadsheet" | "spreadsheets" | "xlsx" | "sheet" | "sheets" => {
            ("spreadsheets", &["xls", "xlsx", "csv", "ods"])
        }
        "slides" | "presentation" | "presentations" | "powerpoint" | "pptx" => {
            ("presentations", &["ppt", "pptx", "odp", "key"])
        }
        "image" | "images" | "photo" | "photos" | "picture" | "pictures" | "pic" | "pics"
        | "screenshot" | "screenshots" => (
            "images",
            &[
                "png", "jpg", "jpeg", "gif", "webp", "heic", "bmp", "tif", "tiff", "svg", "raw",
                "cr2", "nef",
            ],
        ),
        "video" | "videos" | "movie" | "movies" | "clip" | "clips" | "recording" | "recordings" => {
            (
                "videos",
                &["mp4", "mkv", "mov", "avi", "webm", "wmv", "m4v"],
            )
        }
        "song" | "songs" | "music" | "audio" | "track" | "tracks" | "podcast" => {
            ("audio", &["mp3", "wav", "flac", "ogg", "m4a", "aac"])
        }
        "zip" | "zips" | "archive" | "archives" | "rar" | "7z" => {
            ("archives", &["zip", "7z", "rar", "tar", "gz", "tgz", "xz"])
        }
        "installer" | "installers" | "setup" | "exe" | "program" | "programs" | "msi" => {
            ("installers", &["exe", "msi", "msix", "appx"])
        }
        "script" | "scripts" | "bat" | "powershell" => {
            ("scripts", &["bat", "cmd", "ps1", "sh", "py", "ahk", "vbs"])
        }
        "font" | "fonts" => ("fonts", &["ttf", "otf", "woff", "woff2"]),
        "iso" | "image-file" => ("disk images", &["iso", "img", "vhd", "vhdx"]),
        "ebook" | "ebooks" | "book" | "books" | "epub" => {
            ("ebooks", &["epub", "mobi", "azw3", "pdf"])
        }
        "psd" | "design" | "designs" | "figma" => (
            "design files",
            &["psd", "ai", "fig", "sketch", "xd", "blend", "afdesign"],
        ),
        _ => return None,
    })
}

fn unit_days(w: &str) -> Option<i64> {
    Some(match w.trim_end_matches('s') {
        "day" => 1,
        "week" => 7,
        "month" => 30,
        "year" => 365,
        "hour" => 0,
        _ => return None,
    })
}

fn parse_size(num: &str, unit: &str) -> Option<u64> {
    let n: f64 = num.parse().ok()?;
    let m = match unit.trim_end_matches('s') {
        "kb" | "k" => 1024.0,
        "mb" | "m" | "meg" => 1_048_576.0,
        "gb" | "g" | "gig" => 1_073_741_824.0,
        "tb" => 1_099_511_627_776.0,
        _ => return None,
    };
    Some((n * m) as u64)
}

const MONTHS: &[&str] = &[
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

pub fn parse(input: &str) -> Query {
    let mut q = Query::default();
    let t = now();
    let lower = input.to_lowercase();

    // Quoted phrases are taken literally.
    let mut rest = String::new();
    let mut in_quote = false;
    let mut phrase = String::new();
    for ch in lower.chars() {
        if ch == '"' {
            if in_quote && !phrase.trim().is_empty() {
                q.terms.push(phrase.trim().to_string());
            }
            phrase.clear();
            in_quote = !in_quote;
        } else if in_quote {
            phrase.push(ch);
        } else {
            rest.push(ch);
        }
    }

    let words: Vec<String> = rest
        .split(|c: char| c.is_whitespace() || c == ',' || c == '?')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_string())
        .collect();
    let fuzzy = lower.contains("roughly")
        || lower.contains("about")
        || lower.contains("around")
        || lower.contains("approx");
    let mut used = vec![false; words.len()];
    let mut i = 0;
    while i < words.len() {
        let w = words[i].as_str();
        let next = words.get(i + 1).map(|s| s.as_str()).unwrap_or("");
        let next2 = words.get(i + 2).map(|s| s.as_str()).unwrap_or("");

        // "5 months ago", "a week ago"
        let num = if w == "a" || w == "an" || w == "one" {
            Some(1.0)
        } else {
            w.parse::<f64>().ok()
        };
        if let (Some(n), Some(days)) = (num, unit_days(next)) {
            if next2 == "ago"
                || words
                    .get(i.wrapping_sub(1))
                    .map(|p| p == "last" || p == "past")
                    .unwrap_or(false)
            {
                let center = t - (n * days as f64 * DAY as f64) as i64;
                let spread = ((n * days as f64 * DAY as f64) * if fuzzy { 0.3 } else { 0.2 })
                    .max(DAY as f64 * 2.0) as i64;
                if next2 == "ago" {
                    q.after = Some(center - spread);
                    q.before = Some(center + spread);
                    q.explain.push(format!(
                        "{}~{} {} ago",
                        if fuzzy { "" } else { "" },
                        n,
                        next
                    ));
                } else {
                    q.after = Some(center);
                    q.explain.push(format!("last {n} {next}"));
                }
                used[i] = true;
                used[i + 1] = true;
                if next2 == "ago" {
                    used[i + 2] = true;
                }
                i += 3;
                continue;
            }
        }
        // "bigger than 2 gb", "over 500mb", "under 10 mb"
        if matches!(
            w,
            "bigger" | "larger" | "over" | "above" | "smaller" | "under" | "below" | ">" | "<"
        ) {
            let (mut j, mut num_s, mut unit_s) = (i + 1, String::new(), String::new());
            if words.get(j).map(|s| s == "than").unwrap_or(false) {
                j += 1;
            }
            if let Some(tok) = words.get(j) {
                let split = tok.find(|c: char| c.is_alphabetic()).unwrap_or(tok.len());
                num_s = tok[..split].to_string();
                unit_s = tok[split..].to_string();
                if unit_s.is_empty() {
                    unit_s = words.get(j + 1).cloned().unwrap_or_default();
                    if parse_size(&num_s, &unit_s).is_some() {
                        used[j + 1] = true;
                    }
                }
            }
            if let Some(sz) = parse_size(&num_s, &unit_s) {
                let bigger = matches!(w, "bigger" | "larger" | "over" | "above" | ">");
                if bigger {
                    q.min_size = Some(sz);
                } else {
                    q.max_size = Some(sz);
                }
                q.explain.push(format!(
                    "{} {}{}",
                    if bigger { ">" } else { "<" },
                    num_s,
                    unit_s
                ));
                for k in i..=j {
                    used[k] = true;
                }
                i = j + 1;
                continue;
            }
        }
        match w {
            "today" => {
                q.after = Some(t - DAY);
                q.explain.push("today".into());
                used[i] = true;
            }
            "yesterday" => {
                q.after = Some(t - 2 * DAY);
                q.before = Some(t - DAY / 2);
                q.explain.push("yesterday".into());
                used[i] = true;
            }
            "recent" | "recently" | "new" | "latest" => {
                q.after = Some(t - 14 * DAY);
                q.explain.push("recent".into());
                used[i] = true;
            }
            "old" | "ancient" | "forgotten" => {
                q.before = Some(t - 365 * DAY);
                q.explain.push("over a year old".into());
                used[i] = true;
            }
            "big" | "large" | "huge" | "heavy" => {
                q.min_size = Some(if w == "huge" { 1 << 30 } else { 100 << 20 });
                q.explain.push(if w == "huge" {
                    "> 1 GB".into()
                } else {
                    "> 100 MB".into()
                });
                used[i] = true;
            }
            "small" | "tiny" => {
                q.max_size = Some(1 << 20);
                q.explain.push("< 1 MB".into());
                used[i] = true;
            }
            "folder" | "folders" | "directory" | "directories" | "dir" => {
                q.dirs_only = true;
                q.explain.push("folders".into());
                used[i] = true;
            }
            "last" | "this" if matches!(next, "week" | "month" | "year") => {
                let d = unit_days(next).unwrap_or(7);
                q.after = Some(t - if w == "last" { 2 * d } else { d } * DAY);
                if w == "last" {
                    q.before = Some(t - d * DAY / 2);
                }
                q.explain.push(format!("{w} {next}"));
                used[i] = true;
                used[i + 1] = true;
                i += 2;
                continue;
            }
            "in" | "on" | "from" | "inside" | "under" if !next.is_empty() => {
                if let Some(mi) = MONTHS
                    .iter()
                    .position(|m| *m == next || (next.len() >= 3 && m.starts_with(next)))
                {
                    // "in march": the most recent March.
                    let (y, m, _, _) = crate::timeutil::local_now();
                    let month = mi as i64 + 1;
                    let year = if month > m as i64 {
                        y as i64 - 1
                    } else {
                        y as i64
                    };
                    let start = days_from_civil(year, month, 1) * DAY;
                    let end = days_from_civil(
                        if month == 12 { year + 1 } else { year },
                        if month == 12 { 1 } else { month + 1 },
                        1,
                    ) * DAY;
                    q.after = Some(start);
                    q.before = Some(end);
                    q.explain.push(format!("in {}", MONTHS[mi]));
                    used[i] = true;
                    used[i + 1] = true;
                    i += 2;
                    continue;
                }
                if let Ok(year) = next.parse::<i64>() {
                    if (1990..=2100).contains(&year) {
                        q.after = Some(days_from_civil(year, 1, 1) * DAY);
                        q.before = Some(days_from_civil(year + 1, 1, 1) * DAY);
                        q.explain.push(format!("in {year}"));
                        used[i] = true;
                        used[i + 1] = true;
                        i += 2;
                        continue;
                    }
                }
                let loc = next.trim_end_matches(':').trim_end_matches('\\');
                let known = [
                    "downloads",
                    "desktop",
                    "documents",
                    "pictures",
                    "videos",
                    "music",
                    "appdata",
                    "dev",
                    "onedrive",
                ];
                if known.contains(&loc)
                    || (loc.len() == 1 && next.ends_with(':'))
                    || next.contains('\\')
                {
                    let l = if loc.len() == 1 {
                        format!("{loc}:")
                    } else {
                        loc.to_string()
                    };
                    q.explain.push(format!("in {l}"));
                    q.location.push(l);
                    used[i] = true;
                    used[i + 1] = true;
                    i += 2;
                    continue;
                }
            }
            _ => {}
        }
        if !used[i] {
            let bare = w.trim_start_matches('.');
            if let Some((label, exts)) = type_exts(bare) {
                q.exts.extend(exts.iter().map(|e| e.to_string()));
                q.files_only = true;
                q.explain.push(label.to_string());
                used[i] = true;
            } else if w.starts_with('.') && w.len() > 1 && w.len() <= 6 {
                q.exts.push(bare.to_string());
                q.explain.push(format!(".{bare}"));
                used[i] = true;
            } else if w.starts_with("*.") {
                q.exts.push(w[2..].to_string());
                q.explain.push(w.to_string());
                used[i] = true;
            }
        }
        i += 1;
    }

    for (k, w) in words.iter().enumerate() {
        if !used[k] && !STOP.contains(&w.as_str()) {
            q.terms
                .push(w.trim_matches(|c: char| c == '.' || c == '\'').to_string());
        }
    }
    q.terms.retain(|t| !t.is_empty());
    q.exts.sort();
    q.exts.dedup();
    if !q.terms.is_empty() {
        q.explain.insert(0, format!("“{}”", q.terms.join(" ")));
    }
    q
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn contains_ci(hay: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let h = hay.as_bytes();
    let n = needle.as_bytes();
    if n.len() > h.len() {
        return None;
    }
    'outer: for i in 0..=h.len() - n.len() {
        for j in 0..n.len() {
            if h[i + j].to_ascii_lowercase() != n[j] {
                continue 'outer;
            }
        }
        return Some(i);
    }
    None
}

impl Query {
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
            && self.exts.is_empty()
            && self.min_size.is_none()
            && self.max_size.is_none()
            && self.after.is_none()
            && self.before.is_none()
            && self.location.is_empty()
    }

    /// Name-level filter + base score. None = no match.
    pub fn prefilter(&self, name: &str, is_dir: bool, size: u64, mtime: i64) -> Option<f32> {
        if self.is_empty() || (self.dirs_only && !is_dir) || (self.files_only && is_dir) {
            return None;
        }
        if let Some(m) = self.min_size {
            if size < m {
                return None;
            }
        }
        if let Some(m) = self.max_size {
            if size > m || is_dir {
                return None;
            }
        }
        if let Some(a) = self.after {
            if mtime < a {
                return None;
            }
        }
        if let Some(b) = self.before {
            if mtime > b {
                return None;
            }
        }
        if !self.exts.is_empty() {
            let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
            if !self.exts.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                return None;
            }
        }
        let mut score = 1.0f32;
        for t in &self.terms {
            let pos = contains_ci(name, t)?;
            score += 10.0;
            if pos == 0 {
                score += 12.0;
            } else if !name.as_bytes()[pos - 1].is_ascii_alphanumeric() {
                score += 6.0;
            }
            if name.len() == t.len() || name.len() <= t.len() + 5 {
                score += 8.0;
            }
        }
        score -= (name.len() as f32 / 40.0).min(4.0);
        // A gentle nudge towards recent things.
        let age_days = (now() - mtime).max(0) as f32 / 86_400.0;
        score += 3.0 / (1.0 + age_days / 30.0);
        if is_dir {
            score += 1.0;
        }
        Some(score)
    }

    /// Path-level filter (locations) and noise penalties.
    pub fn path_score(&self, path: &str, base: f32) -> Option<f32> {
        let low = path.to_lowercase();
        for l in &self.location {
            if l.ends_with(':') {
                if !low.starts_with(l.as_str()) {
                    return None;
                }
            } else if !low.contains(&format!("\\{l}")) {
                return None;
            }
        }
        let mut s = base;
        for (noise, pen) in [
            ("\\appdata\\", 6.0),
            ("\\windows\\", 8.0),
            ("\\program files", 4.0),
            ("\\node_modules\\", 10.0),
            ("\\.git\\", 10.0),
            ("\\$recycle.bin", 20.0),
            ("\\winsxs\\", 20.0),
            ("\\site-packages\\", 8.0),
        ] {
            if low.contains(noise) {
                s -= pen;
            }
        }
        for good in [
            "\\downloads\\",
            "\\desktop\\",
            "\\documents\\",
            "\\pictures\\",
            "\\dev\\",
        ] {
            if low.contains(good) {
                s += 2.0;
            }
        }
        Some(s)
    }

    pub fn score(
        &self,
        name: &str,
        path: &str,
        is_dir: bool,
        size: u64,
        mtime: i64,
    ) -> Option<f32> {
        let base = self.prefilter(name, is_dir, size, mtime)?;
        self.path_score(path, base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn understands_loose_questions() {
        let q = parse("find me a pdf with invoice in its name downloaded roughly 5 months ago");
        assert_eq!(q.terms, vec!["invoice"]);
        assert!(q.exts.contains(&"pdf".to_string()));
        let center = now() - 150 * DAY;
        assert!(q.after.unwrap() < center && q.before.unwrap() > center);

        let q = parse("big videos on d:");
        assert!(q.min_size.is_some());
        assert_eq!(q.location, vec!["d:"]);
        assert!(q.exts.contains(&"mp4".to_string()));

        let q = parse("\"tax 2024\" bigger than 2 mb");
        assert_eq!(q.terms, vec!["tax 2024"]);
        assert_eq!(q.min_size, Some(2 * 1_048_576));

        let q = parse("budget folder in documents");
        assert!(q.dirs_only);
        assert_eq!(q.location, vec!["documents"]);
        assert_eq!(q.terms, vec!["budget"]);
    }

    #[test]
    fn scoring_prefers_prefix() {
        let q = parse("report");
        let a = q.prefilter("report.pdf", false, 10, now()).unwrap();
        let b = q
            .prefilter("old_report_final_v2.pdf", false, 10, now())
            .unwrap();
        assert!(a > b);
        assert!(q.prefilter("other.pdf", false, 10, now()).is_none());
    }
}
