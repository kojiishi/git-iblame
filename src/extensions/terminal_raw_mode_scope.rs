use std::io;

use crossterm::terminal;
use log::*;

/// Enable or disable the
/// [terminal raw mode](https://docs.rs/crossterm/latest/crossterm/terminal/index.html#raw-mode)
/// while its instance is in scope.
///
/// While it automatically resets the mode when it's out of scope,
/// `reset()` should be called at the end
/// to avoid it being dropped earlier.
/// # Examples
/// ```no_run
/// # fn main() -> std::io::Result<()> {
/// use git_iblame::TerminalRawModeScope;
///
/// let mut terminal_raw_mode = TerminalRawModeScope::new(true)?;
/// // Do work.
/// terminal_raw_mode.reset();
/// # Ok(())
/// # }
/// ```
pub struct TerminalRawModeScope {
    is_enabled: bool,
    is_reset: bool,
}

impl TerminalRawModeScope {
    pub fn new(enable: bool) -> io::Result<Self> {
        Self::enable(enable)?;
        Ok(Self {
            is_enabled: enable,
            is_reset: false,
        })
    }

    pub fn reset(&mut self) {
        if !self.is_reset {
            if let Err(error) = Self::enable(!self.is_enabled) {
                warn!("Failed to change terminal raw mode, ignored: {error}");
            }
            self.is_reset = true;
        }
    }

    fn enable(enable: bool) -> io::Result<()> {
        debug!("TerminalRawModeScope.enable({enable})");
        if enable {
            terminal::enable_raw_mode()
        } else {
            terminal::disable_raw_mode()
        }
    }
}

impl Drop for TerminalRawModeScope {
    fn drop(&mut self) {
        self.reset();
    }
}
