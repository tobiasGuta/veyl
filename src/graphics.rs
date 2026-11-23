//! Kitty graphics APC framing experiment.
//!
//! Veyl uses VTE to read from the PTY. VTE does not expose a raw output stream
//! suitable for feeding this parser. This module is deliberately test-only
//! until a supported interception/rendering design has been validated.
//! Do NOT advertise Kitty support or send positive query responses based on
//! this parser alone. A parser is not a graphics renderer.

const ESC: u8 = 0x1b;
const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Event {
    /// Bytes that must still reach the regular VTE terminal parser, unchanged.
    Terminal(Vec<u8>),
    /// The raw control-and-payload section of ESC _ G ... ESC \.
    /// Image decoding, transmission chunks, and placement are NOT implemented.
    KittyFrame(Vec<u8>),
    /// A frame exceeded the per-frame memory limit and was dropped.
    RejectedOversize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Text,
    Esc,
    ApcStart,
    Kitty,
    KittyEsc,
    Oversize,
    OversizeEsc,
    ForeignApc,
    ForeignApcEsc,
}

/// A bounded transport-level frame splitter for future PTY interception.
///
/// Neither invoking this splitter nor recognizing frames implies support for
/// the Kitty graphics protocol. In particular, no query should be answered
/// until GTK rendering, image storage and placement exist.
pub(super) struct KittyFrameSplitter {
    state: State,
    frame: Vec<u8>,
    max_frame_bytes: usize,
}

impl KittyFrameSplitter {
    pub(super) fn new() -> Self {
        Self::with_limit(MAX_FRAME_BYTES)
    }

    fn with_limit(max_frame_bytes: usize) -> Self {
        Self {
            state: State::Text,
            frame: Vec::new(),
            max_frame_bytes,
        }
    }

    pub(super) fn push(&mut self, bytes: &[u8]) -> Vec<Event> {
        let mut events = Vec::new();
        let mut terminal = Vec::new();

        for &byte in bytes {
            match self.state {
                State::Text => {
                    if byte == ESC {
                        self.state = State::Esc;
                    } else {
                        terminal.push(byte);
                    }
                }
                State::Esc => {
                    if byte == b'_' {
                        self.state = State::ApcStart;
                    } else {
                        terminal.push(ESC);
                        if byte != ESC {
                            terminal.push(byte);
                            self.state = State::Text;
                        }
                    }
                }
                State::ApcStart => {
                    if byte == b'G' {
                        self.frame.clear();
                        self.state = State::Kitty;
                    } else {
                        terminal.extend_from_slice(&[ESC, b'_', byte]);
                        self.state = if byte == ESC {
                            State::ForeignApcEsc
                        } else {
                            State::ForeignApc
                        };
                    }
                }
                State::Kitty => {
                    if byte == ESC {
                        self.state = State::KittyEsc;
                    } else {
                        self.append_or_discard(byte);
                    }
                }
                State::KittyEsc => {
                    if byte == b'\\' {
                        emit_text(&mut terminal, &mut events);
                        events.push(Event::KittyFrame(std::mem::take(&mut self.frame)));
                        self.state = State::Text;
                    } else {
                        self.append_or_discard(ESC);
                        if self.state == State::Kitty {
                            if byte == ESC {
                                self.state = State::KittyEsc;
                            } else {
                                self.append_or_discard(byte);
                            }
                        }
                    }
                }
                State::Oversize => {
                    if byte == ESC {
                        self.state = State::OversizeEsc;
                    }
                }
                State::OversizeEsc => {
                    if byte == b'\\' {
                        emit_text(&mut terminal, &mut events);
                        events.push(Event::RejectedOversize);
                        self.state = State::Text;
                    } else if byte != ESC {
                        self.state = State::Oversize;
                    }
                }
                State::ForeignApc => {
                    terminal.push(byte);
                    if byte == ESC {
                        self.state = State::ForeignApcEsc;
                    }
                }
                State::ForeignApcEsc => {
                    terminal.push(byte);
                    self.state = if byte == b'\\' {
                        State::Text
                    } else if byte == ESC {
                        State::ForeignApcEsc
                    } else {
                        State::ForeignApc
                    };
                }
            }
        }

        emit_text(&mut terminal, &mut events);
        events
    }

