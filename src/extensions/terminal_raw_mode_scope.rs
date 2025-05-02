use std::io;

use crossterm::terminal;
use log::warn;

/// Enable or disable the
/// [terminal raw mode](https://docs.rs/crossterm/latest/crossterm/terminal/index.html#raw-mode)
/// while its instance is in scope.
/// # Examples
/// ```no_run
/// # use git_iblame::TerminalRawModeScope;
/// let _ = TerminalRawModeScope::new(true);
/// ```
pub struct TerminalRawModeScope {
    is_enabled: bool,
}

impl TerminalRawModeScope {
    pub fn new(enable: bool) -> io::Result<Self> {
        Self::enable(enable)?;
        Ok(Self { is_enabled: enable })
    }

    fn enable(enable: bool) -> io::Result<()> {
        if enable {
            terminal::enable_raw_mode()
        } else {
            terminal::disable_raw_mode()
        }
    }
}

impl Drop for TerminalRawModeScope {
    fn drop(&mut self) {
        if let Err(error) = Self::enable(!self.is_enabled) {
            warn!("Failed to change terminal raw mode, ignored: {error}");
        }
    }
}
