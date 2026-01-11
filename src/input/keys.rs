//! Key definitions and escape sequence generation.

/// Represents a keyboard key or key combination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// A regular character key.
    Char(char),
    /// Enter/Return key.
    Enter,
    /// Tab key.
    Tab,
    /// Escape key.
    Escape,
    /// Backspace key.
    Backspace,
    /// Delete key.
    Delete,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page Up key.
    PageUp,
    /// Page Down key.
    PageDown,
    /// Function key (F1-F12).
    F(u8),
    /// Ctrl + character.
    Ctrl(char),
    /// Alt + character.
    Alt(char),
}

impl Key {
    /// Convert the key to its escape sequence bytes.
    pub fn to_escape_sequence(&self) -> Vec<u8> {
        match self {
            Key::Char(c) => {
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf).as_bytes().to_vec()
            }
            Key::Enter => vec![b'\r'],
            Key::Tab => vec![b'\t'],
            Key::Escape => vec![0x1b],
            Key::Backspace => vec![0x7f],
            Key::Delete => vec![0x1b, b'[', b'3', b'~'],
            Key::Up => vec![0x1b, b'[', b'A'],
            Key::Down => vec![0x1b, b'[', b'B'],
            Key::Right => vec![0x1b, b'[', b'C'],
            Key::Left => vec![0x1b, b'[', b'D'],
            Key::Home => vec![0x1b, b'[', b'H'],
            Key::End => vec![0x1b, b'[', b'F'],
            Key::PageUp => vec![0x1b, b'[', b'5', b'~'],
            Key::PageDown => vec![0x1b, b'[', b'6', b'~'],
            Key::F(n) => match n {
                1 => vec![0x1b, b'O', b'P'],
                2 => vec![0x1b, b'O', b'Q'],
                3 => vec![0x1b, b'O', b'R'],
                4 => vec![0x1b, b'O', b'S'],
                5 => vec![0x1b, b'[', b'1', b'5', b'~'],
                6 => vec![0x1b, b'[', b'1', b'7', b'~'],
                7 => vec![0x1b, b'[', b'1', b'8', b'~'],
                8 => vec![0x1b, b'[', b'1', b'9', b'~'],
                9 => vec![0x1b, b'[', b'2', b'0', b'~'],
                10 => vec![0x1b, b'[', b'2', b'1', b'~'],
                11 => vec![0x1b, b'[', b'2', b'3', b'~'],
                12 => vec![0x1b, b'[', b'2', b'4', b'~'],
                _ => vec![],
            },
            Key::Ctrl(c) => {
                // Ctrl+A = 0x01, Ctrl+B = 0x02, etc.
                let c = c.to_ascii_lowercase();
                if c.is_ascii_lowercase() {
                    vec![(c as u8) - b'a' + 1]
                } else {
                    vec![]
                }
            }
            Key::Alt(c) => {
                // Alt is ESC followed by the character
                let mut seq = vec![0x1b];
                let mut buf = [0u8; 4];
                seq.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                seq
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_sequence() {
        assert_eq!(Key::Char('a').to_escape_sequence(), vec![b'a']);
        assert_eq!(Key::Char('Z').to_escape_sequence(), vec![b'Z']);
    }

    #[test]
    fn test_special_keys() {
        assert_eq!(Key::Enter.to_escape_sequence(), vec![b'\r']);
        assert_eq!(Key::Tab.to_escape_sequence(), vec![b'\t']);
        assert_eq!(Key::Escape.to_escape_sequence(), vec![0x1b]);
    }

    #[test]
    fn test_arrow_keys() {
        assert_eq!(Key::Up.to_escape_sequence(), vec![0x1b, b'[', b'A']);
        assert_eq!(Key::Down.to_escape_sequence(), vec![0x1b, b'[', b'B']);
        assert_eq!(Key::Right.to_escape_sequence(), vec![0x1b, b'[', b'C']);
        assert_eq!(Key::Left.to_escape_sequence(), vec![0x1b, b'[', b'D']);
    }

    #[test]
    fn test_ctrl_key() {
        assert_eq!(Key::Ctrl('c').to_escape_sequence(), vec![0x03]); // Ctrl+C
        assert_eq!(Key::Ctrl('a').to_escape_sequence(), vec![0x01]); // Ctrl+A
        assert_eq!(Key::Ctrl('z').to_escape_sequence(), vec![0x1a]); // Ctrl+Z
    }

    #[test]
    fn test_alt_key() {
        assert_eq!(Key::Alt('x').to_escape_sequence(), vec![0x1b, b'x']);
    }
}
