use super::Color;

#[derive(Copy, Clone)]
pub struct Block {
    pub c: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub underlined: bool
}

impl Block {
    pub fn new() -> Self {
        Block {
            c: ' ',
            fg: Color::ansi(7),
            bg: Color::ansi(0),
            bold: false,
            underlined: false
        }
    }
}
