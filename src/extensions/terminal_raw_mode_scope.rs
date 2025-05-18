use std::io;

use crossterm::{execute, terminal};
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
/// use git_iblame::extensions::TerminalRawModeScope;
///
/// let mut terminal_raw_mode = TerminalRawModeScope::new(true)?;
/// // Do the work.
/// // If it returns early, the terminal raw mode will be reset automatically.
/// terminal_raw_mode.reset()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct TerminalRawModeScope {
    is_enabled: bool,
    is_alt_screen_enabled: bool,
    is_reset: bool,
}

impl TerminalRawModeScope {
    /// Enable the raw mode if `enable` is true,
    /// or disable it if `enable` is false.
    pub fn new(enable: bool) -> io::Result<Self> {
        Self::enable(enable)?;
        Ok(Self {
            is_enabled: enable,
            is_alt_screen_enabled: false,
            is_reset: false,
        })
    }

    /// Switches to the alternate screen, in addition to the raw mode.
    /// See `crossterm::terminal::EnterAlternateScreen`.
    pub fn new_with_alternate_screen() -> io::Result<Self> {
        Self::enable(true)?;
        Self::enable_alternate_screen(true)?;
        Ok(Self {
            is_enabled: true,
            is_alt_screen_enabled: true,
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
    pub fn reset(&mut self) -> io::Result<()> {
        if self.is_reset {
            return Ok(());
        }
        Self::enable(!self.is_enabled)?;
        if self.is_alt_screen_enabled {
            Self::enable_alternate_screen(false)?;
        }
        self.is_reset = true;
        Ok(())
    }

    fn enable(enable: bool) -> io::Result<()> {
        debug!("TerminalRawModeScope.enable({enable})");
        if enable {
            terminal::enable_raw_mode()
        } else {
            terminal::disable_raw_mode()
        }
    }

    fn enable_alternate_screen(enable: bool) -> io::Result<()> {
        debug!("TerminalRawModeScope.enable_alternate_screen({enable})");
        if enable {
            execute!(io::stdout(), terminal::EnterAlternateScreen)
        } else {
            execute!(io::stdout(), terminal::LeaveAlternateScreen)
        }
    }
}

impl Drop for TerminalRawModeScope {
    /// Calls `reset()` if it's not already reset.
    /// This is called when the instance goes out of scope.
    fn drop(&mut self) {
        if let Err(error) = self.reset() {
            warn!("Failed to change terminal raw mode, ignored: {error}");
        }
    }
}