    /// End-of-stream: preserve ordinary incomplete escapes, drop incomplete
    /// graphics (which must not leak base64 into terminal text).
    pub(super) fn finish(&mut self) -> Vec<Event> {
        let tail = match self.state {
            State::Esc => vec![ESC],
            State::ApcStart => vec![ESC, b'_'],
            _ => Vec::new(),
        };
        self.frame.clear();
        self.state = State::Text;
        if tail.is_empty() {
            Vec::new()
        } else {
            vec![Event::Terminal(tail)]
        }
    }

    fn append_or_discard(&mut self, byte: u8) {
        if self.frame.len() >= self.max_frame_bytes {
            self.frame.clear();
            self.state = State::Oversize;
        } else {
            self.frame.push(byte);
            self.state = State::Kitty;
        }
    }
}

fn emit_text(terminal: &mut Vec<u8>, events: &mut Vec<Event>) {
    if !terminal.is_empty() {
        events.push(Event::Terminal(std::mem::take(terminal)));
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, KittyFrameSplitter, MAX_FRAME_BYTES};

    #[test]
    fn normal_text_is_transparent() {
        let mut splitter = KittyFrameSplitter::new();
        let input = b"hello\n\x1b[31mred\x1b[0m\n";
        assert_eq!(splitter.push(input), vec![Event::Terminal(input.to_vec())]);
        assert!(splitter.finish().is_empty());
    }

    #[test]
    fn extracts_one_graphics_frame_in_stream_order() {
        let mut splitter = KittyFrameSplitter::new();
        assert_eq!(
            splitter.push(b"left\x1b_Ga=T,f=100;AA==\x1b\\right"),
            vec![
                Event::Terminal(b"left".to_vec()),
                Event::KittyFrame(b"a=T,f=100;AA==".to_vec()),
                Event::Terminal(b"right".to_vec())
            ]
        );
    }

    #[test]
    fn fragmented_control_sequence_and_payload() {
        let input = b"a\x1b_Gi=9,m=0;ZGF0YQ==\x1b\\b";
        let mut splitter = KittyFrameSplitter::new();
        let mut graphics = Vec::new();
        let mut text = Vec::new();
        for byte in input {
            for event in splitter.push(std::slice::from_ref(byte)) {
                match event {
                    Event::Terminal(part) => text.extend(part),
                    Event::KittyFrame(frame) => graphics.push(frame),
                    Event::RejectedOversize => panic!("unexpected oversize"),
                }
            }
        }
        assert_eq!(text, b"ab");
        assert_eq!(graphics, vec![b"i=9,m=0;ZGF0YQ==".to_vec()]);
    }

    #[test]
    fn preserves_foreign_apc_and_adjacent_terminal_escapes() {
        let mut splitter = KittyFrameSplitter::new();
        let input = b"\x1b_other\x1b\\\x1b[2J\x1b\x1b_Ga=q;\x1b\\";
        assert_eq!(
            splitter.push(input),
            vec![
                Event::Terminal(b"\x1b_other\x1b\\\x1b[2J\x1b".to_vec()),
                Event::KittyFrame(b"a=q;".to_vec()),
            ]
        );
    }

    #[test]
    fn rejects_oversized_frames_without_leaking_image_bytes() {
        let mut splitter = KittyFrameSplitter::with_limit(4);
        assert_eq!(
            splitter.push(b"ok\x1b_G12345\x1b\\done"),
            vec![
                Event::Terminal(b"ok".to_vec()),
                Event::RejectedOversize,
                Event::Terminal(b"done".to_vec())
            ]
        );
    }

    #[test]
    fn rejects_oversize_spanning_many_reads() {
        let mut splitter = KittyFrameSplitter::with_limit(2);
        assert!(splitter.push(b"\x1b_G1").is_empty());
        assert!(splitter.push(b"234567\x1b").is_empty());
        assert_eq!(
            splitter.push(b"\\after"),
            vec![
                Event::RejectedOversize,
                Event::Terminal(b"after".to_vec())
            ]
        );
    }

    #[test]
    fn drops_truncated_frames_and_preserves_incomplete_plain_escapes() {
        let mut splitter = KittyFrameSplitter::new();
        assert!(splitter.push(b"\x1b_Gi=1;AA==").is_empty());
        assert!(splitter.finish().is_empty());
        assert_eq!(splitter.push(b"\x1b"), vec![]);
        assert_eq!(splitter.finish(), vec![Event::Terminal(vec![0x1b])]);
    }

    #[test]
    fn default_memory_limit_is_bounded() {
        assert_eq!(MAX_FRAME_BYTES, 4 * 1024 * 1024);
    }
}
