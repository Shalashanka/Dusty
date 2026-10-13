//! Idle code projects and their rebuildable heavyweights
//! (node_modules, target, .venv, build caches...).

use super::Ctx;
use crate::model::{Action, Finding, Kind};
use crate::timeutil::{human_age, unix_res};
use crate::walk::{self, is_link, name_of};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

const MARKERS: &[&str] = &[
    ".git",
    "package.json",
    "cargo.toml",
    "pyproject.toml",
    "requirements.txt",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "composer.json",
    "gemfile",
    "pubspec.yaml",
    "cmakelists.txt",
];

/// Junk directory names that are always regenerable.
const ALWAYS: &[&str] = &[
    "node_modules",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".parcel-cache",
    ".angular",
    ".expo",
    ".vite",
    ".dart_tool",
    ".gradle",
    ".build",
];

struct Listing {
    names: Vec<String>,
}

impl Listing {
    fn of(dir: &Path) -> Self {
        let names = fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.file_name().to_string_lossy().to_lowercase())
                    .collect()
            })
            .unwrap_or_default();
        Listing { names }
    }
    fn has(&self, n: &str) -> bool {
        self.names.iter().any(|x| x == n)
    }
    fn has_ext(&self, ext: &str) -> bool {
        self.names.iter().any(|x| x.ends_with(ext))
    }
    fn is_project(&self) -> bool {
        MARKERS.iter().any(|m| self.has(m)) || self.has_ext(".sln") || self.has_ext(".csproj")
    }
}

/// Is `name` (a child dir of a folder whose contents are `parent`) rebuildable?
fn is_junk(name: &str, dir: &Path, parent: &Listing) -> bool {
    if ALWAYS.contains(&name) {
        return true;
    }
    // Python virtualenvs and conda environments, whatever they're called.
    if dir.join("pyvenv.cfg").is_file() || dir.join("conda-meta").is_dir() {
        return true;
    }
    match name {
        "target" => parent.has("cargo.toml") || parent.has("pom.xml"),
        "dist" | "build" | "out" => {
            parent.has("package.json") || parent.has("build.gradle") || parent.has("pubspec.yaml")
        }
        "bin" | "obj" => parent.has_ext(".csproj"),
        "pods" => parent.has("podfile"),
        _ => false,
    }
}

fn find_projects(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(md) = e.metadata() else { continue };
        if !md.is_dir() || is_link(&md) {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_lowercase();
        if name.starts_with('.') || name == "node_modules" {
            continue;
        }
        let p = e.path();
        if Listing::of(&p).is_project() {
            out.push(p);
        } else if depth < 2 {
            find_projects(&p, depth + 1, out);
        }
    }
}

/// Walks a project: returns (junk dirs, newest source modification).
fn survey(dir: &Path, depth: u32) -> (Vec<PathBuf>, i64) {
    let listing = Listing::of(dir);
    let Ok(rd) = fs::read_dir(dir) else {
        return (vec![], 0);
    };
    let mut junk = Vec::new();
    let mut newest = 0;
    let mut sub = Vec::new();
    for e in rd.flatten() {
        let Ok(md) = e.metadata() else { continue };
        if is_link(&md) {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_lowercase();
        if md.is_dir() {
            let p = e.path();
            if is_junk(&name, &p, &listing) {
                junk.push(p);
            } else if name != ".git" && depth < 8 {
                sub.push(p);
            }
        } else {
            newest = newest.max(unix_res(md.modified()));
        }
    }
    let (j, n) = sub.par_iter().map(|d| survey(d, depth + 1)).reduce(
        || (vec![], 0),
        |mut a, b| {
            a.0.extend(b.0);
            (a.0, a.1.max(b.1))
        },
    );
    junk.extend(j);
    (junk, newest.max(n))
}

/// Last time git was touched: commits, checkouts, staging, fetches.
fn git_activity(project: &Path) -> i64 {
    let g = project.join(".git");
    ["index", "HEAD", "FETCH_HEAD", "logs/HEAD"]
        .iter()
        .filter_map(|f| fs::metadata(g.join(f)).ok())
        .map(|m| unix_res(m.modified()))
        .max()
        .unwrap_or(0)
}

pub fn scan(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let mut projects = Vec::new();
    for root in &s.dev_roots {
        find_projects(Path::new(root), 0, &mut projects);
    }
    let min_bytes = s.junk_min_mb as u64 * 1_048_576;
    projects
        .par_iter()
        .filter_map(|proj| {
            let (junk, newest_src) = survey(proj, 0);
            if junk.is_empty() {
                return None;
            }
            let active = newest_src.max(git_activity(proj));
            let idle = ctx.age_days(active);
            if idle < s.dev_idle_days as i64 {
                return None;
            }
            let stats: Vec<walk::DirStats> = junk.par_iter().map(|j| walk::dir_stats(j)).collect();
            let bytes: u64 = stats.iter().map(|st| st.bytes).sum();
            if bytes < min_bytes {
                return None;
            }
            let files: u64 = stats.iter().map(|st| st.files).sum();
            let mut kinds: Vec<String> = junk.iter().map(|j| name_of(j)).collect();
            kinds.sort();
            kinds.dedup();
            let list = match kinds.len() {
                1 => kinds[0].clone(),
                2 => format!("{} and {}", kinds[0], kinds[1]),
                _ => format!("{}, {} and {} more", kinds[0], kinds[1], kinds.len() - 2),
            };
            let mut f = Finding::new(
                Kind::DevJunk,
                &proj.to_string_lossy(),
                name_of(proj),
                Action::Delete,
                vec![Action::Delete, Action::Keep, Action::Snooze],
            );
            f.related = junk.iter().map(|j| j.to_string_lossy().into_owned()).collect();
            f.bytes = bytes;
            f.items = files;
            f.is_dir = true;
            f.last_touched = active;
            f.confidence = (0.7 + idle as f32 / 600.0).min(0.95);
            f.reason = format!(
                "No changes in {}. Its {} can be rebuilt any time with an install or build. Your code stays untouched.",
                human_age(idle),
                list
            );
            Some(f)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_junk_only_where_regenerable() {
        let d = std::env::temp_dir().join(format!("dustling-dev-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let app = d.join("app");
        fs::create_dir_all(app.join("node_modules/x")).unwrap();
        fs::create_dir_all(app.join("build")).unwrap();
        fs::create_dir_all(app.join("src/target")).unwrap(); // not cargo: keep
        fs::write(app.join("package.json"), b"{}").unwrap();
        let mut projects = vec![];
        find_projects(&d, 0, &mut projects);
        assert_eq!(projects, vec![app.clone()]);
        let (mut junk, _) = survey(&app, 0);
        junk.sort();
        assert_eq!(junk, vec![app.join("build"), app.join("node_modules")]);
        fs::remove_dir_all(&d).unwrap();
    }
}
