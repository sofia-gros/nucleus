pub fn match_git_path(file_path: &str, git_node_path: &str, is_dir: bool) -> bool {
    let norm_file = file_path.replace('\\', "/");
    let norm_git = git_node_path.replace('\\', "/");

    if is_dir {
        let trimmed_file = norm_file.trim_end_matches('/');
        let trimmed_git = norm_git.trim_start_matches('/');

        // 1. 完全先頭一致
        if trimmed_git.starts_with(trimmed_file) {
            return true;
        }
        // 2. 末尾パスが Git ノードの先頭に含まれるか
        for part in trimmed_file.split('/') {
            if !part.is_empty() && part != "A:" && part != "C:" && part != "D:" {
                if let Some(pos) = trimmed_file.rfind(part) {
                    let suffix = &trimmed_file[pos..];
                    if trimmed_git.starts_with(&format!("{}/", suffix)) {
                        return true;
                    }
                }
            }
        }
        false
    } else {
        // 1. 完全一致
        if norm_file == norm_git {
            return true;
        }
        // 2. 末尾サフィックス一致 (絶対パス vs 相対パス)
        if norm_file.ends_with(&format!("/{}", norm_git.trim_start_matches('/')))
            || norm_git.ends_with(&format!("/{}", norm_file.trim_start_matches('/')))
        {
            return true;
        }
        // 3. ルートなし一致
        norm_file.trim_start_matches("./") == norm_git.trim_start_matches("./")
    }
}

#[test]
fn test_git_path_matching_variations() {
    // 1. Windows 絶対パス vs 相対パス (スラッシュ)
    assert!(match_git_path(
        "A:\\Project\\nucleus\\src\\main.rs",
        "src/main.rs",
        false
    ));

    // 2. Windows 絶対パス vs Windows 相対パス (バックスラッシュ)
    assert!(match_git_path(
        "A:\\Project\\nucleus\\src\\workspace\\mod.rs",
        "src\\workspace\\mod.rs",
        false
    ));

    // 3. 同一の絶対パス
    assert!(match_git_path(
        "A:/Project/nucleus/Cargo.toml",
        "A:/Project/nucleus/Cargo.toml",
        false
    ));

    // 4. ディレクトリマッチ (フォルダ内に変更ファイルが存在する場合)
    assert!(match_git_path(
        "A:\\Project\\nucleus\\src\\workspace",
        "src/workspace/mod.rs",
        true
    ));

    // 5. 不一致ケース
    assert!(!match_git_path(
        "A:\\Project\\nucleus\\src\\lib.rs",
        "src/main.rs",
        false
    ));
}
