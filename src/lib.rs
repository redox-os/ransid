#![crate_name="ransid"]
#![crate_type="lib"]

extern crate vte;

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
    Move {
        from_x: usize,
        from_y: usize,
        to_x: usize,
        to_y: usize,
        w: usize,
        h: usize,
    },
    Resize {
        w: usize,
        h: usize,
    },
    Title {
        title: String
    }
}

pub struct State {
    pub x: usize,
    pub y: usize,
    pub save_x: usize,
    pub save_y: usize,
    pub w: usize,
    pub h: usize,
    pub top_margin: usize,
    pub bottom_margin: usize,
    pub g0: char,
    pub g1: char,
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub inverted: bool,
    pub underlined: bool,
    pub cursor: bool,
    pub redraw: bool,
    pub origin: bool,
    pub autowrap: bool,
    pub mouse_vt200: bool,
    pub mouse_btn: bool,
    pub mouse_sgr: bool,
    pub mouse_rxvt: bool,
}

impl State {
    pub fn new(w: usize, h: usize) -> State {
        State {
            x: 0,
            y: 0,
            save_x: 0,
            save_y: 0,
            w: w,
            h: h,
            top_margin: 0,
            bottom_margin: cmp::max(0, h as isize - 1) as usize,
            g0: 'B',
            g1: '0',
            foreground: Color::Ansi(7),
            background: Color::Ansi(0),
            bold: false,
            inverted: false,
            underlined: false,
            cursor: true,
            redraw: true,
            origin: false,
            autowrap: true,
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
        //TODO: Use min and max to ensure correct behavior
        callback(Event::Move {
            from_x: 0,
            from_y: self.top_margin + rows,
            to_x: 0,
            to_y: self.top_margin,
            w: self.w,
            h: (self.bottom_margin + 1) - rows,
        });
        callback(Event::Rect {
            x: 0,
            y: (self.bottom_margin + 1) - rows,
            w: self.w,
            h: rows,
            color: self.background,
        });
    }

    fn reverse_scroll<F: FnMut(Event)>(&self, rows: usize, callback: &mut F) {
        //TODO: Use min and max to ensure correct behavior
        callback(Event::Move {
            from_x: 0,
            from_y: self.top_margin,
            to_x: 0,
            to_y: self.top_margin + rows,
            w: self.w,
            h: (self.bottom_margin + 1) - rows,
        });
        callback(Event::Rect {
            x: 0,
            y: self.top_margin,
            w: self.w,
            h: rows,
            color: self.background,
        });
    }

    fn fix_cursor<F: FnMut(Event)>(&mut self, callback: &mut F) {
        let w = self.w;
        let h = self.h;

        if self.x >= w {
            if self.autowrap {
                println!("Autowrap");
                self.x = 0;
                self.y += 1;
            } else {
                self.x = w.checked_sub(1).unwrap_or(0);
            }
        }

        if self.y + 1 > h {
            let rows = self.y + 1 - h;
            self.scroll(rows, callback);
            self.y = self.y.checked_sub(rows).unwrap_or(0);
        }
    }

    /*
    pub fn code<F: FnMut(Event)>(&mut self, c: char, callback: &mut F) {
        if self.escape_sequence {
            match c {
                ';' => {
                    // Split sequence into list
                    self.sequence.push(String::new());
                },
                '?' => self.escape_extra = true,
                '@' ... '~' => {
                    println!("Unknown escape_sequence {:?} {:?}", self.sequence, c);
                    self.escape_sequence = false
                },
                _ => {
                    // Add a number to the sequence list
                    if let Some(value) = self.sequence.last_mut() {
                        value.push(c);
                    }
                },
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
                        _ => {
                            println!("Unknown OS command {:?}", self.sequence);
                        }
                    }

                    self.escape_os = false;
                },
                _ => {
                    // Add a character to the sequence list
                    if let Some(value) = self.sequence.last_mut() {
                        value.push(c);
                    }
                },
            }

            if !self.escape_os {
                self.sequence.clear();
                self.escape = false;
                self.escape_extra = false;
            }
        }
    }
    */

    pub fn print<F: FnMut(Event)>(&mut self, c: char, callback: &mut F) {
        self.fix_cursor(callback);
        self.block(c, callback);
        self.x += 1;
    }

    pub fn execute<F: FnMut(Event)>(&mut self, c: char, _callback: &mut F) {
        match c {
            //'\x07' => {}, // FIXME: Add bell
            '\x08' => { // Backspace
                if self.x >= 1 {
                    self.x -= 1;
                }
            },
            '\x09' => { // Tab
                self.x = ((self.x / 8) + 1) * 8;
            },
            '\x0A' => { // Newline
                self.x = 0;
                self.y += 1;
            },
            '\x0D' => { // Carriage Return
                self.x = 0;
            },
            _ => {
                println!("Unknown execute {:?}", c);
            }
        }
    }

    pub fn csi<F: FnMut(Event)>(&mut self, c: char, params: &[i64], _intermediates: &[u8], callback: &mut F) {
        match c {
            'A' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                self.y -= cmp::min(self.y, cmp::max(1, param) as usize);
            },
            'B' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                self.y += cmp::min(self.h.checked_sub(self.y + 1).unwrap_or(0), cmp::max(1, param) as usize);
            },
            'C' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                self.x += cmp::min(self.w.checked_sub(self.x + 1).unwrap_or(0), cmp::max(1, param) as usize);
            },
            'D' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                self.x -= cmp::min(self.x, cmp::max(1, param) as usize);
            },
            'E' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                self.x = 0;
                self.y += cmp::min(self.h.checked_sub(self.y + 1).unwrap_or(0), cmp::max(1, param) as usize);
            },
            'F' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                self.x = 0;
                self.y -= cmp::min(self.y, cmp::max(1, param) as usize);
            },
            'G' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                let col = cmp::max(1, param);
                self.x = cmp::max(0, cmp::min(self.w as i64 - 1, col - 1)) as usize;
            },
            'H' | 'f' => {
                {
                    let param = params.get(0).map(|v| *v).unwrap_or(1);
                    let row = cmp::max(1, param);
                    self.y = cmp::max(0, cmp::min(self.h as i64 - 1, row - 1)) as usize;
                }

                {
                    let param = params.get(1).map(|v| *v).unwrap_or(1);
                    let col = cmp::max(1, param);
                    self.x = cmp::max(0, cmp::min(self.w as i64 - 1, col - 1)) as usize;
                }
            },
            'J' => {
                self.fix_cursor(callback);

                let param = params.get(0).map(|v| *v).unwrap_or(0);
                match param {
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
                    },
                    _ => {
                        println!("Unknown CSI {:?} param {:?}", c, param);
                    }
                }
            },
            'K' => {
                self.fix_cursor(callback);

                let param = params.get(0).map(|v| *v).unwrap_or(0);
                match param {
                    0 => {
                        // Clear current row from cursor
                        callback(Event::Rect {
                            x: self.x,
                            y: self.y,
                            w: self.w - self.x,
                            h: 1,
                            color: self.background
                        });
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
                    },
                    _ => {
                        println!("Unknown CSI {:?} param {:?}", c, param);
                    }
                }
            },
            'P' => {
                //TODO: Fix
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                let cols = cmp::max(0, cmp::min(self.w as i64 - 1, param)) as usize;
                //TODO: Use min and max to ensure correct behavior
                callback(Event::Move {
                    from_x: self.x + cols,
                    from_y: self.y,
                    to_x: self.x,
                    to_y: self.y,
                    w: self.w - cols,
                    h: 1,
                });
                callback(Event::Rect {
                    x: self.w - cols,
                    y: self.y,
                    w: cols,
                    h: 1,
                    color: self.background,
                });
            },
            'S' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);;
                self.scroll(cmp::max(0, param) as usize, callback);
            },
            'T' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                self.reverse_scroll(cmp::max(0, param) as usize, callback);
            },
            'c' => {
                let report = format!("\x1B[?6c");
                callback(Event::Input {
                    data: &report.into_bytes()
                });
            },
            'd' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                self.y = cmp::max(0, cmp::min(self.h as i64 - 1, param - 1)) as usize;
            },
            'h' => {
                //TODO: Check intermediate
                let param = params.get(0).map(|v| *v).unwrap_or(0);
                match param {
                    3 => {
                        self.x = 0;
                        self.y = 0;
                        self.top_margin = 0;
                        self.bottom_margin = self.h;

                        self.w = 132;
                        //Resize screen
                        callback(Event::Resize {
                            w: self.w,
                            h: self.h
                        });

                        // Clear screen
                        callback(Event::Rect {
                            x: 0,
                            y: 0,
                            w: self.w,
                            h: self.h,
                            color: self.background
                        });
                    },
                    6 => self.origin = true,
                    7 => self.autowrap = true,
                    25 => self.cursor = true,
                    47 => callback(Event::ScreenBuffer {
                        alternate: true,
                        clear: false,
                    }),
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
                    unknown => {
                        println!("Unknown CSI {:?} param {:?}", c, unknown);
                    }
                }
            },
            'l' => {
                //TODO: Check intermediate
                let param = params.get(0).map(|v| *v).unwrap_or(0);
                match param {
                    3 => {
                        self.x = 0;
                        self.y = 0;
                        self.top_margin = 0;
                        self.bottom_margin = self.h;

                        self.w = 80;
                        //Resize screen
                        callback(Event::Resize {
                            w: self.w,
                            h: self.h
                        });

                        // Clear screen
                        callback(Event::Rect {
                            x: 0,
                            y: 0,
                            w: self.w,
                            h: self.h,
                            color: self.background
                        });
                    },
                    6 => self.origin = false,
                    7 => self.autowrap = false,
                    25 => self.cursor = false,
                    47 => callback(Event::ScreenBuffer {
                        alternate: false,
                        clear: false,
                    }),
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
                    unknown => {
                        println!("Unknown CSI {:?} param {:?}", c, unknown);
                    }
                }
            },
            'm' => {
                // Display attributes
                let mut value_iter = params.iter();
                while let Some(value) = value_iter.next() {
                    match *value {
                        0 => {
                            self.foreground = Color::Ansi(7);
                            self.background = Color::Ansi(0);
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
                        30 ... 37 => self.foreground = Color::Ansi(*value as u8 - 30),
                        38 => match value_iter.next().map(|v| *v).unwrap_or(0) {
                            2 => {
                                //True color
                                let r = value_iter.next().map(|v| *v).unwrap_or(0);
                                let g = value_iter.next().map(|v| *v).unwrap_or(0);
                                let b = value_iter.next().map(|v| *v).unwrap_or(0);
                                self.foreground = Color::TrueColor(r as u8, g as u8, b as u8);
                            },
                            5 => {
                                //256 color
                                let color_value = value_iter.next().map(|v| *v).unwrap_or(0);
                                self.foreground = Color::Ansi(color_value as u8);
                            },
                            _ => {}
                        },
                        39 => {
                            self.foreground = Color::Ansi(7);
                        },
                        40 ... 47 => self.background = Color::Ansi(*value as u8 - 40),
                        48 => match value_iter.next().map(|v| *v).unwrap_or(0) {
                            2 => {
                                //True color
                                let r = value_iter.next().map(|v| *v).unwrap_or(0);
                                let g = value_iter.next().map(|v| *v).unwrap_or(0);
                                let b = value_iter.next().map(|v| *v).unwrap_or(0);
                                self.background = Color::TrueColor(r as u8, g as u8, b as u8);
                            },
                            5 => {
                                //256 color
                                let color_value = value_iter.next().map(|v| *v).unwrap_or(0);
                                self.background = Color::Ansi(color_value as u8);
                            },
                            _ => {}
                        },
                        49 => {
                            self.background = Color::Ansi(0);
                        },
                        _ => {
                            println!("Unknown CSI {:?} param {:?}", c, value);
                        },
                    }
                }
            },
            'n' => {
                let param = params.get(0).map(|v| *v).unwrap_or(0);
                match param {
                    6 => {
                        let report = format!("\x1B[{};{}R", self.y + 1, self.x + 1);
                        callback(Event::Input {
                            data: &report.into_bytes()
                        });
                    },
                    _ => {
                        println!("Unknown CSI {:?} param {:?}", c, param);
                    }
                }
            },
            'r' => {
                let top = params.get(0).map(|v| *v).unwrap_or(1);
                let bottom = params.get(1).map(|v| *v).unwrap_or(self.h as i64);
                self.top_margin = cmp::max(0, top as isize - 1) as usize;
                self.bottom_margin = cmp::max(self.top_margin as isize, cmp::min(self.h as isize - 1, bottom as isize - 1)) as usize;
            },
            's' => {
                self.save_x = self.x;
                self.save_y = self.y;
            },
            'u' => {
                self.x = self.save_x;
                self.y = self.save_y;
            },
            _ => {
                println!("Unknown CSI {:?}", c);
            }
        }
    }

    pub fn esc<F: FnMut(Event)>(&mut self, c: char, _params: &[i64], intermediates: &[u8], callback: &mut F) {
        match c {
            //'(' => {},
            //')' => {},
            'D' => {
                self.x = 0;
            },
            'E' => {
                self.y += 1;
            },
            'M' => {
                while self.y <= 0 {
                    self.reverse_scroll(1, callback);
                    self.y += 1;
                }
                self.y -= 1;
            },
            '7' => {
                // Save
                self.save_x = self.x;
                self.save_y = self.y;
            },
            '8' => {
                match intermediates.get(0).map(|v| *v as char) {
                    Some('#') => {
                        // Test pattern
                        for x in (self.w/2).checked_sub(30).unwrap_or(10)..(self.w/2).checked_add(30).unwrap_or(70) {
                            self.x = x;

                            self.y = 8;
                            self.block('E', callback);

                            self.y = 15;
                            self.block('E', callback);
                        }

                        for y in 9..15 {
                            self.y = y;

                            self.x = (self.w/2).checked_sub(30).unwrap_or(10);
                            self.block('E', callback);

                            self.x = (self.w/2).checked_add(29).unwrap_or(69);
                            self.block('E', callback);
                        }

                        self.x = 0;
                        self.y = 0;
                    },
                    Some(inter) => {
                        println!("Unknown ESC {:?} intermediate {:?}", c, inter);
                    },
                    None => {
                        // Restore
                        self.x = self.save_x;
                        self.y = self.save_y;
                    }
                }
            },
            'c' => {
                // Reset
                self.x = 0;
                self.y = 0;
                self.save_x = 0;
                self.save_y = 0;
                self.top_margin = 0;
                self.bottom_margin = cmp::max(0, self.h as isize - 1) as usize;
                self.cursor = true;
                self.g0 = 'B';
                self.g1 = '0';
                self.foreground = Color::Ansi(7);
                self.background = Color::Ansi(0);
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
            },
            _ => {
                println!("Unknown ESC {:?}", c);
            }
        }
    }
}

