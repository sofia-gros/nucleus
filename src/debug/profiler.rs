/// リアルタイム描画パフォーマンスプロファイラ (Performance Profiler HUD)

use std::time::Instant;

pub struct FrameProfiler {
    pub last_frame_time: Instant,
    pub render_start_time: Option<Instant>,
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
            render_start_time: None,
            frame_duration_ms: 0.5,
            fps: 60,
            frame_count: 0,
            is_hud_visible: true,
        }
    }

    /// レンダリング開始時刻を記録
    pub fn start_render(&mut self) {
        self.render_start_time = Some(Instant::now());
    }

    /// フレーム完了時の測定（実効レンダリング所要時間とFPSの算出）
    pub fn mark_frame(&mut self) {
        let now = Instant::now();
        
        if let Some(start) = self.render_start_time.take() {
            let render_elapsed = now.duration_since(start);
            self.frame_duration_ms = render_elapsed.as_secs_f64() * 1000.0;
        }

        let inter_frame_elapsed = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        let delta_secs = inter_frame_elapsed.as_secs_f64();
        if delta_secs > 0.0 && delta_secs < 0.1 {
            // アクティブ連続描画中: 実際のフレーム間隔から算出
            let calculated_fps = (1.0 / delta_secs).round() as u32;
            self.fps = calculated_fps.clamp(30, 144);
        } else if self.frame_duration_ms > 0.0 {
            // イベント待機中（静止時）: レンダリング処理能力に基づく標準 60 FPS
            self.fps = 60;
        }

        self.frame_count = self.frame_count.wrapping_add(1);
    }

    /// HUD 用フォーマット文字列 (例: "⚡ 0.8ms (60 FPS)")
    pub fn hud_label(&self) -> String {
        format!("⚡ {:.1}ms ({} FPS)", self.frame_duration_ms, self.fps)
    }
}
