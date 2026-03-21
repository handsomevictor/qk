/// TUI application state — query string, result records, scroll offset.
use crate::record::Record;

pub struct App {
    /// Query being typed by the user.
    pub query: String,
    /// Byte offset of the cursor within `query`.
    pub cursor_pos: usize,
    /// All records loaded from disk (never modified after load).
    pub all_records: Vec<Record>,
    /// Records after applying the current query.
    pub results: Vec<Record>,
    /// Vertical scroll offset (in lines) for the results pane.
    pub scroll: usize,
    /// Status message shown in the bottom bar.
    pub status: String,
    /// Eval error from the last query run, if any.
    pub error: Option<String>,
    /// Set to `true` when the user requests to quit.
    pub should_quit: bool,
}

impl App {
    /// Create a new App from pre-loaded records and the file names they came from.
    pub fn new(all_records: Vec<Record>, file_names: &[String]) -> Self {
        let count = all_records.len();
        let files_str = if file_names.is_empty() {
            "<stdin>".to_string()
        } else {
            file_names.join(", ")
        };
        App {
            query: String::new(),
            cursor_pos: 0,
            results: all_records.clone(),
            all_records,
            scroll: 0,
            status: format!("{count} records · {files_str}"),
            error: None,
            should_quit: false,
        }
    }

    /// Insert `c` at the current cursor position and advance the cursor.
    pub fn insert_char(&mut self, c: char) {
        self.query.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    /// Delete the character immediately before the cursor (Backspace behaviour).
    pub fn delete_char_before(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let prev = self.query[..self.cursor_pos]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.query.remove(prev);
        self.cursor_pos = prev;
    }

    /// Move the cursor one codepoint to the left.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        self.cursor_pos = self.query[..self.cursor_pos]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    /// Move the cursor one codepoint to the right.
    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos >= self.query.len() {
            return;
        }
        self.cursor_pos = self.query[self.cursor_pos..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| self.cursor_pos + i)
            .unwrap_or(self.query.len());
    }

    /// Re-evaluate `self.query` against `self.all_records` and update `self.results`.
    ///
    /// Detects DSL vs keyword mode automatically (same logic as `main.rs`).
    pub fn eval(&mut self) {
        use crate::query;

        if self.query.trim().is_empty() {
            self.results = self.all_records.clone();
            self.error = None;
            self.scroll = 0;
            return;
        }

        let args: Vec<String> = self.query.split_whitespace().map(String::from).collect();
        let first = args.first().map(String::as_str).unwrap_or("");
        let is_dsl = first.starts_with('.') || first.starts_with("not ") || first.starts_with('|');

        let result = if is_dsl {
            let expr = self.query.trim();
            query::dsl::parser::parse(expr).and_then(|(q, _)| {
                query::dsl::eval::eval(&q, self.all_records.clone()).map(|(r, _)| r)
            })
        } else {
            query::fast::parser::parse(&args).and_then(|(q, _)| {
                query::fast::eval::eval(&q, self.all_records.clone()).map(|(r, _)| r)
            })
        };

        match result {
            Ok(records) => {
                self.results = records;
                self.error = None;
                self.scroll = 0;
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }

    /// Scroll the results pane down by `n` lines.
    pub fn scroll_down(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_add(n);
    }

    /// Scroll the results pane up by `n` lines.
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }
}
