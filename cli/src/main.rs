mod security;
mod transcripts;

use anyhow::Result;
use clap::Parser;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "cc-vibeguard",
    about = "Security audit & risk dashboard for Claude Code sessions",
    version
)]
struct Cli {}

struct Spinner {
    running: Arc<AtomicBool>,
    message: Arc<Mutex<String>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    fn new(initial: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let message = Arc::new(Mutex::new(initial.to_string()));
        let r = running.clone();
        let m = message.clone();
        let handle = thread::spawn(move || {
            let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut i: usize = 0;
            while r.load(Ordering::Relaxed) {
                let msg = m.lock().map(|g| g.clone()).unwrap_or_default();
                let mut err = std::io::stderr().lock();
                let _ = write!(err, "\r{} {}                              ", frames[i % frames.len()], msg);
                let _ = err.flush();
                drop(err);
                i = i.wrapping_add(1);
                thread::sleep(Duration::from_millis(80));
            }
        });
        Spinner { running, message, handle: Some(handle) }
    }

    fn set_message(&self, msg: &str) {
        if let Ok(mut m) = self.message.lock() {
            *m = msg.to_string();
        }
    }

    fn finish(&mut self, final_msg: &str) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        let mut err = std::io::stderr().lock();
        let _ = write!(err, "\r                                                                                          \r");
        let _ = writeln!(err, "✓ {}", final_msg);
        let _ = err.flush();
    }
}

fn main() -> Result<()> {
    let _ = Cli::parse();

    let projects_dir = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".claude").join("projects"))
        .unwrap_or_else(|| PathBuf::from(".claude/projects"));
    let projects_dir_str = projects_dir.to_string_lossy().to_string();

    let mut spinner = Spinner::new("Discovering transcripts...");

    let sessions = transcripts::discover_sessions(&projects_dir_str, true)?;
    spinner.set_message(&format!("Parsing {} transcripts...", sessions.len()));

    let parsed = transcripts::parse_sessions(sessions)?;
    spinner.set_message(&format!("Analyzing {} sessions...", parsed.len()));

    let analysis = security::analyze(parsed, true);
    spinner.set_message("Generating report...");

    let json = serde_json::to_string(&analysis)?;
    let out_dir = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Documents").join("cc-vibeguard"))
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&out_dir)?;

    let html_template = include_str!("../../ui/security-metrics-dev-brand-sections.html");
    let html = html_template.replace(
        "fetch(dataUrl)\n      .then(r => { if (!r.ok) throw new Error(`Failed to load ${dataUrl}: ${r.status}`); return r.json(); })\n      .then(render)",
        &format!("Promise.resolve({}).then(render)", json),
    );
    let html_path = out_dir.join("report.html");
    fs::write(&html_path, &html)?;

    spinner.finish(&format!("Report ready: {}", html_path.display()));

    #[cfg(target_os = "macos")]
    { let _ = std::process::Command::new("open").arg(&html_path).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = std::process::Command::new("xdg-open").arg(&html_path).spawn(); }
    #[cfg(target_os = "windows")]
    { let _ = std::process::Command::new("explorer").arg(&html_path).spawn(); }

    Ok(())
}
