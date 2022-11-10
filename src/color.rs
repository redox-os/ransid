/// A color
#[derive(Copy, Clone, Debug)]
pub enum Color {
    Ansi(u8),
    TrueColor(u8, u8, u8),
}

impl Color {
    pub fn as_rgb(&self) -> u32 {
        let encode_rgb = |r: u8, g: u8, b: u8| -> u32 {
            0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
        };

        match *self {
            Color::TrueColor(r, g, b) => encode_rgb(r, g, b),
            Color::Ansi(value) => match value {
                0 => encode_rgb(0x00, 0x00, 0x00),
                1 => encode_rgb(0x80, 0x00, 0x00),
                2 => encode_rgb(0x00, 0x80, 0x00),
                3 => encode_rgb(0x80, 0x80, 0x00),
                4 => encode_rgb(0x00, 0x00, 0x80),
                5 => encode_rgb(0x80, 0x00, 0x80),
                6 => encode_rgb(0x00, 0x80, 0x80),
                7 => encode_rgb(0xc0, 0xc0, 0xc0),
                8 => encode_rgb(0x80, 0x80, 0x80),
                9 => encode_rgb(0xff, 0x00, 0x00),
                10 => encode_rgb(0x00, 0xff, 0x00),
                11 => encode_rgb(0xff, 0xff, 0x00),
                12 => encode_rgb(0x00, 0x00, 0xff),
                13 => encode_rgb(0xff, 0x00, 0xff),
                14 => encode_rgb(0x00, 0xff, 0xff),
                15 => encode_rgb(0xff, 0xff, 0xff),
                16 ... 231 => {
                    let convert = |value: u8| -> u8 {
                        match value {
                            0 => 0,
                            _ => value * 0x28 + 0x28
                        }
                    };

                    let r = convert((value - 16) / 36 % 6);
                    let g = convert((value - 16) / 6 % 6);
                    let b = convert((value - 16) % 6);
                    encode_rgb(r, g, b)
                },
                232 ... 255 => {
                    let gray = (value - 232) * 10 + 8;
                    encode_rgb(gray, gray, gray)
                },
                _ => encode_rgb(0, 0, 0)
            }
        }
    }
}
