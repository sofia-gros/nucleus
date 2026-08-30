/// リアルタイム描画パフォーマンスプロファイラ (Performance Profiler HUD)

use std::time::Instant;

pub struct FrameProfiler {
    pub last_frame_time: Instant,
    pub frame_duration_ms: f64,
    pub fps: u32,
    pub frame_count: u32,
    pub is_hud_visible: bool,
}

impl Default for FrameProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameProfiler {
    pub fn new() -> Self {
        Self {
            last_frame_time: Instant::now(),
            frame_duration_ms: 0.0,
            fps: 60,
            frame_count: 0,
            is_hud_visible: true,
        }
    }

    /// フレーム開始・終了時の測定
    pub fn mark_frame(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        self.frame_duration_ms = elapsed.as_secs_f64() * 1000.0;
        if self.frame_duration_ms > 0.0 {
            self.fps = ((1000.0 / self.frame_duration_ms).round() as u32).min(144);
        }
        self.frame_count = self.frame_count.wrapping_add(1);
    }

    /// HUD 用フォーマット文字列 (例: "⚡ 0.8ms (60 FPS)")
    pub fn hud_label(&self) -> String {
        format!("⚡ {:.1}ms ({} FPS)", self.frame_duration_ms, self.fps)
    }
}
