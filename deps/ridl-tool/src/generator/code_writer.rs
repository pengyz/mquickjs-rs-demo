#[derive(Debug, Default, Clone)]
pub(crate) struct CodeWriter {
    lines: Vec<String>,
    indent: usize,
}

impl CodeWriter {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push_line(&mut self, line: impl AsRef<str>) {
        let line = line.as_ref();
        if line.is_empty() {
            self.lines.push(String::new());
            return;
        }

        let mut s = String::new();
        for _ in 0..self.indent {
            s.push_str("    ");
        }
        s.push_str(line);
        self.lines.push(s);
    }

    pub(crate) fn indent(&mut self) {
        self.indent += 1;
    }

    pub(crate) fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    pub(crate) fn into_string(self) -> String {
        self.lines.join("\n")
    }
}
