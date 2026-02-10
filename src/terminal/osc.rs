//! OSC color query emulation for PTY-hosted applications.

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;

const DEFAULT_FG: Rgb8 = Rgb8 {
    r: 0xf0,
    g: 0xf0,
    b: 0xf0,
};
const DEFAULT_BG: Rgb8 = Rgb8 {
    r: 0x00,
    g: 0x00,
    b: 0x00,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OscTerminator {
    Bel,
    St,
}

#[derive(Debug)]
enum ParserState {
    Ground,
    Esc,
    Osc { buf: Vec<u8>, pending_esc: bool },
}

#[derive(Clone, Copy, Debug)]
pub struct OscColorState {
    foreground: Rgb8,
    background: Rgb8,
    cursor: Rgb8,
}

impl Default for OscColorState {
    fn default() -> Self {
        Self {
            foreground: DEFAULT_FG,
            background: DEFAULT_BG,
            cursor: DEFAULT_FG,
        }
    }
}

impl OscColorState {
    fn get(&self, code: u8) -> Option<Rgb8> {
        match code {
            10 => Some(self.foreground),
            11 => Some(self.background),
            12 => Some(self.cursor),
            _ => None,
        }
    }

    fn set(&mut self, code: u8, value: Rgb8) -> bool {
        match code {
            10 => self.foreground = value,
            11 => self.background = value,
            12 => self.cursor = value,
            _ => return false,
        }
        true
    }
}

#[derive(Debug)]
pub struct OscEmulator {
    state: OscColorState,
    parser_state: ParserState,
}

impl OscEmulator {
    pub fn new(state: OscColorState) -> Self {
        Self {
            state,
            parser_state: ParserState::Ground,
        }
    }

    pub fn process_output(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();
        let state = &mut self.state;

        for byte in bytes.iter().copied() {
            match &mut self.parser_state {
                ParserState::Ground => {
                    if byte == ESC {
                        self.parser_state = ParserState::Esc;
                    } else if byte == 0x9d {
                        self.parser_state = ParserState::Osc {
                            buf: Vec::new(),
                            pending_esc: false,
                        };
                    }
                }
                ParserState::Esc => {
                    if byte == b']' {
                        self.parser_state = ParserState::Osc {
                            buf: Vec::new(),
                            pending_esc: false,
                        };
                    } else {
                        self.parser_state = ParserState::Ground;
                    }
                }
                ParserState::Osc { buf, pending_esc } => {
                    if *pending_esc {
                        *pending_esc = false;
                        if byte == b'\\' {
                            if let Some(response) = handle_command(state, buf, OscTerminator::St) {
                                responses.push(response);
                            }
                            self.parser_state = ParserState::Ground;
                        } else {
                            buf.push(ESC);
                            if byte == BEL {
                                if let Some(response) =
                                    handle_command(state, buf, OscTerminator::Bel)
                                {
                                    responses.push(response);
                                }
                                self.parser_state = ParserState::Ground;
                            } else if byte == ESC {
                                *pending_esc = true;
                            } else {
                                buf.push(byte);
                            }
                        }
                    } else if byte == BEL {
                        if let Some(response) = handle_command(state, buf, OscTerminator::Bel) {
                            responses.push(response);
                        }
                        self.parser_state = ParserState::Ground;
                    } else if byte == ESC {
                        *pending_esc = true;
                    } else {
                        buf.push(byte);
                    }
                }
            }
        }

        responses
    }
}

fn handle_command(
    state: &mut OscColorState,
    buf: &[u8],
    terminator: OscTerminator,
) -> Option<Vec<u8>> {
    let command = std::str::from_utf8(buf).ok()?;
    let (code_str, payload) = command.split_once(';')?;
    let code = code_str.parse::<u8>().ok()?;

    if !matches!(code, 10 | 11 | 12) {
        return None;
    }

    if payload.trim() == "?" {
        let color = state.get(code)?;
        return Some(encode_query_response(code, color, terminator));
    }

    let parsed = parse_color(payload.trim())?;
    let _ = state.set(code, parsed);
    None
}

fn encode_query_response(code: u8, color: Rgb8, terminator: OscTerminator) -> Vec<u8> {
    let r = u16::from(color.r) * 257;
    let g = u16::from(color.g) * 257;
    let b = u16::from(color.b) * 257;

    let mut out = format!("\u{1b}]{};rgb:{r:04x}/{g:04x}/{b:04x}", code).into_bytes();
    match terminator {
        OscTerminator::Bel => out.push(BEL),
        OscTerminator::St => out.extend_from_slice(b"\x1b\\"),
    }
    out
}

fn parse_color(value: &str) -> Option<Rgb8> {
    if let Some(rest) = value.strip_prefix("rgb:") {
        return parse_rgb_spec(rest);
    }
    if let Some(rest) = value.strip_prefix('#') {
        return parse_hex_hash(rest);
    }
    None
}

fn parse_hex_hash(value: &str) -> Option<Rgb8> {
    if value.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&value[0..2], 16).ok()?;
    let g = u8::from_str_radix(&value[2..4], 16).ok()?;
    let b = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some(Rgb8 { r, g, b })
}

