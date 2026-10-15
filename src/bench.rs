//! Frame-time benchmark. Run with `UNPRICED_BENCH=<frames>` to force continuous
//! repaints, cycle through the tabs, print frame statistics, and exit.

use std::time::Instant;

use eframe::egui;

const WARMUP_FRAMES: usize = 120;

pub struct Bench {
    frames: usize,
    cpu_ms: Vec<f32>,
    frame_starts: Vec<Instant>,
}

impl Bench {
    pub fn from_env() -> Option<Self> {
        let frames = std::env::var("UNPRICED_BENCH").ok()?.parse().ok()?;
        Some(Self { frames, cpu_ms: Vec::with_capacity(frames), frame_starts: Vec::with_capacity(frames) })
    }

    /// Returns the tab index to show this frame.
    pub fn tick(&mut self, ctx: &egui::Context, frame: &eframe::Frame, frame_nr: usize) -> usize {
        if self.frame_starts.len() >= self.frames {
            return frame_nr / 30;
        }
        ctx.request_repaint();
        if frame_nr >= WARMUP_FRAMES {
            if let Some(cpu) = frame.info().cpu_usage {
                self.cpu_ms.push(cpu * 1000.0);
            }
            self.frame_starts.push(Instant::now());
        }
        if self.frame_starts.len() >= self.frames {
            self.report();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        frame_nr / 30
    }

    fn report(&mut self) {
        let mut cpu = std::mem::take(&mut self.cpu_ms);
        cpu.sort_by(f32::total_cmp);
        let pct = |p: f32| cpu[((cpu.len() - 1) as f32 * p) as usize];
        let wall = self.frame_starts.last().unwrap().duration_since(self.frame_starts[0]);
        let fps = (self.frame_starts.len() - 1) as f64 / wall.as_secs_f64();
        println!(
            "n={} fps={:.1} cpu_ms p50={:.3} p95={:.3} max={:.3}",
            cpu.len(),
            fps,
            pct(0.5),
            pct(0.95),
            cpu.last().unwrap()
        );
    }
}
