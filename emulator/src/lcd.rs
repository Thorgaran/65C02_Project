use std::thread::{self, JoinHandle};
use std::sync::mpsc::{self, Sender, Receiver, TryRecvError};
use crate::{ToGuiMessage, logger::LogMessage};

const FONT_TABLE: [char; 256] = [
    ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
    ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
    ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
    '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
    'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '¥', ']', '^', '_',
    '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
    'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '→', '←',
    ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
    ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
    ' ', '｡', '｢', '｣', '､', '･', 'ｦ', 'ｧ', 'ｨ', 'ｩ', 'ｪ', 'ｫ', 'ｬ', 'ｭ', 'ｮ', 'ｯ',
    'ｰ', 'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ｹ', 'ｺ', 'ｻ', 'ｼ', 'ｽ', 'ｾ', 'ｿ',
    'ﾀ', 'ﾁ', 'ﾂ', 'ﾃ', 'ﾄ', 'ﾅ', 'ﾆ', 'ﾇ', 'ﾈ', 'ﾉ', 'ﾊ', 'ﾋ', 'ﾌ', 'ﾍ', 'ﾎ', 'ﾏ',
    'ﾐ', 'ﾑ', 'ﾒ', 'ﾓ', 'ﾔ', 'ﾕ', 'ﾖ', 'ﾗ', 'ﾘ', 'ﾙ', 'ﾚ', 'ﾛ', 'ﾜ', 'ﾝ', 'ﾞ', 'ﾟ',
    'α', 'ä', 'β', 'ε', 'μ', 'σ', 'ρ', 'g', '√', '¹', 'j', 'ˣ', '¢', 'Ⱡ', 'ñ', 'ö',
    'p', 'q', 'θ', '∞', 'Ω', 'ü', 'Σ', 'π', '𝔵', 'y', '千', '万', '円', '÷', ' ', '█',
];

pub enum CpuToLcdMessage {
    PinChange(LCDPins),
    Exit,
}

pub struct LCDPins {
    pub rs: bool,
    pub rw: bool,
    pub data: u8,
}

// enum ConfigBit { ValueIfHigh, ValueIfLow }
#[derive(PartialEq)]
enum DataLength { Eigth, Four }
enum NbLines { Two, One }
enum Font { FiveByTen, FiveByEight }
enum DisplayState { On, Off }
#[derive(PartialEq)]
enum CursorState { On, Off }
enum ShiftDir { Right, Left }
#[derive(PartialEq)]
enum DisplayBehavior { ShiftDisplay, MoveCursor }

struct LCDConfig {
    data_length: DataLength,
    nb_lines: NbLines,
    font: Font,
    display_state: DisplayState,
    cursor_state: CursorState,
    blinking: bool,
    shift_dir: ShiftDir,
    display_behavior: DisplayBehavior,
}

enum AddrCounter {
    Ddram,
    Cgram
}

pub struct LCD {
    pins: LCDPins,
    screen: String,
    display_addr: u8,
    addr_counter: AddrCounter,
    ddram_data: [u8; 0x80],
    ddram_addr: u8,
    config: LCDConfig,
    waiting_for_more_data: bool,
    tx_log_msgs: Sender<LogMessage>,
    tx_to_gui: Sender<ToGuiMessage>,
}

impl LCD {
    pub fn new(
        tx_log_msgs: Sender<LogMessage>,
        tx_to_gui: Sender<ToGuiMessage>
    ) -> LCD {
        let mut lcd = LCD {
            pins: LCDPins {
                rs: false,
                rw: false,
                data: 0b0000_0000,
            },
            screen: String::new(),
            display_addr: 0x0,
            addr_counter: AddrCounter::Ddram,
            ddram_data: [0xff; 0x80],
            ddram_addr: 0x0,
            config: LCDConfig {
                data_length: DataLength::Eigth,
                nb_lines: NbLines::One,
                font: Font::FiveByEight,
                display_state: DisplayState::On,
                cursor_state: CursorState::Off,
                blinking: false,
                shift_dir: ShiftDir::Right,
                display_behavior: DisplayBehavior::MoveCursor,
            },
            waiting_for_more_data: false,
            tx_log_msgs,
            tx_to_gui,
        };
        lcd.update_screen();
        lcd
    }

