//! Opt-in development harness; excluded from ordinary builds.
use serde::Serialize;
use std::{path::PathBuf, time::Duration};

const SPEEDS: [u32; 6] = [200, 400, 800, 1200, 2000, 4000];
#[derive(Serialize)]
struct Sample {
    scheduled_ns: u64,
    wake_ns: u64,
    ui_ns: u64,
    submission_ns: Option<u64>,
}
pub struct Bench {
    directory: PathBuf,
    phase: usize,
    samples: Vec<Sample>,
    results: Vec<serde_json::Value>,
}
impl Bench {
    pub fn from_env() -> Option<Self> {
        std::env::var_os("VRX_BENCH_DIR").map(|p| Self {
            directory: PathBuf::from(p),
            phase: 0,
            samples: Vec::with_capacity(32),
            results: Vec::with_capacity(6),
        })
    }
    pub fn first_speed(&self) -> u32 {
        SPEEDS.first().copied().unwrap_or(200)
    }
    /// Complete each word's submission sample on the following timer event.
    /// Some(Some(speed)) starts the next phase; Some(None) means finished.
    pub fn observe(
        &mut self,
        scheduled: Duration,
        wake: Duration,
        ui: Duration,
        last_submission_ns: u64,
    ) -> Option<Option<u32>> {
        if let Some(previous) = self.samples.last_mut()
            && last_submission_ns >= previous.ui_ns
        {
            previous.submission_ns = Some(last_submission_ns);
        }
        self.samples.push(Sample {
            scheduled_ns: ns(scheduled),
            wake_ns: ns(wake),
            ui_ns: ns(ui),
            submission_ns: None,
        });
        if self.samples.len() < 31 {
            return None;
        }
        self.samples.truncate(30);
        let mut wake: Vec<u64> = self
            .samples
            .iter()
            .map(|s| s.wake_ns.saturating_sub(s.scheduled_ns))
            .collect();
        let mut ui: Vec<u64> = self
            .samples
            .iter()
            .map(|s| s.ui_ns.saturating_sub(s.wake_ns))
            .collect();
        let mut submission: Vec<u64> = self
            .samples
            .iter()
            .filter_map(|s| s.submission_ns.map(|t| t.saturating_sub(s.ui_ns)))
            .collect();
        wake.sort_unstable();
        ui.sort_unstable();
        submission.sort_unstable();
        let summary = |samples: &[u64]| serde_json::json!({"p50_ms":percentile(samples,50),"p95_ms":percentile(samples,95),"p99_ms":percentile(samples,99),"samples":samples.len()});
        self.results.push(serde_json::json!({"wpm":SPEEDS.get(self.phase),"wake_lateness":summary(&wake),"wake_to_ui":summary(&ui),"ui_to_draw_submission":summary(&submission),"raw":self.samples}));
        self.samples.clear();
        self.phase += 1;
        if let Some(wpm) = SPEEDS.get(self.phase) {
            Some(Some(*wpm))
        } else {
            let result = (|| -> vrx::Result<()> {
                std::fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
                let bytes = serde_json::to_vec_pretty(&serde_json::json!({"description":"Iced deadline subscription; renderer submission is not GPU completion or physical presentation","version":vrx::VERSION,"speeds":self.results})).map_err(|e|e.to_string())?;
                vrx::persistence::atomic_write(&self.directory.join("native-timing.json"), &bytes)
            })();
            if let Err(e) = result {
                eprintln!("Native benchmark failed: {e}");
            }
            Some(None)
        }
    }
}
fn ns(d: Duration) -> u64 {
    d.as_nanos().min(u128::from(u64::MAX)) as u64
}
fn percentile(samples: &[u64], p: usize) -> f64 {
    samples
        .get((samples.len().saturating_sub(1) * p).div_ceil(100))
        .copied()
        .unwrap_or(0) as f64
        / 1e6
}
