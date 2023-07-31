/// Removes ANSI control characters from a single line of text ('\n' is ignored).
pub fn strip_ansi(input: String) -> String {
    let mut parser = vte::Parser::new();
    let mut performer = AnsiStripper::default();

    for b in input.bytes() {
        parser.advance(&mut performer, b);
    }

    performer.buf
}

#[derive(Debug, Default)]
struct AnsiStripper {
    buf: String,
}

impl vte::Perform for AnsiStripper {
    fn print(&mut self, c: char) {
        self.buf.push(c);
    }
}
