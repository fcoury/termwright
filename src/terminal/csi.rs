//! CSI query emulation for PTY-hosted applications.

use crate::screen::Position;

const ESC: u8 = 0x1b;
const CSI_C1: u8 = 0x9b;

#[derive(Debug, Default)]
enum ParserState {
    #[default]
    Ground,
    Esc,
    Csi {
        buf: Vec<u8>,
    },
}

#[derive(Debug, Default)]
pub struct CsiEmulator {
    parser_state: ParserState,
}

impl CsiEmulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_output(&mut self, bytes: &[u8], cursor: Position) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();

        for byte in bytes.iter().copied() {
            match &mut self.parser_state {
                ParserState::Ground => {
                    if byte == ESC {
                        self.parser_state = ParserState::Esc;
                    } else if byte == CSI_C1 {
                        self.parser_state = ParserState::Csi { buf: Vec::new() };
                    }
                }
                ParserState::Esc => {
                    if byte == b'[' {
                        self.parser_state = ParserState::Csi { buf: Vec::new() };
                    } else {
                        self.parser_state = ParserState::Ground;
                    }
                }
                ParserState::Csi { buf } => {
                    if (0x40..=0x7e).contains(&byte) {
                        if let Some(response) = handle_csi_query(buf, byte, cursor) {
                            responses.push(response);
                        }
                        self.parser_state = ParserState::Ground;
                    } else if (0x20..=0x3f).contains(&byte) {
                        buf.push(byte);
                    } else {
                        self.parser_state = ParserState::Ground;
                    }
                }
            }
        }

        responses
    }
}

fn handle_csi_query(params: &[u8], final_byte: u8, cursor: Position) -> Option<Vec<u8>> {
    if final_byte != b'n' {
        return None;
    }

    let params = std::str::from_utf8(params).ok()?.trim();
    let row = cursor.row.saturating_add(1);
    let col = cursor.col.saturating_add(1);

    match params {
        "6" => Some(format!("\u{1b}[{row};{col}R").into_bytes()),
        "?6" => Some(format!("\u{1b}[?{row};{col}R").into_bytes()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responds_to_csi_6n() {
        let mut csi = CsiEmulator::new();
        let responses = csi.process_output(b"\x1b[6n", Position::new(4, 9));
        assert_eq!(responses, vec![b"\x1b[5;10R".to_vec()]);
    }

    #[test]
    fn responds_to_private_csi_6n() {
        let mut csi = CsiEmulator::new();
        let responses = csi.process_output(b"\x1b[?6n", Position::new(2, 3));
        assert_eq!(responses, vec![b"\x1b[?3;4R".to_vec()]);
    }

    #[test]
    fn responds_to_c1_csi() {
        let mut csi = CsiEmulator::new();
        let responses = csi.process_output(b"\x9b6n", Position::new(0, 0));
        assert_eq!(responses, vec![b"\x1b[1;1R".to_vec()]);
    }

    #[test]
    fn handles_split_sequence_across_chunks() {
        let mut csi = CsiEmulator::new();
        assert!(csi.process_output(b"\x1b[", Position::new(0, 0)).is_empty());
        assert!(csi.process_output(b"6", Position::new(0, 0)).is_empty());
        let responses = csi.process_output(b"n", Position::new(0, 0));
        assert_eq!(responses, vec![b"\x1b[1;1R".to_vec()]);
    }

    #[test]
    fn ignores_non_cursor_query() {
        let mut csi = CsiEmulator::new();
        assert!(
            csi.process_output(b"\x1b[5n", Position::new(1, 1))
                .is_empty()
        );
    }
}