pub struct Performer<'a, F: FnMut(Event) + 'a> {
    state: &'a mut State,
    callback: &'a mut F,
}

impl<'a, F: FnMut(Event)> vte::Perform for Performer<'a, F> {
    fn print(&mut self, c: char) {
        //println!("[print] {:?}", c);
        self.state.print(c, self.callback);
    }

    fn execute(&mut self, byte: u8) {
        //println!("[execute] {:02x}", byte);
        self.state.execute(byte as char, self.callback);
    }

    fn hook(&mut self, params: &[i64], intermediates: &[u8], ignore: bool) {
        //println!("[hook] params={:?}, intermediates={:?}, ignore={:?}",
        //         params, intermediates, ignore);
    }

    fn put(&mut self, byte: u8) {
        //println!("[put] {:02x}", byte);
    }

    fn unhook(&mut self) {
        //println!("[unhook]");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]]) {
        println!("[osc_dispatch] params={:?}", params);
    }

    fn csi_dispatch(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, c: char) {
        //println!("[csi_dispatch] params={:?}, intermediates={:?}, ignore={:?}, char={:?}",
        //         params, intermediates, ignore, c);
        self.state.csi(c, params, intermediates, self.callback);
    }

    fn esc_dispatch(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, byte: u8) {
        //println!("[esc_dispatch] params={:?}, intermediates={:?}, ignore={:?}, byte={:02x}",
        //         params, intermediates, ignore, byte);
        self.state.esc(byte as char, params, intermediates, self.callback);
    }
}

pub struct Console {
    pub parser: vte::Parser,
    pub state: State,
}

impl Console {
    pub fn new(w: usize, h: usize) -> Console {
        Console {
            parser: vte::Parser::new(),
            state: State::new(w, h),
        }
    }

    pub fn write<F: FnMut(Event)>(&mut self, bytes: &[u8], mut callback: F) {
        for byte in bytes.iter() {
            self.parser.advance(&mut Performer {
                state: &mut self.state,
                callback: &mut callback,
            }, *byte);
            /*
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
                if self.escape && (c < '\x08' || c > '\x0D') {
                    self.code(c, &mut callback);
                } else {
                    self.character(c, &mut callback);
                }
            }
            */
        };
    }
}
