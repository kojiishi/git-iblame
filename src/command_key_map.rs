use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::Command;

#[derive(Debug, Default)]
pub struct CommandKeyMap {
    map: HashMap<(KeyCode, KeyModifiers), Command>,
}

impl CommandKeyMap {
    pub fn new() -> Self {
        CommandKeyMap {
            map: Self::create_hash_map(),
        }
    }

    pub fn get(&self, key_code: KeyCode, modifiers: KeyModifiers) -> Option<&Command> {
        self.map.get(&(key_code, modifiers))
    }

    #[rustfmt::skip]
    fn create_hash_map() ->HashMap<(KeyCode, KeyModifiers), Command> {
        HashMap::from([
            ((KeyCode::Char('c'), KeyModifiers::NONE), Command::Copy),
            ((KeyCode::Char('s'), KeyModifiers::NONE), Command::Show),
            ((KeyCode::Char('q'), KeyModifiers::NONE), Command::Quit),

            // `vi`, `emacs`, or `less`-like key bindings.
            ((KeyCode::Char('b'), KeyModifiers::NONE), Command::PrevPage),
            ((KeyCode::Char('b'), KeyModifiers::CONTROL), Command::PrevPage),
            ((KeyCode::Char('f'), KeyModifiers::NONE), Command::NextPage),
            ((KeyCode::Char('f'), KeyModifiers::CONTROL), Command::NextPage),
            ((KeyCode::Char('j'), KeyModifiers::NONE), Command::NextDiff),
            ((KeyCode::Char('k'), KeyModifiers::NONE), Command::PrevDiff),
            ((KeyCode::Char('l'), KeyModifiers::CONTROL), Command::Repaint),
            ((KeyCode::Char('n'), KeyModifiers::CONTROL), Command::NextDiff),
            ((KeyCode::Char('p'), KeyModifiers::CONTROL), Command::PrevDiff),
            ((KeyCode::Char('r'), KeyModifiers::NONE), Command::Repaint),
            ((KeyCode::Char('r'), KeyModifiers::CONTROL), Command::Repaint),

            ((KeyCode::Enter, KeyModifiers::NONE), Command::Older),
            ((KeyCode::Backspace, KeyModifiers::NONE), Command::Newer),
            ((KeyCode::Up, KeyModifiers::NONE), Command::PrevDiff),
            ((KeyCode::Down, KeyModifiers::NONE), Command::NextDiff),
            ((KeyCode::PageUp, KeyModifiers::NONE), Command::PrevPage),
            ((KeyCode::PageDown, KeyModifiers::NONE), Command::NextPage),
            ((KeyCode::Home, KeyModifiers::NONE), Command::FirstLine),
            ((KeyCode::End, KeyModifiers::NONE), Command::LastLine),
        ])
    }
}
