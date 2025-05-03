use std::io;

use crossterm::terminal;
use log::*;

/// Enable or disable the
/// [terminal raw mode](https://docs.rs/crossterm/latest/crossterm/terminal/index.html#raw-mode)
/// while its instance is in scope.
///
/// While it automatically resets the mode when it's out of scope,
/// `reset()` should be called explicitly when the mode should be reset
/// or at the end of the scope
/// to avoid it being dropped earlier by the compiler.
/// # Examples
/// ```no_run
/// # fn main() -> std::io::Result<()> {
/// use git_iblame::TerminalRawModeScope;
///
/// let mut terminal_raw_mode = TerminalRawModeScope::new(true)?;
/// // Do the work.
/// // If it returns early, the terminal raw mode will be reset automatically.
/// terminal_raw_mode.reset();
/// # Ok(())
/// # }
/// ```
pub struct TerminalRawModeScope {
    is_enabled: bool,
    is_reset: bool,
}

impl TerminalRawModeScope {
    /// Enable the raw mode if `enable` is true,
    /// or disable it if `enable` is false.
    pub fn new(enable: bool) -> io::Result<Self> {
        Self::enable(enable)?;
        Ok(Self {
            is_enabled: enable,
            is_reset: false,
        })
    }

    /// Reset the terminal raw mode.
    /// This should be called when the mode should be reset,
    /// or at the end of the scope.
    ///
    /// Even if the mode should be reset at the end of the scope,
    /// and that the `Drop` trait should reset the raw mode,
    /// it should be called to avoid it being dropped earlier by the compiler.
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
    /// Calls `reset()` if it's not already reset.
    /// This is called when the instance goes out of scope.
    fn drop(&mut self) {
        self.reset();
    }
}
