//! Watches running processes over time to spot resource hogs and processes
//! left behind by tools you already closed (dev servers, build daemons...).

use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// Background helpers that are often forgotten after closing an IDE or terminal.
const DAEMONS: &[&str] = &[
    "node.exe",
    "python.exe",
    "pythonw.exe",
    "java.exe",
    "javaw.exe",
    "adb.exe",
    "esbuild.exe",
    "vbcscompiler.exe",
    "msbuild.exe",
    "dotnet.exe",
    "gradle.exe",
    "deno.exe",
    "bun.exe",
    "ruby.exe",
    "php.exe",
    "postgres.exe",
    "mongod.exe",
    "redis-server.exe",
    "ngrok.exe",
    "rust-analyzer.exe",
    "cargo.exe",
    "qemu-system-x86_64.exe",
    "emulator.exe",
    "uvicorn.exe",
    "conhost.exe",
];

/// Never suggest touching these.
const SYSTEM: &[&str] = &[
    "system",
    "registry",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "services.exe",
    "lsass.exe",
    "svchost.exe",
    "winlogon.exe",
    "dwm.exe",
    "explorer.exe",
    "memory compression",
    "fontdrvhost.exe",
    "audiodg.exe",
    "msmpeng.exe",
    "searchindexer.exe",
    "spoolsv.exe",
    "sihost.exe",
    "ctfmon.exe",
    "runtimebroker.exe",
    "taskhostw.exe",
    "startmenuexperiencehost.exe",
    "shellexperiencehost.exe",
    "textinputhost.exe",
    "securityhealthservice.exe",
    "nissrv.exe",
    "wudfhost.exe",
    "lsaiso.exe",
    "dusty.exe",
    "system idle process",
    "secure system",
    "vmmem",
    "vmmemwsl",
    "searchhost.exe",
    "widgets.exe",
    "msedgewebview2.exe",
    "conhost.exe",
];

#[derive(Debug, Clone, Copy, Default)]
struct Sample {
    cpu: f32,
    mem: u64,
    disk: u64,
    idle: bool,
}

pub struct Monitor {
    sys: System,
    history: HashMap<String, VecDeque<Sample>>,
    cores: f32,
    last: std::time::Instant,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcGroup {
    pub name: String,
    pub pids: Vec<u32>,
    pub count: usize,
    pub memory: u64,
    /// Percent of the whole machine (0-100).
    pub cpu: f32,
    /// Bytes per second read + written.
    pub disk_rate: u64,
    pub exe: Option<String>,
    pub has_window: bool,
    /// Average CPU (whole machine %) while you were away.
    pub idle_cpu: f32,
    pub avg_memory: u64,
    /// "leftover" | "busy_idle" | "memory_hog" | "system"
    pub flags: Vec<String>,
    /// Pids considered leftovers (parent gone, no window, running for hours).
    pub leftover_pids: Vec<u32>,
    pub oldest_secs: u64,
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind());
        Monitor {
            sys,
            history: HashMap::new(),
            cores: std::thread::available_parallelism()
                .map(|n| n.get() as f32)
                .unwrap_or(4.0),
            last: std::time::Instant::now(),
        }
    }

    pub fn sample(&mut self, user_idle: bool) {
        self.sys
            .refresh_processes_specifics(ProcessesToUpdate::All, true, kind());
        let secs = self.last.elapsed().as_secs_f32().max(0.5);
        self.last = std::time::Instant::now();
        let mut agg: HashMap<String, Sample> = HashMap::new();
        for p in self.sys.processes().values() {
            let name = p.name().to_string_lossy().to_lowercase();
            let s = agg.entry(name).or_default();
            s.cpu += p.cpu_usage() / self.cores;
            s.mem += p.memory();
            let du = p.disk_usage();
            s.disk += ((du.read_bytes + du.written_bytes) as f32 / secs) as u64;
            s.idle = user_idle;
        }
        for (name, s) in agg {
            let h = self.history.entry(name).or_default();
            h.push_back(s);
            while h.len() > 40 {
                h.pop_front();
            }
        }
        let alive: HashSet<String> = self
            .sys
            .processes()
            .values()
            .map(|p| p.name().to_string_lossy().to_lowercase())
            .collect();
        self.history.retain(|k, _| alive.contains(k));
    }

    pub fn groups(&self) -> Vec<ProcGroup> {
        let windows = crate::winsys::pids_with_windows();
        let procs = self.sys.processes();
        let mut groups: HashMap<String, ProcGroup> = HashMap::new();
        for (pid, p) in procs {
            let name = p.name().to_string_lossy().to_lowercase();
            let display = p.name().to_string_lossy().into_owned();
            let g = groups.entry(name.clone()).or_insert_with(|| ProcGroup {
                name: display,
                pids: vec![],
                count: 0,
                memory: 0,
                cpu: 0.0,
                disk_rate: 0,
                exe: p.exe().map(|e| e.to_string_lossy().into_owned()),
                has_window: false,
                idle_cpu: 0.0,
                avg_memory: 0,
                flags: vec![],
                leftover_pids: vec![],
                oldest_secs: 0,
            });
            g.pids.push(pid.as_u32());
            g.count += 1;
            g.memory += p.memory();
            g.cpu += p.cpu_usage() / self.cores;
            g.has_window |= windows.contains(&pid.as_u32());
            g.oldest_secs = g.oldest_secs.max(p.run_time());
            let orphan = p
                .parent()
                .map(|pp: Pid| procs.get(&pp).is_none())
                .unwrap_or(true);
            if DAEMONS.contains(&name.as_str())
                && orphan
                && p.run_time() > 3600
                && !windows.contains(&pid.as_u32())
                && name != "conhost.exe"
            {
                g.leftover_pids.push(pid.as_u32());
            }
        }
        let mut out: Vec<ProcGroup> = groups
            .into_iter()
            .map(|(key, mut g)| {
                if let Some(h) = self.history.get(&key) {
                    let idle: Vec<&Sample> = h.iter().filter(|s| s.idle).collect();
                    if !idle.is_empty() {
                        g.idle_cpu = idle.iter().map(|s| s.cpu).sum::<f32>() / idle.len() as f32;
                    }
                    g.avg_memory = h.iter().map(|s| s.mem).sum::<u64>() / h.len() as u64;
                    g.disk_rate = h.back().map(|s| s.disk).unwrap_or(0);
                    if idle.len() >= 6 && g.idle_cpu > 8.0 {
                        g.flags.push("busy_idle".into());
                    }
                }
                if SYSTEM.contains(&key.as_str()) {
                    g.flags.push("system".into());
                    g.leftover_pids.clear();
                } else if g.avg_memory > 1_500_000_000 && !g.has_window {
                    g.flags.push("memory_hog".into());
                }
                if !g.leftover_pids.is_empty() {
                    g.flags.push("leftover".into());
                }
                g
            })
            .collect();
        out.sort_by_key(|g| std::cmp::Reverse(g.memory));
        out
    }

    pub fn kill(&self, pids: &[u32]) -> usize {
        pids.iter()
            .filter_map(|p| self.sys.process(Pid::from_u32(*p)))
            .filter(|p| {
                let n = p.name().to_string_lossy().to_lowercase();
                !SYSTEM.contains(&n.as_str())
            })
            .filter(|p| p.kill())
            .count()
    }

    pub fn total_memory() -> (u64, u64) {
        let mut s = System::new();
        s.refresh_memory();
        (s.total_memory(), s.used_memory())
    }
}

fn kind() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing()
        .with_cpu()
        .with_memory()
        .with_disk_usage()
        .with_exe(sysinfo::UpdateKind::OnlyIfNotSet)
}
