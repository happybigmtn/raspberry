use fabro_llm::types::{ToolCall, ToolResult};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default)]
struct FileOps {
    read: bool,
    written: bool,
    edited: bool,
}

#[derive(Debug, Default)]
pub struct FileTracker {
    files: BTreeMap<String, FileOps>,
}

impl FileTracker {
    pub fn record_read(&mut self, path: &str) {
        self.files.entry(path.to_string()).or_default().read = true;
    }

    pub fn record_write(&mut self, path: &str) {
        self.files.entry(path.to_string()).or_default().written = true;
    }

    pub fn record_edit(&mut self, path: &str) {
        self.files.entry(path.to_string()).or_default().edited = true;
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn read_files(&self) -> Vec<String> {
        self.files
            .iter()
            .filter_map(|(path, ops)| ops.read.then_some(path.clone()))
            .collect()
    }

    pub fn written_files(&self) -> Vec<String> {
        self.files
            .iter()
            .filter_map(|(path, ops)| (ops.written || ops.edited).then_some(path.clone()))
            .collect()
    }

    pub fn touched_files(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        for (path, ops) in &self.files {
            let mut labels = Vec::new();
            if ops.read {
                labels.push("read");
            }
            if ops.written {
                labels.push("written");
            }
            if ops.edited {
                labels.push("edited");
            }
            output.push_str(&format!("- {path} ({})\n", labels.join(", ")));
        }
        output
    }

    pub fn record_from_tool_calls(&mut self, tool_calls: &[ToolCall], results: &[ToolResult]) {
        for (tc, result) in tool_calls.iter().zip(results.iter()) {
            if result.is_error {
                continue;
            }
            match tc.name.as_str() {
                "read_file" => {
                    if let Some(path) = tc.arguments.get("file_path").and_then(|v| v.as_str()) {
                        self.record_read(path);
                    }
                }
                "read_many_files" => {
                    if let Some(paths) = tc.arguments.get("paths").and_then(|v| v.as_array()) {
                        for path in paths.iter().filter_map(|v| v.as_str()) {
                            self.record_read(path);
                        }
                    }
                }
                "write_file" => {
                    if let Some(path) = tc.arguments.get("file_path").and_then(|v| v.as_str()) {
                        self.record_write(path);
                    }
                }
                "edit_file" => {
                    if let Some(path) = tc.arguments.get("file_path").and_then(|v| v.as_str()) {
                        self.record_edit(path);
                    }
                }
                "apply_patch" => {
                    let content = match result.content.as_str() {
                        Some(s) => s.to_string(),
                        None => result.content.to_string(),
                    };
                    for line in content.lines() {
                        let line = line.trim();
                        if let Some(path) = line.strip_prefix("Added file: ") {
                            self.record_write(path.trim());
                        } else if let Some(path) = line.strip_prefix("Updated file: ") {
                            self.record_edit(path.trim());
                        }
                    }
                }
                "grep" => {
                    let content = match result.content.as_str() {
                        Some(s) => s.to_string(),
                        None => result.content.to_string(),
                    };
                    for line in content.lines() {
                        if let Some(path) = line.split(':').next() {
                            let path = path.trim();
                            if !path.is_empty() {
                                self.record_read(path);
                            }
                        }
                    }
                }
                "glob" => {
                    let content = match result.content.as_str() {
                        Some(s) => s.to_string(),
                        None => result.content.to_string(),
                    };
                    for line in content.lines() {
                        let path = line.trim();
                        if !path.is_empty() {
                            self.record_read(path);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_read_renders_read_flag() {
        let mut tracker = FileTracker::default();
        tracker.record_read("src/main.rs");
        assert_eq!(tracker.render(), "- src/main.rs (read)\n");
    }

    #[test]
    fn record_write_and_edit_renders_all_ops() {
        let mut tracker = FileTracker::default();
        tracker.record_read("src/lib.rs");
        tracker.record_write("src/lib.rs");
        tracker.record_edit("src/lib.rs");
        assert_eq!(tracker.render(), "- src/lib.rs (read, written, edited)\n");
    }

    #[test]
    fn multiple_files_sorted_by_path() {
        let mut tracker = FileTracker::default();
        tracker.record_write("z.rs");
        tracker.record_read("a.rs");
        let rendered = tracker.render();
        assert_eq!(rendered, "- a.rs (read)\n- z.rs (written)\n");
    }

    #[test]
    fn record_from_tool_calls_read_file() {
        let mut tracker = FileTracker::default();
        let tool_calls = vec![ToolCall::new(
            "tc1",
            "read_file",
            serde_json::json!({"file_path": "/tmp/foo.rs"}),
        )];
        let results = vec![ToolResult::success(
            "tc1",
            serde_json::json!("file contents"),
        )];
        tracker.record_from_tool_calls(&tool_calls, &results);
        assert_eq!(tracker.render(), "- /tmp/foo.rs (read)\n");
    }

    #[test]
    fn record_from_tool_calls_write_file() {
        let mut tracker = FileTracker::default();
        let tool_calls = vec![ToolCall::new(
            "tc1",
            "write_file",
            serde_json::json!({"file_path": "/tmp/bar.rs", "content": "hello"}),
        )];
        let results = vec![ToolResult::success("tc1", serde_json::json!("ok"))];
        tracker.record_from_tool_calls(&tool_calls, &results);
        assert_eq!(tracker.render(), "- /tmp/bar.rs (written)\n");
    }

    #[test]
    fn record_from_tool_calls_edit_file() {
        let mut tracker = FileTracker::default();
        let tool_calls = vec![ToolCall::new(
            "tc1",
            "edit_file",
            serde_json::json!({"file_path": "/tmp/baz.rs"}),
        )];
        let results = vec![ToolResult::success("tc1", serde_json::json!("ok"))];
        tracker.record_from_tool_calls(&tool_calls, &results);
        assert_eq!(tracker.render(), "- /tmp/baz.rs (edited)\n");
    }

    #[test]
    fn record_from_tool_calls_skips_errors() {
        let mut tracker = FileTracker::default();
        let tool_calls = vec![ToolCall::new(
            "tc1",
            "read_file",
            serde_json::json!({"file_path": "/tmp/missing.rs"}),
        )];
        let results = vec![ToolResult::error("tc1", "File not found")];
        tracker.record_from_tool_calls(&tool_calls, &results);
        assert!(tracker.is_empty());
    }

    #[test]
    fn record_from_tool_calls_apply_patch_added() {
        let mut tracker = FileTracker::default();
        let tool_calls = vec![ToolCall::new(
            "tc1",
            "apply_patch",
            serde_json::json!({"patch": "..."}),
        )];
        let results = vec![ToolResult::success(
            "tc1",
            serde_json::json!("Added file: src/new.rs\nUpdated file: src/old.rs"),
        )];
        tracker.record_from_tool_calls(&tool_calls, &results);
        assert_eq!(
            tracker.render(),
            "- src/new.rs (written)\n- src/old.rs (edited)\n"
        );
    }

    #[test]
    fn is_empty_and_file_count() {
        let mut tracker = FileTracker::default();
        assert!(tracker.is_empty());
        assert_eq!(tracker.file_count(), 0);

        tracker.record_read("a.rs");
        tracker.record_write("b.rs");
        assert!(!tracker.is_empty());
        assert_eq!(tracker.file_count(), 2);
    }

    #[test]
    fn record_from_tool_calls_ignores_unknown_tools() {
        let mut tracker = FileTracker::default();
        let tool_calls = vec![ToolCall::new(
            "tc1",
            "shell",
            serde_json::json!({"command": "ls"}),
        )];
        let results = vec![ToolResult::success(
            "tc1",
            serde_json::json!("file1\nfile2"),
        )];
        tracker.record_from_tool_calls(&tool_calls, &results);
        assert!(tracker.is_empty());
    }

    #[test]
    fn read_and_written_views_split_operations() {
        let mut tracker = FileTracker::default();
        tracker.record_read("src/lib.rs");
        tracker.record_write("src/main.rs");
        tracker.record_edit("src/config.rs");

        assert_eq!(tracker.read_files(), vec!["src/lib.rs".to_string()]);
        assert_eq!(
            tracker.written_files(),
            vec!["src/config.rs".to_string(), "src/main.rs".to_string()]
        );
        assert_eq!(
            tracker.touched_files(),
            vec![
                "src/config.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/main.rs".to_string(),
            ]
        );
    }

    #[test]
    fn record_from_tool_calls_tracks_grep_and_glob_reads() {
        let mut tracker = FileTracker::default();
        let tool_calls = vec![
            ToolCall::new("tc1", "grep", serde_json::json!({"pattern": "foo"})),
            ToolCall::new("tc2", "glob", serde_json::json!({"pattern": "*.rs"})),
        ];
        let results = vec![
            ToolResult::success(
                "tc1",
                serde_json::json!("src/lib.rs:1:foo\nsrc/main.rs:2:foo"),
            ),
            ToolResult::success("tc2", serde_json::json!("src/bin.rs\nsrc/lib.rs")),
        ];
        tracker.record_from_tool_calls(&tool_calls, &results);

        assert_eq!(
            tracker.read_files(),
            vec![
                "src/bin.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/main.rs".to_string(),
            ]
        );
        assert!(tracker.written_files().is_empty());
    }
}
