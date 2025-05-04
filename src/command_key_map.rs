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

    pub fn print_help(&self) {
        let key_str_from_command = self.key_str_from_command();
        for (help, command) in Self::help_list() {
            if let Some(heading) = help.strip_prefix('#') {
                println!("\n        {}\n", heading);
                continue;
            }
            let key_str = match command {
                Command::LineNumber(_) => "[number] + Enter".to_string(),
                _ => key_str_from_command.get(command).unwrap().clone(),
            };
            println!("  {:<20} {}", key_str, help);
        }
    }

    fn key_str_from_command(&self) -> HashMap<Command, String> {
        let mut keys_from_command: HashMap<Command, Vec<(KeyCode, KeyModifiers)>> = HashMap::new();
        for (key, command) in self.map.iter() {
            keys_from_command
                .entry(command.clone())
                .or_insert_with(Vec::new)
                .push(key.clone());
        }

        let key_str_from_command: HashMap<Command, String> = keys_from_command
            .iter()
            .map(|(command, keys)| {
                let key_strings: Vec<String> = keys
                    .iter()
                    .map(|(key, modifiers)| Self::key_str_from_key(*key, *modifiers))
                    .collect();
                (command.clone(), key_strings.join(", "))
            })
            .collect();
        key_str_from_command
    }

    fn key_str_from_key(key: KeyCode, modifiers: KeyModifiers) -> String {
        let key_str = key.to_string();
        if modifiers == KeyModifiers::NONE {
            key_str
        } else if modifiers == KeyModifiers::CONTROL {
            format!("^{}", key_str.to_uppercase())
        } else {
            format!("{}+{}", modifiers, key_str)
        }
    }

    #[rustfmt::skip]
    fn create_hash_map() ->HashMap<(KeyCode, KeyModifiers), Command> {
        HashMap::from([
            ((KeyCode::Char('c'), KeyModifiers::NONE), Command::Copy),
            ((KeyCode::Char('d'), KeyModifiers::NONE), Command::ShowDiff),
            ((KeyCode::Char('h'), KeyModifiers::NONE), Command::Help),
            ((KeyCode::Char('s'), KeyModifiers::NONE), Command::ShowCommit),
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

    #[rustfmt::skip]
    fn help_list() -> &'static [(&'static str, Command)] {
        &[
            ("Show this help.", Command::Help),
            ("Quit the program.", Command::Quit),

            ("#COMMITS", Command::ShowCommit),
            ("Show the current line commit.", Command::ShowCommit),
            ("Show the current file of the current line commit.", Command::ShowDiff),
            ("Copy the current line commit ID to clipboard.", Command::Copy),

            ("#TRAVERSING TREES", Command::Older),
            ("Show the parent tree of the current line commit.", Command::Older),
            ("Back to the last tree.", Command::Newer),

            ("#MOVING", Command::NextDiff),
            ("Move to the next diff.", Command::NextDiff),
            ("Move to the previous diff.", Command::PrevDiff),
            ("Move to the next page.", Command::NextPage),
            ("Move to the previous page.", Command::PrevPage),
            ("Move to the first line.", Command::FirstLine),
            ("Move to the last line.", Command::LastLine),
            ("Move to the line number.", Command::LineNumber(0)),
            ("Repaint the screen.", Command::Repaint),
        ]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn key_str_from_key() -> anyhow::Result<()> {
        let target = CommandKeyMap::key_str_from_key;
        assert_eq!(target(KeyCode::Char('a'), KeyModifiers::NONE), "a");
        assert_eq!(target(KeyCode::Char('a'), KeyModifiers::CONTROL), "^A");
        Ok(())
    }
}