    pub fn run(mut self, rx_cpu_msgs: Receiver<CpuToLcdMessage>) -> JoinHandle<()> {
        thread::Builder::new().name("LCD thread".to_string()).spawn(move || {
            let (tx_timer_msgs, rx_timer_msgs) = mpsc::channel();
            let timer = timer::MessageTimer::new(tx_timer_msgs);

            let _guard = timer.schedule_repeating(chrono::Duration::microseconds(4), ());

            'lcd_thread_main: loop {
                rx_timer_msgs.recv().expect("Error receiving LCD clock pulse");

                'read_cpu_msgs: loop { match rx_cpu_msgs.try_recv() {
                    Err(TryRecvError::Disconnected) => panic!("CPU thread has hung up"),
                    Err(TryRecvError::Empty) => break 'read_cpu_msgs,
                    Ok(CpuToLcdMessage::PinChange(lcd_pins)) => {
                        if self.waiting_for_more_data {
                            self.pins = lcd_pins;
                            self.waiting_for_more_data = false;
                        } else {
                            if self.config.data_length == DataLength::Four {
                                self.pins.data |= lcd_pins.data >> 4;
                                if self.pins.rs != lcd_pins.rs
                                    || self.pins.rw != lcd_pins.rw
                                {
                                    panic!("Control pins difference between the two data halves");
                                }
                                self.waiting_for_more_data = true;
                            } else {
                                self.pins = lcd_pins;
                            }
                            // println!("DATA: {:#010b}, LCD STATE: {:?}", self.pins.data, self);
                            self.read_pins();
                        }
                    },
                    Ok(CpuToLcdMessage::Exit) => break 'lcd_thread_main,
                }};
            }
        }).unwrap()
    }

    fn slice_to_string(slice: &[u8]) -> String {
        let mut string = String::new();

        for char_code in slice.iter() {
            string.push(FONT_TABLE[*char_code as usize]);
        }

        string
    }

    fn update_screen(&mut self) {
        let addr = self.display_addr as usize;
        let mut new_screen = String::from("╔════════════════╗\n║");
        let mut data_with_cursor = self.ddram_data;
        
        if data_with_cursor[self.ddram_addr as usize] == 0x20
            && self.config.cursor_state == CursorState::On 
        {
            data_with_cursor[self.ddram_addr as usize] = 0x5f
        }

        match self.config.nb_lines {
            NbLines::One => {
                if addr > 0x30 {
                    new_screen.push_str(&LCD::slice_to_string(
                        &data_with_cursor[addr..0x4f]
                    ));
                    new_screen.push_str(&LCD::slice_to_string(
                        &data_with_cursor[..addr-0x30]
                    ));
                    new_screen.push_str("║\n║                ");
                } else {
                    new_screen.push_str(&LCD::slice_to_string(
                        &data_with_cursor[addr..addr+0x10]
                    ));
                    new_screen.push_str("║\n║                ");
                }
            },
            NbLines::Two => {
                if addr > 0x18 {
                    new_screen.push_str(&LCD::slice_to_string(
                        &data_with_cursor[addr..0x27]
                    ));
                    new_screen.push_str(&LCD::slice_to_string(
                        &data_with_cursor[..addr-0x18]
                    ));
                    new_screen.push_str("║\n║");
                    new_screen.push_str(&LCD::slice_to_string(
                        &data_with_cursor[addr+0x40..]
                    ));
                    new_screen.push_str(&LCD::slice_to_string(
                        &data_with_cursor[0x40..addr+0x28]
                    ));
                } else {
                    new_screen.push_str(&LCD::slice_to_string(
                        &data_with_cursor[addr..addr+0x10]
                    ));
                    new_screen.push_str("║\n║");
                    new_screen.push_str(&LCD::slice_to_string(
                        &data_with_cursor[addr+0x40..addr+0x50]
                    ));
                }
            },
        }

        new_screen.push_str("║\n╚════════════════╝");
    
        self.screen = new_screen;
        log!(self.tx_log_msgs, "\n{}", self.screen);
        self.tx_to_gui.send(ToGuiMessage::LcdScreen(self.screen.clone()))
            .expect("GUI thread has hung up");
    }
    
    fn change_ddram_addr(&mut self, new_ddram_addr: u8) {
        match self.config.nb_lines {
            NbLines::One => self.ddram_addr = new_ddram_addr % 0x4f,
            NbLines::Two => self.ddram_addr = match new_ddram_addr % 0x67 {
                0x28 => 0x40,
                0x3f => 0x27,
                _ => new_ddram_addr % 0x67,
            },
        }
    }

    fn change_display_addr(&mut self, new_display_addr: u8) {
        self.display_addr = new_display_addr % match self.config.nb_lines {
            NbLines::One => 0x4f,
            NbLines::Two => 0x27,
        }
    }

    fn read_pins(&mut self) {
        match (self.pins.rs, self.pins.rw) {
            // Instruction register write
            (false, false) => match self.pins.data.leading_zeros() {
                // Not in the datasheet (to test on real hardware?)
                8 => panic!("Unknown behavior for instruction 0b0000_0000!"),
                // Clear display
                7 => {
                    self.ddram_data = [0x20; 0x80];
                    self.addr_counter = AddrCounter::Ddram;
                    self.ddram_addr = 0x0;
                    self.display_addr = 0x0;
                    self.config.shift_dir = ShiftDir::Right;
                    self.update_screen();
                },
                // Return Home
                6 => {
                    self.addr_counter = AddrCounter::Ddram;
                    self.ddram_addr = 0x0;
                    self.display_addr = 0x0;
                    self.update_screen();
                },
                // Entry mode set
                5 => {
                    self.config.shift_dir = if self.pins.data & 0b0000_0010 == 0 {
                        ShiftDir::Left
                    } else {
                        ShiftDir::Right
                    };
                    self.config.display_behavior = if self.pins.data & 0b0000_0001 == 0 {
                        DisplayBehavior::MoveCursor
                    } else {
                        DisplayBehavior::ShiftDisplay
                    };
                },
                // Display on/off control
                4 => {
                    self.config.display_state = if self.pins.data & 0b0000_0100 == 0 {
                        DisplayState::Off
                    } else {
                        DisplayState::On
                    };
                    self.config.cursor_state = if self.pins.data & 0b0000_0010 == 0 {
                        CursorState::Off
                    } else {
                        CursorState::On
                    };
                    self.config.blinking = if self.pins.data & 0b0000_0001 == 0 {
                        false
                    } else {
                        true
                    };
                    self.update_screen();
                },
                // Cursor or display shift
                3 => match (self.pins.data & 0b0000_1000, self.pins.data & 0b0000_0100) {
                    // S/C=0, R/L=0
                    (0, 0) => self.change_ddram_addr(self.ddram_addr - 1),
                    // S/C=0, R/L=1
                    (0, _) => self.change_ddram_addr(self.ddram_addr + 1),
                    // S/C=1, R/L=0
                    (_, 0) => self.change_display_addr(self.display_addr - 1),
                    // S/C=1, R/L=1
                    (_, _) => self.change_display_addr(self.display_addr + 1),
                },
                // Function set
                2 => {
                    let mut fully_valid_data = true;
                    self.config.data_length = if self.pins.data & 0b0001_0000 == 0 {
                        self.waiting_for_more_data = true;
                        if self.config.data_length == DataLength::Eigth {
                            fully_valid_data = false;
                        }
                        DataLength::Four
                    } else {
                        DataLength::Eigth
                    };
                    if fully_valid_data {
                        self.config.nb_lines = if self.pins.data & 0b0000_1000 == 0 {
                            NbLines::One
                        } else {
                            NbLines::Two
                        };
                        self.config.font = if self.pins.data & 0b0000_0100 == 0 {
                            Font::FiveByEight
                        } else {
                            Font::FiveByTen
                        };
                        self.ddram_data = [0x20; 0x80];
                        self.update_screen();
                    }
                },
                // Set CGRAM address
                1 => {
                    self.addr_counter = AddrCounter::Cgram;
                    todo!();
                    // self.cgram_addr = self.pins.data & 0b0011_1111;
                },
                // Set DDRAM address
                0 => {
                    self.addr_counter = AddrCounter::Ddram;
                    self.ddram_addr = self.pins.data & 0b0111_1111;
                    match self.config.nb_lines {
                        NbLines::One => if let 0x50..=0xff = self.ddram_addr {
                            panic!("Illegal DDRAM address: {}", self.ddram_addr);
                        },
                        NbLines::Two => if let 0x28..=0x3f | 0x68..=0xff = self.ddram_addr {
                            panic!("Illegal DDRAM address: {}", self.ddram_addr);
                        },
                    };
                    self.update_screen();
                },
                _ => unreachable!(),
            },
            // Read busy flag (DB7) and address counter (DB0-DB6)
            (false, true) => todo!(),
            // Write to DDRAM or CGRAM
            (true, false) => {
                match (&self.addr_counter, &self.config.shift_dir) {
                    (&AddrCounter::Ddram, &ShiftDir::Right) => {
                        self.ddram_data[self.ddram_addr as usize] = self.pins.data;
                        self.change_ddram_addr(self.ddram_addr + 1);
                        if self.config.display_behavior == DisplayBehavior::ShiftDisplay {
                            self.change_display_addr(self.display_addr + 1);
                        }
                        self.update_screen();
                    },
                    (&AddrCounter::Ddram, &ShiftDir::Left) => {
                        self.ddram_data[self.ddram_addr as usize] = self.pins.data;
                        self.change_ddram_addr(self.ddram_addr - 1);
                        if self.config.display_behavior == DisplayBehavior::ShiftDisplay {
                            self.change_display_addr(self.display_addr - 1);
                        }
                        self.update_screen();
                    },
                    (&AddrCounter::Cgram, _) => todo!(),
                }
            },
            // Read DDRAM or CGRAM
            (true, true) => todo!(),
        }
    }
}
