/// 起動時間計測および初期化パフォーマンスプロファイラモジュール

use std::time::{Instant, Duration};

/// 起動プロファイラ
#[derive(Clone, Debug)]
pub struct StartupProfiler {
    start_time: Instant,
    marks: Vec<(String, Duration)>,
}

impl StartupProfiler {
    /// 新規プロファイラの開始
    pub fn start() -> Self {
        Self {
            start_time: Instant::now(),
            marks: Vec::new(),
        }
    }

    /// 各フェーズのチェックポイント記録
    pub fn mark(&mut self, label: &str) {
        let elapsed = self.start_time.elapsed();
        self.marks.push((label.to_string(), elapsed));
    }

    /// 起動完了までの総所要時間
    pub fn total_elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// 起動プロファイルのサマリー文字列生成
    pub fn summary(&self) -> String {
        let total = self.total_elapsed();
        let mut out = format!("⚡ Nucleus Startup: {}ms\n", total.as_millis());
        let mut last = Duration::ZERO;
        for (label, time) in &self.marks {
            let diff = time.saturating_sub(last);
            out.push_str(&format!("  ├─ {}: +{}ms (total: {}ms)\n", label, diff.as_millis(), time.as_millis()));
            last = *time;
        }
        out
    }
}
