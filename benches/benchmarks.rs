/// Nucleus コア機能のパフォーマンス・マイクロベンチマーク

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use nucleus::workspace::command_palette::fuzzy::fuzzy_match;
use nucleus::editor::bracket_match::BracketMatchEngine;
use nucleus::editor::find_replace::FindReplaceState;

// 1. 旧方式: 線形走査による Git ステータス判定 (O(M x N))
fn legacy_git_lookup(file_path: &str, git_nodes: &[serde_json::Value]) -> Option<String> {
    let norm_file = file_path.replace('\\', "/");
    for node in git_nodes {
        if let Some(p) = node.get("path").and_then(|p| p.as_str()) {
            let norm_p = p.replace('\\', "/");
            if norm_file.ends_with(&format!("/{}", norm_p.trim_start_matches('/'))) {
                return node.get("status").and_then(|s| s.as_str()).map(|s| s.to_string());
            }
        }
    }
    None
}

// 2. 新方式: 事前ハッシュマップによる O(1) ルックアップ
fn optimized_git_lookup(file_path: &str, map: &HashMap<String, String>) -> Option<String> {
    let norm_file = file_path.replace('\\', "/");
    if let Some(s) = map.get(&norm_file) {
        return Some(s.clone());
    }
    for (k, v) in map {
        if norm_file.ends_with(&format!("/{}", k)) {
            return Some(v.clone());
        }
    }
    None
}

fn bench_git_status_lookup(c: &mut Criterion) {
    let git_nodes: Vec<serde_json::Value> = (0..100).map(|i| {
        serde_json::json!({
            "path": format!("src/module_{}/file_{}.rs", i, i),
            "status": "M"
        })
    }).collect();

    let mut git_map = HashMap::new();
    for node in &git_nodes {
        let p = node["path"].as_str().unwrap().to_string();
        let s = node["status"].as_str().unwrap().to_string();
        git_map.insert(p, s);
    }

    let target_file = "A:/Project/nucleus/src/module_50/file_50.rs";

    let mut group = c.benchmark_group("Git Status Lookup");
    group.bench_function("Legacy Linear Search", |b| {
        b.iter(|| {
            legacy_git_lookup(black_box(target_file), black_box(&git_nodes))
        })
    });

    group.bench_function("Optimized Map Lookup", |b| {
        b.iter(|| {
            optimized_git_lookup(black_box(target_file), black_box(&git_map))
        })
    });
    group.finish();
}

fn bench_fuzzy_search(c: &mut Criterion) {
    let targets: Vec<String> = (0..1000).map(|i| format!("src/components/panel_{}/item_{}.rs", i, i)).collect();
    let query = "panel_50";

    c.bench_function("Fuzzy Match 1000 files", |b| {
        b.iter(|| {
            let mut matches = 0;
            for t in &targets {
                if fuzzy_match(black_box(query), black_box(t)).is_some() {
                    matches += 1;
                }
            }
            matches
        })
    });
}

fn bench_bracket_matching(c: &mut Criterion) {
    let sample_code = r#"
        fn complex_function() {
            if (a > 0) {
                let list = vec![1, 2, 3];
                for item in list {
                    println!("Item: {}", item);
                }
            }
        }
    "#.repeat(20);

    c.bench_function("Bracket Match 160 lines", |b| {
        b.iter(|| {
            BracketMatchEngine::find_bracket_pairs(black_box(&sample_code))
        })
    });
}

fn bench_find_replace(c: &mut Criterion) {
    let sample_text = "fn calculate_total() -> i32 { let total = 100; total }\n".repeat(50);
    let mut state = FindReplaceState::new();
    state.query = "total".to_string();

    c.bench_function("Find Replace 50 lines", |b| {
        b.iter(|| {
            state.update_matches(black_box(&sample_text))
        })
    });
}

criterion_group!(
    benches,
    bench_git_status_lookup,
    bench_fuzzy_search,
    bench_bracket_matching,
    bench_find_replace
);
criterion_main!(benches);
