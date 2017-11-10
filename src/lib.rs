#![crate_name="ransid"]
#![crate_type="lib"]

extern crate vte;

use std::{char, cmp, str};

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
        let h = cmp::min(self.h, self.bottom_margin + 1);

        if self.x >= w {
            if self.autowrap {
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

    pub fn print<F: FnMut(Event)>(&mut self, c: char, callback: &mut F) {
        self.fix_cursor(callback);
        self.block(c, callback);
        self.x += 1;
    }

    pub fn execute<F: FnMut(Event)>(&mut self, c: char, _callback: &mut F) {
        // Fix for vt100 wrapping behavior: http://invisible-island.net/xterm/xterm.faq.html#vt100_wrapping
        //let xenl = self.x + 1 >= self.w;
        let xenl = false;

        match c {
            //'\x07' => {}, // FIXME: Add bell
            '\x08' => { // Backspace
                self.x = cmp::max(0, self.x as i64 - 1) as usize;
            },
            '\x09' => if ! xenl { // Tab
                self.x = cmp::max(0, cmp::min(self.w as i64 - 1, ((self.x as i64 / 8) + 1) * 8)) as usize;
            },
            '\x0A' => if ! xenl { // Newline
                self.x = 0;
                self.y += 1;
            },
            '\x0D' => if ! xenl { // Carriage Return
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
                if self.y < self.top_margin {
                    self.y = cmp::max(0, self.y as i64 - cmp::max(1, param)) as usize;
                } else {
                    self.y = cmp::max(self.top_margin as i64, self.y as i64 - cmp::max(1, param)) as usize;
                }
            },
            'B' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                if self.y > self.bottom_margin {
                    self.y = cmp::max(0, cmp::min(self.h as i64 - 1, self.y as i64 + cmp::max(1, param))) as usize;
                } else {
                    self.y = cmp::max(0, cmp::min(self.bottom_margin as i64, self.y as i64 + cmp::max(1, param))) as usize;
                }
            },
            'C' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                self.x = cmp::max(0, cmp::min(self.w as i64 - 1, self.x as i64 + cmp::max(1, param))) as usize;
            },
            'D' => {
                let param = params.get(0).map(|v| *v).unwrap_or(1);
                self.x = cmp::max(0, self.x as i64 - cmp::max(1, param)) as usize;
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

                    let (top, bottom) = if self.origin {
                        (self.top_margin, self.bottom_margin + 1)
                    } else {
                        (0, self.h)
                    };

                    self.y = cmp::max(0, cmp::min(bottom as i64 - 1, row + top as i64 - 1)) as usize;
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
                        self.bottom_margin = cmp::max(0, self.h as isize - 1) as usize;

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
                    6 => {
                        self.origin = true;
                        self.x = 0;
                        self.y = self.top_margin;
                    },
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
                        self.bottom_margin = cmp::max(0, self.h as isize - 1) as usize;

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
                    6 => {
                        self.origin = false;
                        self.x = 0;
                        self. y = 0;
                    },
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
                let mut value_iter = if params.len() == 0 {
                    [0].iter()
                } else {
                    params.iter()
                };
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
            'D' => {
                self.y += 1;
            },
            'E' => {
                self.x = 0;
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

    pub fn osc<F: FnMut(Event)>(&mut self, params: &[&[u8]], callback: &mut F) {
        match params.get(0).map(|s| s.get(0).map(|b| *b).unwrap_or(0)).unwrap_or(0) as char {
            '0' | '1' | '2' => if let Some(bytes) = params.get(1) {
                if let Ok(string) = str::from_utf8(bytes) {
                    callback(Event::Title {
                        title: string.to_string()
                    });
                } else {
                    println!("Invalid UTF-8 {:?}", bytes);
                }
            } else {
                println!("Unknown OSC {:?}", params);
            },
            _ => {
                println!("Unknown OSC {:?}", params);
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
        //println!("[print] {:?} at {}, {}", c, self.state.x, self.state.y);
        self.state.print(c, self.callback);
    }

    fn execute(&mut self, byte: u8) {
        //println!("[execute] {:02x} at {}, {}", byte, self.state.x, self.state.y);
        self.state.execute(byte as char, self.callback);
    }

    fn hook(&mut self, params: &[i64], intermediates: &[u8], ignore: bool) {
        //println!("[hook] params={:?}, intermediates={:?}, ignore={:?}", params, intermediates, ignore);
    }

    fn put(&mut self, byte: u8) {
        //println!("[put] {:02x}", byte);
    }

    fn unhook(&mut self) {
        //println!("[unhook]");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]]) {
        //println!("[osc] params={:?}", params);
        self.state.osc(params, self.callback);
    }

    fn csi_dispatch(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, c: char) {
        //println!("[csi] params={:?}, intermediates={:?}, ignore={:?}, char={:?} at {}, {}", params, intermediates, ignore, c, self.state.x, self.state.y);
        self.state.csi(c, params, intermediates, self.callback);
    }

    fn esc_dispatch(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, byte: u8) {
        //println!("[esc] params={:?}, intermediates={:?}, ignore={:?}, byte={:02x} at {}, {}", params, intermediates, ignore, byte, self.state.x, self.state.y);
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
        };
    }
}
