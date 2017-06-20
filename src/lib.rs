#![crate_name="ransid"]
#![crate_type="lib"]

use std::{char, cmp};

pub use color::Color;

pub mod color;

pub enum Event<'a> {
    Char {
        x: usize,
        y: usize,
        c: char,
        bold: bool,
        underlined: bool,
        color: Color
    },
    Input {
        data: &'a [u8]
    },
    Rect {
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        color: Color
    },
    ScreenBuffer {
        alternate: bool,
        clear: bool,
    },
    Scroll {
        rows: usize,
        color: Color
    },
    Title {
        title: String
    }
}

pub struct Console {
    pub x: usize,
    pub y: usize,
    pub save_x: usize,
    pub save_y: usize,
    pub w: usize,
    pub h: usize,
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub inverted: bool,
    pub underlined: bool,
    pub cursor: bool,
    pub redraw: bool,
    pub utf_data: u32,
    pub utf_step: u32,
    pub escape: bool,
    pub escape_sequence: bool,
    pub escape_os: bool,
    pub escape_extra: bool,
    pub sequence: Vec<String>,
    pub raw_mode: bool,
    pub mouse_vt200: bool,
    pub mouse_btn: bool,
    pub mouse_sgr: bool,
    pub mouse_rxvt: bool,
}

impl Console {
    pub fn new(w: usize, h: usize) -> Console {
        Console {
            x: 0,
            y: 0,
            save_x: 0,
            save_y: 0,
            w: w,
            h: h,
            foreground: Color::ansi(7),
            background: Color::ansi(0),
            bold: false,
            inverted: false,
            underlined: false,
            cursor: true,
            redraw: true,
            utf_data: 0,
            utf_step: 0,
            escape: false,
            escape_sequence: false,
            escape_os: false,
            escape_extra: false,
            sequence: Vec::new(),

            /*
            @MANSTART{terminal-raw-mode}
            INTRODUCTION
                Since Redox has no ioctl syscall, it uses escape codes for switching to raw mode.

            ENTERING AND EXITING RAW MODE
                Entering raw mode is done using CSI-r (^[?82h). Unsetting raw mode is done by CSI-R (^[?82l).

            RAW MODE
                Raw mode means that the stdin must be handled solely by the program itself. It will not automatically be printed nor will it be modified in any way (modulo escape codes).

                This means that:
                    - stdin is not printed.
                    - newlines are interpreted as carriage returns in stdin.
                    - stdin is not buffered, meaning that the stream of bytes goes directly to the program, without the user having to press enter.
            @MANEND
            */
            raw_mode: false,
            mouse_vt200: false,
            mouse_btn: false,
            mouse_sgr: false,
            mouse_rxvt: false,
        }
    }

    fn block<F: FnMut(Event)>(&self, c: char, callback: &mut F) {
        callback(Event::Rect {
            x: self.x,
            y: self.y,
            w: 1,
            h: 1,
            color: if self.inverted { self.foreground } else { self.background }
        });
        callback(Event::Char {
            x: self.x,
            y: self.y,
            c: c,
            bold: self.bold,
            underlined: self.underlined,
            color: if self.inverted { self.background } else { self.foreground }
        });
    }

    fn scroll<F: FnMut(Event)>(&self, rows: usize, callback: &mut F) {
        callback(Event::Scroll {
            rows: rows,
            color: self.background
        });
    }

    fn fix_cursor<F: FnMut(Event)>(&mut self, callback: &mut F) {
        let w = self.w;
        let h = self.h;

        if self.x >= w {
            self.x = 0;
            self.y += 1;
        }

        if self.y + 1 > h {
            let rows = self.y + 1 - h;
            self.scroll(rows, callback);
            self.y -= rows;
        }
    }