fn parse_rgb_spec(value: &str) -> Option<Rgb8> {
    let mut parts = value.split('/');
    let r = parse_rgb_component(parts.next()?)?;
    let g = parse_rgb_component(parts.next()?)?;
    let b = parse_rgb_component(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some(Rgb8 { r, g, b })
}

fn parse_rgb_component(value: &str) -> Option<u8> {
    if value.is_empty() || value.len() > 4 {
        return None;
    }
    let parsed = u16::from_str_radix(value, 16).ok()?;
    let max = (1u32 << (4 * value.len())) - 1;
    let scaled = ((u32::from(parsed) * 255) + (max / 2)) / max;
    Some(scaled as u8)
}

pub fn initial_color_state() -> OscColorState {
    OscColorState::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_11_bel_response_is_rgb_format() {
        let mut osc = OscEmulator::new(OscColorState {
            foreground: Rgb8 {
                r: 0x20,
                g: 0x30,
                b: 0x40,
            },
            background: Rgb8 {
                r: 0x2c,
                g: 0x2c,
                b: 0x2c,
            },
            cursor: Rgb8 {
                r: 0x20,
                g: 0x30,
                b: 0x40,
            },
        });

        let responses = osc.process_output(b"\x1b]11;?\x07");
        assert_eq!(responses.len(), 1);
        let response = std::str::from_utf8(&responses[0]).unwrap();
        assert_eq!(response, "\x1b]11;rgb:2c2c/2c2c/2c2c\x07");
    }

    #[test]
    fn query_10_st_response_uses_st_terminator() {
        let mut osc = OscEmulator::new(OscColorState {
            foreground: Rgb8 {
                r: 0x1a,
                g: 0x2b,
                b: 0x3c,
            },
            background: Rgb8 {
                r: 0x00,
                g: 0x00,
                b: 0x00,
            },
            cursor: Rgb8 {
                r: 0x1a,
                g: 0x2b,
                b: 0x3c,
            },
        });

        let responses = osc.process_output(b"\x1b]10;?\x1b\\");
        assert_eq!(responses.len(), 1);
        let response = std::str::from_utf8(&responses[0]).unwrap();
        assert_eq!(response, "\x1b]10;rgb:1a1a/2b2b/3c3c\x1b\\");
    }

    #[test]
    fn parser_handles_split_chunks() {
        let mut osc = OscEmulator::new(OscColorState::default());
        assert!(osc.process_output(b"\x1b]11;").is_empty());
        let responses = osc.process_output(b"?\x07");
        assert_eq!(responses.len(), 1);
    }

    #[test]
    fn set_command_updates_future_query() {
        let mut osc = OscEmulator::new(OscColorState::default());
        assert!(osc.process_output(b"\x1b]11;#2c2c2c\x07").is_empty());
        let responses = osc.process_output(b"\x1b]11;?\x07");
        let response = std::str::from_utf8(&responses[0]).unwrap();
        assert_eq!(response, "\x1b]11;rgb:2c2c/2c2c/2c2c\x07");
    }

    #[test]
    fn malformed_set_is_ignored() {
        let mut osc = OscEmulator::new(OscColorState::default());
        assert!(osc.process_output(b"\x1b]11;not-a-color\x07").is_empty());
        let responses = osc.process_output(b"\x1b]11;?\x07");
        let response = std::str::from_utf8(&responses[0]).unwrap();
        assert_eq!(response, "\x1b]11;rgb:0000/0000/0000\x07");
    }

    #[test]
    fn parses_multi_sequence_chunk() {
        let mut osc = OscEmulator::new(OscColorState::default());
        let responses = osc.process_output(b"\x1b]10;?\x07\x1b]11;?\x07");
        assert_eq!(responses.len(), 2);
    }

    #[test]
    fn parse_rgb_spec_short_and_long_components() {
        assert_eq!(
            parse_color("rgb:f/0/8"),
            Some(Rgb8 {
                r: 0xff,
                g: 0x00,
                b: 0x88
            })
        );
        assert_eq!(
            parse_color("rgb:ffff/7fff/0000"),
            Some(Rgb8 {
                r: 0xff,
                g: 0x7f,
                b: 0x00
            })
        );
    }
}
