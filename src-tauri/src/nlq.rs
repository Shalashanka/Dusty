//! Turns loose questions (typed or spoken, English or Italian) into search
//! filters: "find me a pdf with invoice in its name downloaded roughly five
//! months ago" / "trova le foto delle vacanze di due anni fa".
//!
//! Matching is forgiving by design: terms can hit a file's name or any folder
//! above it, results are ranked by how many terms they cover (so adding words
//! never empties the list), plurals/accents/synonyms are folded, and a
//! typo-tolerant pass kicks in when nothing matches.

use crate::timeutil::{now, DAY};
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    /// Words to look for, as typed (for display).
    pub terms: Vec<String>,
    /// Per term: folded stem + synonyms. Any alternative counts as a hit.
    #[serde(skip)]
    pub alts: Vec<Vec<String>>,
    /// Per term: the folded stem of exactly what was typed (ranks above synonyms).
    #[serde(skip)]
    pub primary: Vec<String>,
    pub exts: Vec<String>,
    pub dirs_only: bool,
    pub files_only: bool,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub after: Option<i64>,
    pub before: Option<i64>,
    /// Folder names that must appear in the path ("downloads", "d:").
    pub location: Vec<String>,
    /// Human-readable chips explaining how the query was understood.
    pub explain: Vec<String>,
}

const STOP: &[&str] = &[
    // English
    "find", "me", "a", "an", "the", "file", "files", "with", "something", "like", "named", "called", "that", "which",
    "is", "was", "has", "its", "it's", "it", "name", "roughly", "about", "around", "approximately", "i", "my", "some",
    "of", "and", "or", "for", "show", "where", "search", "look", "get", "open", "please", "downloaded",
    "saved", "created", "modified", "made", "got", "had", "in", "on", "from", "inside", "under", "ago", "where's",
    "whats", "what's", "thing", "things", "stuff", "contains", "containing", "can", "you", "to", "all", "any", "do",
    "have", "did", "there", "this", "those", "these", "at", "by", "one", "ones",
    // Italian
    "trova", "trovami", "cerca", "cercami", "mostra", "mostrami", "fammi", "vedere", "dove", "il", "lo", "la", "i",
    "gli", "le", "un", "una", "uno", "di", "del", "della", "dello", "dei", "delle", "degli", "da", "dal", "dalla",
    "dai", "che", "ho", "hai", "con", "nel", "nella", "nei", "nelle", "negli", "sul", "sulla", "sui", "sulle", "per",
    "mio", "mia", "miei", "mie", "circa", "scaricato", "scaricata", "scaricati", "scaricate", "salvato", "salvati",
    "fa", "quel", "quello", "quella", "quei", "quelle", "e", "o", "ed", "tipo", "nome", "chiamato", "chiamata",
    "tutti", "tutte", "qualche", "ciao", "per favore",
];

