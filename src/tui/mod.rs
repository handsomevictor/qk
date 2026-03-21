/// Interactive TUI mode — launched with `qk --ui <files>`.
mod app;
mod events;
mod ui;

use crate::record::Record;
use crate::util::error::Result;

/// Launch the interactive TUI browser.
///
/// `all_records` are the pre-loaded, pre-cast records to browse.
/// `file_names` are used only for the status bar display.
pub fn run(all_records: Vec<Record>, file_names: &[String]) -> Result<()> {
    let app = app::App::new(all_records, file_names);
    events::run(app)
}
