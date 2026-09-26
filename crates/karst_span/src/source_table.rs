//! @path: karst/crates/karst_span/source_table.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::crates::karst_span::source_table

use std::{rc::Rc, str};

/// source file (single file struct)
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: String,
    pub src: Rc<str>,
    line_starts: Vec<u32>,
}

impl SourceFile {
    fn new(name: &str, src: &str) -> Self {
        let line_starts = compute_line_starts(src);
        SourceFile {
            name: name.to_string(),
            src: Rc::from(src),
            line_starts,
        }
    }

    /// get Source file content (line, col) position tuple
    fn line_col(&self, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(self.src.len()) as u32; // grand
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx] as usize;
        let col_chars = self.src[line_start..offset as usize].chars().count() as u32 + 1; // line
        // start to end

        (line_idx as u32 + 1, col_chars)
    }

    /// get Source file single line content from start to end
    fn line_text(&self, line: u32) -> &str {
        let start_idx = (line as usize).saturating_sub(1);
        if start_idx >= self.line_starts.len() {
            return ""; // over return ""
        }
        let start = self.line_starts[start_idx] as usize;
        let end = self
            .line_starts
            .get(start_idx + 1)
            .map(|&e| e as usize)
            .unwrap_or(self.src.len()); // current line end == next line [start, end)
        self.src[start..end].trim_end_matches(['\n', '\r'])
    }
}

/// from source file content get line starts offset vector
fn compute_line_starts(src: &str) -> Vec<u32> {
    let mut start_offsets = vec![0u32];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            start_offsets.push(i as u32 + 1);
        }
    }
    start_offsets
}

/// Source file register table
#[derive(Debug, Default, Clone)]
pub struct SourceTable {
    files: Vec<SourceFile>,
}

impl SourceTable {
    pub fn new() -> Self {
        SourceTable { files: Vec::new() }
    }

    /// add source file to Source table, return source file push position idx to use file id
    pub fn add_file(&mut self, name: &str, src: &str) -> u32 {
        let id = self.files.len() as u32;
        self.files.push(SourceFile::new(name, src));
        id
    }

    /// query source file from file id
    pub fn file(&self, file_id: u32) -> Option<&SourceFile> {
        self.files.get(file_id as usize)
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Span to readable: "file:line:col[expansion]"
    /// ```
    /// main.krt:3:5
    /// ```
    pub fn render_location(&self, file_id: u32, offset: u32, expansion_id: u32) -> String {
        match self.file(file_id) {
            Some(f) => {
                let (line, col) = f.line_col(offset);
                if expansion_id > 0 {
                    format!("{}:{}:{} (expansion {})", f.name, line, col, expansion_id)
                } else {
                    format!("{}:{}:{}", f.name, line, col)
                }
            }
            None => format!("<unknown file {}>", file_id),
        }
    }

    /// Diagnostic infor excerpt part
    /// ```
    ///    1 | (+ # 1)
    ///      |    ^
    /// ```
    pub fn excerpt(&self, file_id: u32, offset: u32) -> String {
        match self.file(file_id) {
            Some(f) => {
                let (line, col) = f.line_col(offset);
                let text = f.line_text(line);
                let padding = " ".repeat(col.saturating_sub(1) as usize);
                format!("{:>4} | {}\n     | {}^", line, text, padding)
            }
            None => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_basic() {
        let mut sm = SourceTable::new();
        let id = sm.add_file("t.krt", "(define x 1)\n(print x)\n");
        let f = sm.file(id).unwrap();
        assert_eq!(f.line_col(0), (1, 1));
        assert_eq!(f.line_col(1), (1, 2));

        // 第二行从 13 开始（"(define x 1)\n" = 13 字节）
        assert_eq!(f.line_col(12), (1, 13));
        assert_eq!(f.line_col(13), (2, 1));
        assert_eq!(f.line_col(14), (2, 2));
    }

    #[test]
    fn line_col_utf8() {
        let mut sm = SourceTable::new();
        let id = sm.add_file("t.krt", "(+ \"你好\" 1)");
        let f = sm.file(id).unwrap();

        // 字节 4 是 "你" 的第一个字节；列按字符计：src[0..4] = `(+ "` → 第 5 列
        assert_eq!(f.line_col(4), (1, 5));
        // 字节 7 是 "好" 的第一个字节：src[0..7] 前 4 字符 + 1 个汉字 = 第 6 列
        assert_eq!(f.line_col(7), (1, 6));
    }

    #[test]
    fn render_location_and_excerpt() {
        let mut sm = SourceTable::new();
        let id = sm.add_file("t.krf", "(define x 1)\n");
        let loc = sm.render_location(id, 9, 0);
        assert_eq!(loc, "t.krf:1:10");
        let exp = sm.excerpt(id, 9);
        assert!(exp.contains("(define x 1)"));
        assert!(exp.contains('^'));
    }

    #[test]
    fn unknown_file_is_explicit() {
        let sm = SourceTable::new();
        assert_eq!(sm.render_location(7, 0, 0), "<unknown file 7>");
    }
}