    pub fn code<F: FnMut(Event)>(&mut self, c: char, callback: &mut F) {
        if self.escape_sequence {
            match c {
                '0' ... '9' => {
                    // Add a number to the sequence list
                    if let Some(mut value) = self.sequence.last_mut() {
                        value.push(c);
                    }
                },
                ';' => {
                    // Split sequence into list
                    self.sequence.push(String::new());
                },
                'm' => {
                    // Display attributes
                    let mut value_iter = self.sequence.iter();
                    while let Some(value_str) = value_iter.next() {
                        let value = value_str.parse::<u8>().unwrap_or(0);
                        match value {
                            0 => {
                                self.foreground = Color::ansi(7);
                                self.background = Color::ansi(0);
                                self.bold = false;
                                self.underlined = false;
                                self.inverted = false;
                            },
                            1 => {
                                self.bold = true;
                            },
                            4 => {
                                self.underlined = true;
                            },
                            7 => {
                                self.inverted = true;
                            },
                            21 => {
                                self.bold = false;
                            },
                            24 => {
                                self.underlined = false;
                            },
                            27 => {
                                self.inverted = false;
                            },
                            30 ... 37 => self.foreground = Color::ansi(value - 30),
                            38 => match value_iter.next().map_or("", |s| &s).parse::<usize>().unwrap_or(0) {
                                2 => {
                                    //True color
                                    let r = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    let g = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    let b = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    self.foreground = Color::new(r, g, b);
                                },
                                5 => {
                                    //256 color
                                    let color_value = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    self.foreground = Color::ansi(color_value);
                                },
                                _ => {}
                            },
                            39 => {
                                self.foreground = Color::ansi(7);
                            },
                            40 ... 47 => self.background = Color::ansi(value - 40),
                            48 => match value_iter.next().map_or("", |s| &s).parse::<usize>().unwrap_or(0) {
                                2 => {
                                    //True color
                                    let r = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    let g = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    let b = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    self.background = Color::new(r, g, b);
                                },
                                5 => {
                                    //256 color
                                    let color_value = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    self.background = Color::ansi(color_value);
                                },
                                _ => {}
                            },
                            49 => {
                                self.background = Color::ansi(0);
                            },
                            _ => {},
                        }
                    }

                    self.escape_sequence = false;
                },
                'n' => {
                    match self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(0) {
                        6 => {
                            let report = format!("\x1B[{};{}R", self.y + 1, self.x + 1);
                            callback(Event::Input {
                                data: &report.into_bytes()
                            });
                        },
                        _ => ()
                    }
                    self.escape_sequence = false;
                },
                's' => {
                    self.save_x = self.x;
                    self.save_y = self.y;
                    self.escape_sequence = false;
                },
                'u' => {
                    self.x = self.save_x;
                    self.y = self.save_y;
                    self.escape_sequence = false;
                },
                'A' => {
                    self.y -= cmp::min(self.y, self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(1));
                    self.escape_sequence = false;
                },
                'B' => {
                    self.y += cmp::min(self.h - 1 - self.y, self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(1));
                    self.escape_sequence = false;
                },
                'C' => {
                    self.x += cmp::min(self.w - 1 - self.x, self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(1));
                    self.escape_sequence = false;
                },
                'D' => {
                    self.x -= cmp::min(self.x, self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(1));
                    self.escape_sequence = false;
                },
                'H' | 'f' => {
                    let row = self.sequence.get(0).map_or("", |p| &p).parse::<isize>().unwrap_or(1);
                    self.y = cmp::max(0, cmp::min(self.h as isize - 1, row - 1)) as usize;

                    let col = self.sequence.get(1).map_or("", |p| &p).parse::<isize>().unwrap_or(1);
                    self.x = cmp::max(0, cmp::min(self.w as isize - 1, col - 1)) as usize;

                    self.escape_sequence = false;
                },
                'J' => {
                    self.fix_cursor(callback);

                    match self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(0) {
                        0 => {
                            // Clear current row from cursor
                            callback(Event::Rect {
                                x: self.x,
                                y: self.y,
                                w: self.w - self.x,
                                h: 1,
                                color: self.background
                            });

                            // Clear following rows
                            callback(Event::Rect {
                                x: 0,
                                y: self.y,
                                w: self.w,
                                h: self.h - self.y,
                                color: self.background
                            });

                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        1 => {
                            // Clear previous rows
                            callback(Event::Rect {
                                x: 0,
                                y: 0,
                                w: self.w,
                                h: self.y,
                                color: self.background
                            });

                            // Clear current row to cursor
                            callback(Event::Rect {
                                x: 0,
                                y: self.y,
                                w: self.x,
                                h: 1,
                                color: self.background
                            });

                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        2 => {
                            // Erase all
                            self.x = 0;
                            self.y = 0;

                            // Clear all rows
                            callback(Event::Rect {
                                x: 0,
                                y: 0,
                                w: self.w,
                                h: self.h,
                                color: self.background
                            });

                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        _ => {}
                    }

                    self.escape_sequence = false;
                },
                'K' => {
                    self.fix_cursor(callback);

                    match self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(0) {
                        0 => {
                            // Clear current row from cursor
                            callback(Event::Rect {
                                x: self.x,
                                y: self.y,
                                w: self.w - self.x,
                                h: 1,
                                color: self.background
                            });

                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        1 => {
                            // Clear current row to cursor
                            callback(Event::Rect {
                                x: 0,
                                y: self.y,
                                w: self.x,
                                h: 1,
                                color: self.background
                            });

                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        2 => {
                            // Erase row
                            callback(Event::Rect {
                                x: 0,
                                y: self.y,
                                w: self.w,
                                h: 1,
                                color: self.background
                            });

                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        _ => {}
                    }

                    self.escape_sequence = false;
                },
                '?' => self.escape_extra = true,
                'h' if self.escape_extra => {
                    match self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(0) {
                        25 => self.cursor = true,
                        47 => callback(Event::ScreenBuffer {
                            alternate: true,
                            clear: false,
                        }),
                        82 => self.raw_mode = true,
                        1000 => self.mouse_vt200 = true,
                        1002 => self.mouse_btn = true,
                        1006 => self.mouse_sgr = true,
                        1015 => self.mouse_rxvt = true,
                        1047 => callback(Event::ScreenBuffer {
                            alternate: true,
                            clear: false,
                        }),
                        1048 => {
                            self.save_x = self.x;
                            self.save_y = self.y;
                        },
                        1049 => {
                            self.save_x = self.x;
                            self.save_y = self.y;

                            callback(Event::ScreenBuffer {
                                alternate: true,
                                clear: true,
                            });
                        },
                        _ => ()
                    }

                    self.escape_sequence = false;
                },
                'l' if self.escape_extra => {
                    match self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(0) {
                        25 => self.cursor = false,
                        47 => callback(Event::ScreenBuffer {
                            alternate: false,
                            clear: false,
                        }),
                        82 => self.raw_mode = false,
                        1000 => self.mouse_vt200 = false,
                        1002 => self.mouse_btn = false,
                        1006 => self.mouse_sgr = false,
                        1015 => self.mouse_rxvt = false,
                        1047 => callback(Event::ScreenBuffer {
                            alternate: false,
                            clear: true
                        }),
                        1048 => {
                            self.x = self.save_x;
                            self.y = self.save_y;
                        },
                        1049 => {
                            self.x = self.save_x;
                            self.y = self.save_y;

                            callback(Event::ScreenBuffer {
                                alternate: false,
                                clear: false,
                            });
                        }
                        _ => ()
                    }

                    self.escape_sequence = false;
                },
                _ => self.escape_sequence = false,
            }

            if !self.escape_sequence {
                self.sequence.clear();
                self.escape = false;
                self.escape_extra = false;
            }
        } else if self.escape_os {
            match c {
                ';' => {
                    // Split sequence into list
                    self.sequence.push(String::new());
                },
                '\x07' => {
                    // Break on BEL
                    match self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(0) {
                        0 | 1 | 2 => {
                            // Set window title
                            let mut title = String::new();
                            for (i, seq) in self.sequence.iter().skip(1).enumerate() {
                                if i > 0 {
                                    title.push(';');
                                }
                                title.push_str(seq);
                            }

                            callback(Event::Title {
                                title: title
                            });
                        },
                        _ => ()
                    }

                    self.escape_os = false;
                },
                _ => {
                    // Add a character to the sequence list
                    if let Some(mut value) = self.sequence.last_mut() {
                        value.push(c);
                    }
                },
            }

            if !self.escape_os {
                self.sequence.clear();
                self.escape = false;
                self.escape_extra = false;
            }
        } else {
            match c {
                '[' => {
                    // Control sequence initiator

                    self.escape_sequence = true;
                    self.sequence.push(String::new());
                },
                ']' => {
                    // Operating system command

                    self.escape_os = true;
                    self.sequence.push(String::new());
                },
                '7' => {
                    // Save
                    self.save_x = self.x;
                    self.save_y = self.y;
                    self.escape = false;
                },
                '8' => {
                    self.x = self.save_x;
                    self.y = self.save_y;
                    self.escape = false;
                },
                'c' => {
                    // Reset
                    self.x = 0;
                    self.y = 0;
                    self.save_x = 0;
                    self.save_y = 0;
                    self.cursor = true;
                    self.raw_mode = false;
                    self.foreground = Color::ansi(7);
                    self.background = Color::ansi(0);
                    self.bold = false;
                    self.inverted = false;
                    self.underlined = false;

                    // Clear screen
                    callback(Event::Rect {
                        x: 0,
                        y: 0,
                        w: self.w,
                        h: self.h,
                        color: self.background
                    });

                    self.redraw = true;

                    self.escape = false;
                }
                _ => self.escape = false,
            }
        }
    }

    pub fn character<F: FnMut(Event)>(&mut self, c: char, callback: &mut F) {
        if c != '\x1B' && c != '\n' && c != '\r' {
            self.fix_cursor(callback);
        }

        match c {
            '\x00' ... '\x06' => {}, // Ignore
            '\x07' => {}, // FIXME: Add bell
            '\x08' => { // Backspace
                if self.x >= 1 {
                    self.x -= 1;

                    if ! self.raw_mode {
                        self.block(' ', callback);
                    }
                }
            },
            '\x09' => { // Tab
                self.x = ((self.x / 8) + 1) * 8;
            },
            '\x0A' => { // Newline
                self.x = 0;
                self.y += 1;
                if ! self.raw_mode {
                    self.redraw = true;
                }
            },
            '\x0B' ... '\x0C' => {} // Ignore
            '\x0D' => { // Carriage Return
                self.x = 0;
            },
            '\x0E' ... '\x1A' => {} // Ignore
            '\x1B' => { // Escape
                self.escape = true;
            },
            '\x1C' ... '\x1F' => {} // Ignore
            ' ' => { // Space
                self.block(' ', callback);

                self.x += 1;
            },
            _ => {
                self.block(c, callback);

                self.x += 1;
            }
        }
    }

    pub fn write<F: FnMut(Event)>(&mut self, bytes: &[u8], mut callback: F) {
        for byte in bytes.iter() {
            let c_opt = match *byte {
                //ASCII
                0b00000000 ... 0b01111111 => {
                    Some(*byte as char)
                },
                //Continuation byte
                0b10000000 ... 0b10111111 if self.utf_step > 0 => {
                    self.utf_step -= 1;
                    self.utf_data |= ((*byte as u32) & 0b111111) << (6 * self.utf_step);
                    if self.utf_step == 0 {
                        let data = self.utf_data;
                        self.utf_data = 0;
                        char::from_u32(data)
                    } else {
                        None
                    }
                },
                //Two byte lead
                0b11000000 ... 0b11011111 => {
                    self.utf_step = 1;
                    self.utf_data = ((*byte as u32) & 0b11111) << (6 * self.utf_step);
                    None
                },
                //Three byte lead
                0b11100000 ... 0b11101111 => {
                    self.utf_step = 2;
                    self.utf_data = ((*byte as u32) & 0b1111) << (6 * self.utf_step);
                    None
                },
                //Four byte lead
                0b11110000 ... 0b11110111 => {
                    self.utf_step = 3;
                    self.utf_data = ((*byte as u32) & 0b111) << (6 * self.utf_step);
                    None
                },
                //Invalid, use replacement character
                _ => {
                    char::from_u32(0xFFFD)
                }
            };

            if let Some(c) = c_opt {
                if self.escape {
                    self.code(c, &mut callback);
                } else {
                    self.character(c, &mut callback);
                }
            }
        };
    }
}