/// Type words -> extensions (EN + IT).
fn type_exts(word: &str) -> Option<(&'static str, &'static [&'static str])> {
    Some(match word {
        "pdf" | "pdfs" => ("PDFs", &["pdf"]),
        "doc" | "docs" | "word" | "document" | "documents" | "documento" | "documenti" => {
            ("documents", &["doc", "docx", "odt", "rtf", "txt", "md", "pdf"])
        }
        "excel" | "spreadsheet" | "spreadsheets" | "xlsx" | "sheet" | "sheets" | "foglio" | "fogli" => {
            ("spreadsheets", &["xls", "xlsx", "csv", "ods"])
        }
        "slides" | "presentation" | "presentations" | "powerpoint" | "pptx" | "presentazione" | "presentazioni" => {
            ("presentations", &["ppt", "pptx", "odp", "key"])
        }
        "image" | "images" | "photo" | "photos" | "picture" | "pictures" | "pic" | "pics" | "foto" | "immagine"
        | "immagini" | "fotografie" | "fotografia" => (
            "images",
            &["png", "jpg", "jpeg", "gif", "webp", "heic", "bmp", "tif", "tiff", "raw", "cr2", "cr3", "nef", "arw", "dng"],
        ),
        "screenshot" | "screenshots" | "schermata" | "schermate" => ("screenshots", &["png", "jpg", "jpeg"]),
        "video" | "videos" | "movie" | "movies" | "clip" | "clips" | "recording" | "recordings" | "filmato" | "filmati"
        | "film" | "registrazione" | "registrazioni" => ("videos", &["mp4", "mkv", "mov", "avi", "webm", "wmv", "m4v"]),
        "song" | "songs" | "music" | "audio" | "track" | "tracks" | "podcast" | "musica" | "canzone" | "canzoni"
        | "brano" | "brani" => ("audio", &["mp3", "wav", "flac", "ogg", "m4a", "aac"]),
        "zip" | "zips" | "archive" | "archives" | "rar" | "7z" | "archivio" | "archivi" | "compresso" | "compressi" => {
            ("archives", &["zip", "7z", "rar", "tar", "gz", "tgz", "xz"])
        }
        "installer" | "installers" | "setup" | "exe" | "program" | "programs" | "msi" | "programma" | "programmi"
        | "eseguibile" | "eseguibili" => ("installers", &["exe", "msi", "msix", "appx"]),
        "script" | "scripts" | "bat" | "powershell" => ("scripts", &["bat", "cmd", "ps1", "sh", "py", "ahk", "vbs"]),
        "font" | "fonts" | "carattere" | "caratteri" => ("fonts", &["ttf", "otf", "woff", "woff2"]),
        "iso" => ("disk images", &["iso", "img", "vhd", "vhdx"]),
        "ebook" | "ebooks" | "epub" | "libro" | "libri" => ("ebooks", &["epub", "mobi", "azw3", "pdf"]),
        "psd" | "design" | "designs" | "figma" | "photoshop" => {
            ("design files", &["psd", "ai", "fig", "sketch", "xd", "blend", "afdesign"])
        }
        _ => return None,
    })
}

/// Groups of words that mean the same thing when looking for files.
const SYNONYMS: &[&[&str]] = &[
    &["invoice", "fattura", "bill", "receipt", "ricevuta", "fatt"],
    &["contract", "contratto", "agreement", "accordo"],
    &["resume", "cv", "curriculum"],
    &["tax", "taxes", "tasse", "f24", "730", "irpef"],
    &["bank", "banca", "statement", "estratto"],
    &["holiday", "holidays", "vacation", "vacanza", "vacanze", "trip", "viaggio", "travel"],
    &["wedding", "matrimonio", "sposi"],
    &["birthday", "compleanno", "bday"],
    &["passport", "passaporto"],
    &["id card", "carta identita", "documento identita"],
    &["screenshot", "screen", "schermata", "cattura", "snip"],
    &["backup", "bak", "bkp", "copia"],
    &["meeting", "riunione", "call"],
    &["notes", "note", "appunti"],
    &["report", "relazione", "rapporto"],
    &["project", "progetto"],
    &["logo", "logos", "loghi"],
    &["insurance", "assicurazione", "polizza"],
    &["salary", "payslip", "busta paga", "cedolino", "stipendio"],
];

fn number_word(w: &str) -> Option<f64> {
    Some(match w {
        "a" | "an" | "one" | "uno" | "una" | "un" => 1.0,
        "two" | "due" | "couple" => 2.0,
        "three" | "tre" | "few" => 3.0,
        "four" | "quattro" => 4.0,
        "five" | "cinque" => 5.0,
        "six" | "sei" => 6.0,
        "seven" | "sette" => 7.0,
        "eight" | "otto" => 8.0,
        "nine" | "nove" => 9.0,
        "ten" | "dieci" => 10.0,
        "eleven" | "undici" => 11.0,
        "twelve" | "dodici" => 12.0,
        "fifteen" | "quindici" => 15.0,
        "twenty" | "venti" => 20.0,
        "half" | "mezzo" => 0.5,
        _ => return w.parse::<f64>().ok(),
    })
}

fn unit_days(w: &str) -> Option<i64> {
    Some(match w {
        "day" | "days" | "giorno" | "giorni" => 1,
        "week" | "weeks" | "settimana" | "settimane" => 7,
        "month" | "months" | "mese" | "mesi" => 30,
        "year" | "years" | "anno" | "anni" => 365,
        _ => return None,
    })
}

fn parse_size(num: &str, unit: &str) -> Option<u64> {
    let n: f64 = num.replace(',', ".").parse().ok()?;
    let m = match unit.trim_end_matches('s') {
        "kb" | "k" => 1024.0,
        "mb" | "m" | "meg" | "mega" => 1_048_576.0,
        "gb" | "g" | "gig" | "giga" => 1_073_741_824.0,
        "tb" | "tera" => 1_099_511_627_776.0,
        _ => return None,
    };
    Some((n * m) as u64)
}

const MONTHS: &[&[&str]] = &[
    &["january", "gennaio"],
    &["february", "febbraio"],
    &["march", "marzo"],
    &["april", "aprile"],
    &["may", "maggio"],
    &["june", "giugno"],
    &["july", "luglio"],
    &["august", "agosto"],
    &["september", "settembre"],
    &["october", "ottobre"],
    &["november", "novembre"],
    &["december", "dicembre"],
];

/// Lowercase, strip accents (è -> e), keep letters/digits.
pub fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' | 'À' | 'Á' | 'Â' | 'Ä' => 'a',
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' | 'Ò' | 'Ó' | 'Ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// Cheap stem so plurals match: invoices -> invoice, fatture -> fattur, photos -> photo.
fn stem(w: &str) -> String {
    let w = fold(w);
    if w.len() >= 6 && w.ends_with("ies") {
        return format!("{}y", &w[..w.len() - 3]);
    }
    if w.len() >= 5 && (w.ends_with("es") && !w.ends_with("ses")) {
        return w[..w.len() - 1].to_string();
    }
    if w.len() >= 4 && w.ends_with('s') && !w.ends_with("ss") {
        return w[..w.len() - 1].to_string();
    }
    // Italian: fattura/fatture/fatturi -> fattur
    if w.len() >= 5 && w.ends_with(['a', 'e', 'i', 'o']) && w.chars().all(|c| c.is_ascii_alphabetic()) {
        return w[..w.len() - 1].to_string();
    }
    w
}

fn alternatives(term: &str) -> Vec<String> {
    let f = fold(term);
    let mut out = vec![stem(&f)];
    for group in SYNONYMS {
        if group.iter().any(|g| fold(g) == f || stem(g) == stem(&f)) {
            out.extend(group.iter().map(|g| stem(g)));
        }
    }
    out.sort();
    out.dedup();
    out.retain(|a| !a.is_empty());
    out
}

pub fn parse(input: &str) -> Query {
    let mut q = Query::default();
    let t = now();
    let mut lower = input.to_lowercase().replace(['’', '‘'], "'");
    // "Hey Dusty, find ..." -> "find ..."
    for prefix in ["hey dusty", "ok dusty", "okay dusty", "ciao dusty", "dusty,", "hey,", "ok,"] {
        if let Some(rest) = lower.trim_start().strip_prefix(prefix) {
            lower = rest.trim_start_matches([',', ' ']).to_string();
        }
    }

    // Quoted phrases are taken literally.
    let mut rest = String::new();
    let mut in_quote = false;
    let mut phrase = String::new();
    for ch in lower.chars() {
        if ch == '"' || ch == '“' || ch == '”' {
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

    // Split "dell'anno" -> "dell anno", keep "d:" and "c:\dev".
    let words: Vec<String> = rest
        .split(|c: char| c.is_whitespace() || c == ',' || c == '?' || c == '!' || c == ';')
        .flat_map(|w| {
            if w.contains('\'') && !w.contains('\\') {
                w.split('\'').map(|s| s.to_string()).collect::<Vec<_>>()
            } else {
                vec![w.to_string()]
            }
        })
        .map(|w| w.trim_end_matches('.').to_string())
        .filter(|w| !w.is_empty())
        .collect();
    let fuzzy = ["roughly", "about", "around", "approx", "circa", "più o meno"].iter().any(|k| lower.contains(k));
    let mut used = vec![false; words.len()];
    let w_at = |i: usize| words.get(i).map(|s| s.as_str()).unwrap_or("");
    let mut i = 0;
    while i < words.len() {
        let w = w_at(i);
        let next = w_at(i + 1);
        let next2 = w_at(i + 2);

        // "5 months ago", "five months ago", "a week ago", "5 mesi fa"
        if let (Some(n), Some(days)) = (number_word(w), unit_days(next)) {
            let prev = if i > 0 { w_at(i - 1) } else { "" };
            let ago = next2 == "ago" || next2 == "fa";
            if ago || matches!(prev, "last" | "past" | "ultimi" | "ultime" | "ultimo" | "ultima" | "negli" | "nelle") {
                let span = n * days as f64 * DAY as f64;
                if ago {
                    let center = t - span as i64;
                    let spread = (span * if fuzzy { 0.3 } else { 0.22 }).max(DAY as f64 * 2.0) as i64;
                    q.after = Some(center - spread);
                    q.before = Some(center + spread);
                    q.explain.push(format!("~{} {next} ago", n));
                    used[i + 2] = true;
                } else {
                    q.after = Some(t - span as i64);
                    q.explain.push(format!("last {} {next}", n));
                }
                used[i] = true;
                used[i + 1] = true;
                i += if ago { 3 } else { 2 };
                continue;
            }
        }
        // "bigger than 2 gb", "over 500mb", "più di 1 gb", "under 10 mb"
        let bigger_words = ["bigger", "larger", "over", "above", ">", "più", "piu", "oltre", "sopra", "maggiore"];
        let smaller_words = ["smaller", "under", "below", "<", "meno", "sotto", "minore"];
        if bigger_words.contains(&w) || smaller_words.contains(&w) {
            let mut j = i + 1;
            if matches!(w_at(j), "than" | "di" | "dei" | "a") {
                j += 1;
            }
            let tok = w_at(j).to_string();
            let split = tok.find(|c: char| c.is_alphabetic()).unwrap_or(tok.len());
            let num_s = tok[..split].to_string();
            let mut unit_s = tok[split..].to_string();
            let mut consumed = j;
            if unit_s.is_empty() {
                unit_s = w_at(j + 1).to_string();
                consumed = j + 1;
            }
            if let Some(sz) = parse_size(&num_s, &unit_s) {
                if bigger_words.contains(&w) {
                    q.min_size = Some(sz);
                    q.explain.push(format!("> {num_s} {unit_s}"));
                } else {
                    q.max_size = Some(sz);
                    q.explain.push(format!("< {num_s} {unit_s}"));
                }
                used[i..=consumed.min(words.len() - 1)].iter_mut().for_each(|u| *u = true);
                i = consumed + 1;
                continue;
            }
        }
        // "d drive", "drive d", "disco d", "the c drive"
        if (next == "drive" || next == "disk") && w.len() == 1 && w.chars().all(|c| c.is_ascii_alphabetic()) {
            q.location.push(format!("{w}:"));
            q.explain.push(format!("on {}:", w.to_uppercase()));
            used[i] = true;
            used[i + 1] = true;
            i += 2;
            continue;
        }
        if matches!(w, "drive" | "disk" | "disco" | "unità" | "unita") && next.trim_end_matches(':').len() == 1 {
            let l = next.trim_end_matches(':');
            q.location.push(format!("{l}:"));
            q.explain.push(format!("on {}:", l.to_uppercase()));
            used[i] = true;
            used[i + 1] = true;
            i += 2;
            continue;
        }
        match w {
            "today" | "oggi" => {
                q.after = Some(t - DAY);
                q.explain.push("today".into());
                used[i] = true;
            }
            "yesterday" | "ieri" => {
                q.after = Some(t - 2 * DAY);
                q.before = Some(t - DAY / 2);
                q.explain.push("yesterday".into());
                used[i] = true;
            }
            "recent" | "recently" | "new" | "latest" | "recenti" | "recente" | "nuovi" | "ultimi" => {
                q.after = Some(t - 14 * DAY);
                q.explain.push("recent".into());
                used[i] = true;
            }
            "old" | "ancient" | "forgotten" | "vecchi" | "vecchie" | "vecchio" | "dimenticati" => {
                q.before = Some(t - 365 * DAY);
                q.explain.push("over a year old".into());
                used[i] = true;
            }
            "big" | "large" | "huge" | "heavy" | "grandi" | "grossi" | "grosse" | "pesanti" | "enormi" => {
                let huge = matches!(w, "huge" | "enormi");
                q.min_size = Some(if huge { 1 << 30 } else { 100 << 20 });
                q.explain.push(if huge { "> 1 GB".into() } else { "> 100 MB".into() });
                used[i] = true;
            }
            "small" | "tiny" | "piccoli" | "piccole" => {
                q.max_size = Some(1 << 20);
                q.explain.push("< 1 MB".into());
                used[i] = true;
            }
            "folder" | "folders" | "directory" | "directories" | "dir" | "cartella" | "cartelle" => {
                q.dirs_only = true;
                q.explain.push("folders".into());
                used[i] = true;
            }
            // "last week", "this month", "settimana scorsa", "l'anno scorso"
            "last" | "this" | "questa" | "questo" if unit_days(next).is_some() => {
                let d = unit_days(next).unwrap();
                let last = w == "last";
                q.after = Some(t - if last { 2 * d } else { d } * DAY);
                if last {
                    q.before = Some(t - d * DAY / 2);
                }
                q.explain.push(format!("{w} {next}"));
                used[i] = true;
                used[i + 1] = true;
                i += 2;
                continue;
            }
            _ if unit_days(w).is_some() && matches!(next, "scorso" | "scorsa" | "passato" | "passata") => {
                let d = unit_days(w).unwrap();
                q.after = Some(t - 2 * d * DAY);
                q.before = Some(t - d * DAY / 2);
                q.explain.push(format!("last {w}"));
                used[i] = true;
                used[i + 1] = true;
                i += 2;
                continue;
            }
            _ => {}
        }
        // Months and years: "in march", "a marzo", "in 2023", "nel 2023"
        if !used[i] {
            if let Some(mi) = MONTHS.iter().position(|names| names.iter().any(|n| *n == w || (w.len() >= 3 && n.starts_with(w) && w != "mar" && w != "may"))) {
                let (y, m, _, _) = crate::timeutil::local_now();
                let month = mi as i64 + 1;
                let year = if let Ok(yr) = next.parse::<i64>() {
                    used[i + 1] = true;
                    yr
                } else if month > m as i64 {
                    y as i64 - 1
                } else {
                    y as i64
                };
                q.after = Some(days_from_civil(year, month, 1) * DAY);
                q.before = Some(days_from_civil(if month == 12 { year + 1 } else { year }, if month == 12 { 1 } else { month + 1 }, 1) * DAY);
                q.explain.push(format!("{} {year}", MONTHS[mi][0]));
                used[i] = true;
                i += 1;
                continue;
            }
            if let Ok(year) = w.parse::<i64>() {
                let prev = if i > 0 { w_at(i - 1) } else { "" };
                if (1990..=2100).contains(&year) && matches!(prev, "in" | "nel" | "del" | "from" | "dal" | "since") {
                    q.after = Some(days_from_civil(year, 1, 1) * DAY);
                    q.before = Some(days_from_civil(year + 1, 1, 1) * DAY);
                    q.explain.push(format!("in {year}"));
                    used[i] = true;
                    i += 1;
                    continue;
                }
            }
        }
        // Locations: "in downloads", "on desktop", "nei documenti", "on d:"
        if matches!(w, "in" | "on" | "from" | "inside" | "under" | "nei" | "nel" | "nella" | "sul" | "sulla" | "dentro" | "su") && !next.is_empty() {
            let loc = next.trim_end_matches(':').trim_end_matches('\\');
            let mapped = match loc {
                "downloads" | "download" | "scaricati" => Some("downloads"),
                "desktop" | "scrivania" => Some("desktop"),
                "documents" | "documenti" => Some("documents"),
                "pictures" | "immagini" => Some("pictures"),
                "videos" => Some("videos"),
                "music" => Some("music"),
                "appdata" => Some("appdata"),
                "dev" => Some("dev"),
                "onedrive" => Some("onedrive"),
                _ => None,
            };
            if let Some(l) = mapped {
                q.explain.push(format!("in {l}"));
                q.location.push(l.into());
                used[i] = true;
                used[i + 1] = true;
                i += 2;
                continue;
            }
            if (loc.len() == 1 && next.ends_with(':')) || next.contains('\\') {
                let l = if loc.len() == 1 { format!("{loc}:") } else { loc.to_string() };
                q.explain.push(format!("in {l}"));
                q.location.push(l);
                used[i] = true;
                used[i + 1] = true;
                i += 2;
                continue;
            }
        }
        if !used[i] {
            let bare = w.trim_start_matches('.');
            if let Some((label, exts)) = type_exts(bare) {
                q.exts.extend(exts.iter().map(|e| e.to_string()));
                q.files_only = true;
                if !q.explain.contains(&label.to_string()) {
                    q.explain.push(label.to_string());
                }
                // "screenshot" is both a type and a name hint.
                if matches!(bare, "screenshot" | "screenshots" | "schermata" | "schermate") {
                    q.terms.push("screenshot".into());
                }
                used[i] = true;
            } else if w.starts_with('.') && w.len() > 1 && w.len() <= 6 {
                q.exts.push(bare.to_string());
                q.explain.push(format!(".{bare}"));
                used[i] = true;
            } else if let Some(e) = w.strip_prefix("*.") {
                q.exts.push(e.to_string());
                q.explain.push(w.to_string());
                used[i] = true;
            }
        }
        i += 1;
    }

    for (k, w) in words.iter().enumerate() {
        if !used[k] && !STOP.contains(&w.as_str()) {
            let term = w.trim_matches(|c: char| c == '.' || c == '\'' || c == '-').to_string();
            if !term.is_empty() && !q.terms.contains(&term) {
                q.terms.push(term);
            }
        }
    }
    q.exts.sort();
    q.exts.dedup();
    q.alts = q.terms.iter().map(|t| alternatives(t)).collect();
    q.primary = q.terms.iter().map(|t| stem(t)).collect();
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

/// Case/accent-insensitive substring search. `needle` must already be folded.
pub fn find_folded(hay: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if hay.is_ascii() {
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
    } else {
        fold(hay).find(needle)
    }
}

/// Typo variants: drop one letter ("invocie" -> "invoce"/"invcie"...), swap neighbours.
pub fn typo_variants(term: &str) -> Vec<String> {
    let c: Vec<char> = term.chars().collect();
    if c.len() < 5 {
        return vec![];
    }
    let mut v = Vec::new();
    for i in 1..c.len() {
        let mut d = c.clone();
        d.remove(i);
        v.push(d.into_iter().collect());
        if i + 1 < c.len() {
            let mut s = c.clone();
            s.swap(i, i + 1);
            v.push(s.into_iter().collect());
        }
    }
    v.sort();
    v.dedup();
    v
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
            && !self.dirs_only
    }

    /// Size/date/type filters (no name matching).
    pub fn passes(&self, name: &str, is_dir: bool, size: u64, mtime: i64) -> bool {
        if (self.dirs_only && !is_dir) || (self.files_only && is_dir) {
            return false;
        }
        if self.min_size.is_some_and(|m| size < m) || self.max_size.is_some_and(|m| size > m || is_dir) {
            return false;
        }
        if self.after.is_some_and(|a| mtime < a) || self.before.is_some_and(|b| mtime > b) {
            return false;
        }
        if !self.exts.is_empty() {
            let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
            if !self.exts.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                return false;
            }
        }
        true
    }

    /// Bitmask of term groups found in `name`.
    pub fn name_mask(&self, name: &str) -> u32 {
        let mut m = 0;
        for (k, alts) in self.alts.iter().enumerate().take(32) {
            if alts.iter().any(|a| find_folded(name, a).is_some()) {
                m |= 1 << k;
            }
        }
        m
    }

    /// Name-quality bonus for terms found in the name itself.
    pub fn name_score(&self, name: &str, mask: u32) -> f32 {
        let mut s = 0.0;
        for (k, alts) in self.alts.iter().enumerate().take(32) {
            if mask & (1 << k) == 0 {
                continue;
            }
            if let Some(p) = self.primary.get(k) {
                if find_folded(name, p).is_some() {
                    s += 10.0;
                }
            }
            for a in alts {
                if let Some(pos) = find_folded(name, a) {
                    s += 6.0;
                    if pos == 0 {
                        s += 8.0;
                    } else if !name.as_bytes()[pos - 1].is_ascii_alphanumeric() {
                        s += 5.0;
                    }
                    break;
                }
            }
        }
        if !self.alts.is_empty() && mask.count_ones() as usize == self.alts.len() {
            let stem_len: usize = self.terms.iter().map(|t| t.len()).sum();
            if name.len() <= stem_len + 8 {
                s += 6.0;
            }
        }
        s
    }

    pub fn recency(&self, mtime: i64) -> f32 {
        let age_days = (now() - mtime).max(0) as f32 / 86_400.0;
        3.0 / (1.0 + age_days / 30.0)
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
        // Machine-generated files are rarely what people look for.
        let ext = low.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
        if matches!(ext, "class" | "dex" | "pyc" | "o" | "obj" | "pdb" | "dll" | "map" | "cache" | "tmp" | "lock" | "d" | "rlib" | "rmeta") {
            s -= 10.0;
        }
        for (noise, pen) in [
            ("\\appdata\\", 14.0),
            ("\\programdata\\", 14.0),
            ("\\windows\\", 14.0),
            ("\\program files", 8.0),
            ("\\node_modules\\", 20.0),
            ("\\.git\\", 20.0),
            ("\\$recycle.bin", 30.0),
            ("\\winsxs\\", 30.0),
            ("\\site-packages\\", 16.0),
            ("\\.cache\\", 12.0),
            ("\\target\\", 12.0),
            ("\\build\\", 10.0),
            ("\\.gradle\\", 14.0),
            ("\\.transforms\\", 14.0),
            ("\\obj\\", 8.0),
            ("\\__pycache__\\", 16.0),
            ("\\.venv\\", 16.0),
            ("\\.vscode", 12.0),
            ("\\extensions\\", 8.0),
        ] {
            if low.contains(noise) {
                s -= pen;
            }
        }
        for good in ["\\downloads\\", "\\desktop\\", "\\documents\\", "\\pictures\\", "\\videos\\", "\\dev\\"] {
            if low.contains(good) {
                s += 2.0;
            }
        }
        Some(s)
    }

    /// Scores a single path (used for live additions).
    pub fn score(&self, name: &str, path: &str, is_dir: bool, size: u64, mtime: i64) -> Option<f32> {
        if self.is_empty() || !self.passes(name, is_dir, size, mtime) {
            return None;
        }
        let mut mask = 0u32;
        for (k, alts) in self.alts.iter().enumerate().take(32) {
            if alts.iter().any(|a| find_folded(path, a).is_some()) {
                mask |= 1 << k;
            }
        }
        if !self.alts.is_empty() && mask == 0 {
            return None;
        }
        let nm = self.name_mask(name);
        self.path_score(path, mask.count_ones() as f32 * 40.0 + self.name_score(name, nm) + self.recency(mtime))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn understands_loose_questions() {
        let q = parse("find me a pdf with invoice in its name downloaded roughly five months ago");
        assert_eq!(q.terms, vec!["invoice"]);
        assert!(q.exts.contains(&"pdf".to_string()));
        let center = now() - 150 * DAY;
        assert!(q.after.unwrap() < center && q.before.unwrap() > center);

        let q = parse("show me big videos on the D drive");
        assert!(q.min_size.is_some());
        assert_eq!(q.location, vec!["d:"]);
        assert!(q.exts.contains(&"mp4".to_string()));
        assert!(q.terms.is_empty(), "{:?}", q.terms);

        let q = parse("\"tax 2024\" bigger than 2 mb");
        assert_eq!(q.terms, vec!["tax 2024"]);
        assert_eq!(q.min_size, Some(2 * 1_048_576));

        let q = parse("budget folder in documents");
        assert!(q.dirs_only);
        assert_eq!(q.location, vec!["documents"]);
        assert_eq!(q.terms, vec!["budget"]);
    }

    #[test]
    fn understands_italian() {
        let q = parse("trova le foto delle vacanze di due anni fa");
        assert!(q.exts.contains(&"jpg".to_string()));
        assert_eq!(q.terms, vec!["vacanze"]);
        let center = now() - 730 * DAY;
        assert!(q.after.unwrap() < center && q.before.unwrap() > center);
        // Synonyms reach English names too.
        assert!(q.alts[0].contains(&"holiday".to_string()) || q.alts[0].contains(&"trip".to_string()));

        let q = parse("fatture pdf più di 1 mb nei documenti");
        assert_eq!(q.location, vec!["documents"]);
        assert_eq!(q.min_size, Some(1_048_576));
        assert!(q.alts[0].contains(&"invoic".to_string()), "{:?}", q.alts);
    }

    #[test]
    fn forgiving_matching() {
        let q = parse("invoices");
        assert!(q.name_mask("Invoice_ACME.pdf") != 0, "plural folds to singular");
        let q = parse("fattura");
        assert!(q.name_mask("Fatture 2024.pdf") != 0, "Italian plural");
        assert!(q.name_mask("invoice-may.pdf") != 0, "synonym");
        let q = parse("perche");
        assert!(q.name_mask("Perché.txt") != 0, "accents folded");
        assert!(typo_variants("invocie").contains(&"invoice".to_string()));
    }
}
