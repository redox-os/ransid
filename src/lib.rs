#![crate_name="ransid"]
#![crate_type="lib"]
#![feature(alloc)]
#![feature(collections)]
#![no_std]

extern crate alloc;

#[macro_use]
extern crate collections;

use alloc::boxed::Box;

use collections::String;
use collections::Vec;

use core::{char, cmp};

pub use block::Block;
pub use color::Color;

pub mod block;
pub mod color;

pub struct Console {
    pub display: Box<[Block]>,
    pub changed: Box<[bool]>,
    pub x: usize,
    pub y: usize,
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
    pub escape_extra: bool,
    pub sequence: Vec<String>,
    pub raw_mode: bool,
}

impl Console {
    pub fn new(w: usize, h: usize) -> Console {
        Console {
            display: vec![Block::new(); w * h].into_boxed_slice(),
            changed: vec![true; h].into_boxed_slice(),
            x: 0,
            y: 0,
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
        }
    }

    fn block(&self, c: char) -> Block {
        Block {
            c: c,
            fg: if self.inverted { self.background } else { self.foreground },
            bg: if self.inverted { self.foreground } else { self.background },
            bold: self.bold,
            underlined: self.underlined
        }
    }

    pub fn change(&mut self, row: usize) {
        if let Some(mut c) = self.changed.get_mut(row) {
            *c = true;
        }
    }

    pub fn change_cursor(&mut self) {
        let row = self.y;
        self.change(row);
    }

    pub fn fix_cursor(&mut self) {
        let w = self.w;
        let h = self.h;

        if self.x >= w {
            self.x = 0;
            self.change_cursor();
            self.y += 1;
            self.change_cursor();
        }

        while self.y + 1 > h {
            for y in 1..h {
                for x in 0..w {
                    let c = self.display[y * w + x];
                    self.display[(y - 1) * w + x] = c;
                }
                self.change(y - 1);
            }
            let block = self.block(' ');
            for x in 0..w {
                self.display[(h - 1) * w + x] = block;
            }
            self.change(h - 1);
            self.y -= 1;
        }
    }

    pub fn code(&mut self, c: char) {
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
                'A' => {
                    self.change_cursor();
                    self.y -= cmp::min(self.y, self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(1));
                    self.change_cursor();
                    self.escape_sequence = false;
                },
                'B' => {
                    self.change_cursor();
                    self.y += cmp::min(self.h - 1 - self.y, self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(1));
                    self.change_cursor();
                    self.escape_sequence = false;
                },
                'C' => {
                    self.x += cmp::min(self.w - 1 - self.x, self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(1));
                    self.change_cursor();
                    self.escape_sequence = false;
                },
                'D' => {
                    self.x -= cmp::min(self.x, self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(1));
                    self.change_cursor();
                    self.escape_sequence = false;
                },
                'H' | 'f' => {
                    self.change_cursor();

                    let row = self.sequence.get(0).map_or("", |p| &p).parse::<isize>().unwrap_or(1);
                    self.y = cmp::max(0, row - 1) as usize;

                    let col = self.sequence.get(1).map_or("", |p| &p).parse::<isize>().unwrap_or(1);
                    self.x = cmp::max(0, col - 1) as usize;

                    self.change_cursor();

                    self.escape_sequence = false;
                },
                'J' => {
                    self.fix_cursor();

                    match self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(0) {
                        0 => {
                            let block = self.block(' ');
                            for c in self.display[self.y * self.w + self.x ..].iter_mut() {
                                *c = block;
                            }
                            for c in self.changed[self.y ..].iter_mut() {
                                *c = true;
                            }
                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        1 => {
                            let block = self.block(' ');
                            /* Should this add one? */
                            for c in self.display[.. self.y * self.w + self.x + 1].iter_mut() {
                                *c = block;
                            }
                            for c in self.changed[.. self.y + 1].iter_mut() {
                                *c = true;
                            }
                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        2 => {
                            // Erase all
                            self.x = 0;
                            self.y = 0;
                            let block = self.block(' ');
                            for c in self.display.iter_mut() {
                                *c = block;
                            }
                            for c in self.changed.iter_mut() {
                                *c = true;
                            }
                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        _ => {}
                    }

                    self.escape_sequence = false;
                },
                'K' => {
                    self.fix_cursor();

                    match self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(0) {
                        0 => {
                            let block = self.block(' ');
                            for c in self.display[self.y * self.w + self.x .. self.y * self.w + self.w].iter_mut() {
                                *c = block;
                            }
                            self.change_cursor();
                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        1 => {
                            let block = self.block(' ');
                            /* Should this add one? */
                            for c in self.display[self.y * self.w .. self.y * self.w + self.x + 1].iter_mut() {
                                *c = block;
                            }
                            self.change_cursor();
                            if ! self.raw_mode {
                                self.redraw = true;
                            }
                        },
                        2 => {
                            // Erase all
                            self.x = 0;
                            self.y = 0;
                            let block = self.block(' ');
                            for c in self.display[self.y * self.w .. self.y * self.w + self.w].iter_mut() {
                                *c = block;
                            }
                            self.change_cursor();
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
                        82 => self.raw_mode = true,
                        _ => ()
                    }

                    self.escape_sequence = false;
                },
                'l' if self.escape_extra => {
                    match self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(0) {
                        25 => self.cursor = false,
                        82 => self.raw_mode = false,
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
        } else {
            match c {
                '[' => {
                    // Control sequence initiator

                    self.escape_sequence = true;
                    self.sequence.push(String::new());
                },
                'c' => {
                    // Reset
                    self.x = 0;
                    self.y = 0;
                    self.raw_mode = false;
                    self.foreground = Color::ansi(7);
                    self.background = Color::ansi(0);
                    self.bold = false;
                    self.inverted = false;
                    self.underlined = false;
                    let block = self.block(' ');
                    for c in self.display.iter_mut() {
                        *c = block;
                    }
                    for c in self.changed.iter_mut() {
                        *c = true;
                    }
                    self.redraw = true;

                    self.escape = false;
                }
                _ => self.escape = false,
            }
        }
    }

    pub fn character(&mut self, c: char) {
        self.fix_cursor();

        match c {
            '\0' => {},
            '\x1B' => self.escape = true,
            '\n' => {
                self.change_cursor();
                self.x = 0;
                self.y += 1;
                if ! self.raw_mode {
                    self.redraw = true;
                }
                self.change_cursor();
            },
            '\t' => {
                self.x = ((self.x / 8) + 1) * 8;
                self.change_cursor();
            },
            '\r' => {
                self.x = 0;
                self.change_cursor();
            },
            '\x08' => {
                if self.x >= 1 {
                    self.x -= 1;

                    if ! self.raw_mode {
                        self.display[self.y * self.w + self.x] = self.block(' ');
                    }
                    self.change_cursor();
                }
            },
            ' ' => {
                self.display[self.y * self.w + self.x] = self.block(' ');
                self.change_cursor();

                self.x += 1;
            },
            _ => {
                self.display[self.y * self.w + self.x] = self.block(c);
                self.change_cursor();

                self.x += 1;
            }
        }
    }

    pub fn write(&mut self, bytes: &[u8]) {
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
                    self.code(c);
                } else {
                    self.character(c);
                }
            }
        };
    }
}
