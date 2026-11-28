//! 組み込み Extension「Word Tools」のソース。`build.sh` で WASM にして extensions/nucleus.word-tools/ に置く。

use nucleus_extension_api::{Extension, export_extension, host};

struct WordTools;

impl Extension for WordTools {
    fn activate() -> Result<(), String> {
        host::log(host::LogLevel::Info, "Word Tools activated");
        Ok(())
    }

    fn run_command(id: String) -> Result<(), String> {
        match id.as_str() {
            "word-tools.count" => {
                let Some(doc) = host::active_document() else {
                    host::notify(host::NotificationLevel::Warning, "No file is open.");
                    return Ok(());
                };
                let words = doc.text.split_whitespace().count();
                let lines = doc.text.lines().count();
                let chars = doc.text.chars().count();
                host::notify(
                    host::NotificationLevel::Info,
                    &format!("{}: {words} words, {lines} lines, {chars} characters", doc.path),
                );
                Ok(())
            }
            "word-tools.sort-lines" => {
                let Some(doc) = host::active_document() else { return Ok(()) };
                let trailing_newline = doc.text.ends_with('\n');
                let mut lines: Vec<&str> = doc.text.lines().collect();
                lines.sort_unstable();
                let mut text = lines.join("\n");
                if trailing_newline {
                    text.push('\n');
                }
                host::replace_active_text(&text)
            }
            other => Err(format!("unknown command `{other}`")),
        }
    }
}

export_extension!(WordTools);
