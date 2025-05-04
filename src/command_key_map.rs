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
        // Create a reversed map from `Command` to `KeyCode`.
        let mut keys_from_command: HashMap<Command, Vec<(KeyCode, KeyModifiers)>> = HashMap::new();
        for (key, command) in self.map.iter() {
            keys_from_command
                .entry(command.clone())
                .or_default()
                .push(*key);
        }

        keys_from_command
            .into_iter()
            .map(|(command, mut keys)| (command, Self::key_str_from_keys(&mut keys)))
            .collect()
    }

    fn key_str_from_keys(keys: &mut [(KeyCode, KeyModifiers)]) -> String {
        // Sort the keys for each command. `KeyCode::Char` comes first. Then by
        // `KeyModifiers`, and then by the key name.
        keys.sort_by_key(|(key, modifiers)| {
            (
                !matches!(key, KeyCode::Char(_)),
                modifiers.bits(),
                key.to_string(),
            )
        });

        let key_strings: Vec<String> = keys
            .iter()
            .map(|(key, modifiers)| Self::key_str_from_key(*key, *modifiers))
            .collect();
        key_strings.join(", ")
    }

    fn key_str_from_key(key: KeyCode, modifiers: KeyModifiers) -> String {
        let key_str = key.to_string();
        if modifiers == KeyModifiers::NONE {
            key_str
        } else if modifiers == KeyModifiers::SHIFT && matches!(key, KeyCode::Char(_)) {
            // `Shift` is already included in the `key_str` for `Char` keys.
            key_str
        } else if modifiers == KeyModifiers::CONTROL {
            // `^` is a well-known prefix for `Control` keys.
            format!("^{}", key_str.to_uppercase())
        } else {
            format!("{}+{}", modifiers, key_str)
        }
    }

    fn create_hash_map() -> HashMap<(KeyCode, KeyModifiers), Command> {
        let mut map = HashMap::new();
        for (key, command) in Self::key_map_list() {
            let present = map.insert(*key, command.clone());
            assert!(present.is_none(), "Duplicate key found: {key:?}");
        }
        map
    }

    #[rustfmt::skip]
    fn key_map_list() -> &'static [((KeyCode, KeyModifiers), Command)] {
        &[
            ((KeyCode::Char('h'), KeyModifiers::NONE), Command::Help),
            ((KeyCode::Char('q'), KeyModifiers::NONE), Command::Quit),

            ((KeyCode::Char('c'), KeyModifiers::NONE), Command::Copy),
            ((KeyCode::Char('d'), KeyModifiers::NONE), Command::ShowDiff),
            ((KeyCode::Enter, KeyModifiers::CONTROL), Command::ShowDiff),
            ((KeyCode::Char('s'), KeyModifiers::NONE), Command::ShowCommit),

            ((KeyCode::Enter, KeyModifiers::NONE), Command::Older),
            ((KeyCode::Right, KeyModifiers::NONE), Command::Older),
            ((KeyCode::Backspace, KeyModifiers::NONE), Command::Newer),
            ((KeyCode::Left, KeyModifiers::NONE), Command::Newer),

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

            ((KeyCode::Up, KeyModifiers::NONE), Command::PrevDiff),
            ((KeyCode::Down, KeyModifiers::NONE), Command::NextDiff),
            ((KeyCode::PageUp, KeyModifiers::NONE), Command::PrevPage),
            ((KeyCode::PageDown, KeyModifiers::NONE), Command::NextPage),
            ((KeyCode::Home, KeyModifiers::NONE), Command::FirstLine),
            ((KeyCode::End, KeyModifiers::NONE), Command::LastLine),

            ((KeyCode::Char('N'), KeyModifiers::SHIFT), Command::SearchPrev),
            ((KeyCode::Char('n'), KeyModifiers::NONE), Command::SearchNext),
        ]
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

            ("#SEARCHING", Command::SearchNext),
            ("Repeat previous search.", Command::SearchNext),
            ("Repeat previous search in reverse direction.", Command::SearchPrev),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_map_list_duplicate() {
        let mut map = HashMap::new();
        for (key, command) in CommandKeyMap::key_map_list() {
            let present = map.insert(*key, command.clone());
            assert!(present.is_none(), "Duplicate key found: {key:?}");
        }
    }

    #[test]
    fn key_str_from_keys() {
        assert_eq!(
            CommandKeyMap::key_str_from_keys(&mut [
                (KeyCode::Up, KeyModifiers::SHIFT),
                (KeyCode::Up, KeyModifiers::NONE),
                (KeyCode::Char('a'), KeyModifiers::CONTROL),
                (KeyCode::Char('A'), KeyModifiers::SHIFT),
                (KeyCode::Char('a'), KeyModifiers::NONE),
            ]),
            "a, A, ^A, Up, Shift+Up"
        );
    }

    #[test]
    fn key_str_from_key() -> anyhow::Result<()> {
        let target = CommandKeyMap::key_str_from_key;
        assert_eq!(target(KeyCode::Char('a'), KeyModifiers::NONE), "a");
        assert_eq!(target(KeyCode::Char('A'), KeyModifiers::SHIFT), "A");
        assert_eq!(target(KeyCode::Char('a'), KeyModifiers::CONTROL), "^A");

        assert_eq!(target(KeyCode::Up, KeyModifiers::NONE), "Up");
        assert_eq!(target(KeyCode::Up, KeyModifiers::SHIFT), "Shift+Up");
        Ok(())
    }
}
