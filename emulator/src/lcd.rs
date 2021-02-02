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
#[derive(PartialEq)]
enum BlinkState { On, Off }
#[derive(Clone)]
enum ShiftDir { Right, Left }
#[derive(PartialEq, Clone)]
enum DisplayBehavior { Both, MoveCursor, ShiftDisplay }

struct LCDConfig {
    data_length: DataLength,
    nb_lines: NbLines,
    font: Font,
    display_state: DisplayState,
    cursor_state: CursorState,
    blink_state: BlinkState,
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
    current_blink_state: BlinkState,
    cycles_before_blink: usize,
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
            current_blink_state: BlinkState::Off,
            cycles_before_blink: 102_400,
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
                blink_state: BlinkState::Off,
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
                
                // Blinking mecanism
                if self.config.blink_state == BlinkState::On {
                    if self.cycles_before_blink == 0 {
                        self.cycles_before_blink = 102_400;
                        self.current_blink_state = match self.current_blink_state {
                            BlinkState::On => BlinkState::Off,
                            BlinkState::Off => BlinkState::On,
                        };
                        self.update_screen();
                    } else {
                        self.cycles_before_blink -= 1;
                    }
                }

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

    fn ddram_to_string(&self, start_addr: u8, end_addr: u8) -> String {
        let mut string = String::new();

        for (i, char_code) in self.ddram_data[(start_addr as usize)..(end_addr as usize)]
        .iter().enumerate() {
            if self.config.cursor_state == CursorState::On 
            && start_addr + i as u8 == self.ddram_addr {
                string.push_str("\u{200c}\u{0332}");
            }
            
            if self.current_blink_state == BlinkState::On 
            && start_addr + i as u8 == self.ddram_addr {
                string.push(FONT_TABLE[0xff]);
            } else {
                string.push(FONT_TABLE[*char_code as usize]);
            }
        }

        string
    }

    fn update_screen(&mut self) {
        let addr = self.display_addr;
        let mut new_screen = String::from("╔════════════════╗\n║");

        match self.config.nb_lines {
            NbLines::One => {
                if addr > 0x30 {
                    new_screen.push_str(&self.ddram_to_string(addr, 0x50));
                    new_screen.push_str(&self.ddram_to_string(0x00, addr-0x30));
                    new_screen.push_str("║\n║                ");
                } else {
                    new_screen.push_str(&self.ddram_to_string(addr, addr+0x10));
                    new_screen.push_str("║\n║                ");
                }
            },
            NbLines::Two => {
                if addr > 0x18 {
                    new_screen.push_str(&self.ddram_to_string(addr, 0x28));
                    new_screen.push_str(&self.ddram_to_string(0x00, addr-0x18));
                    new_screen.push_str("║\n║");
                    new_screen.push_str(&self.ddram_to_string(addr+0x40, 0x68));
                    new_screen.push_str(&self.ddram_to_string(0x40, addr+0x28));
                } else {
                    new_screen.push_str(&self.ddram_to_string(addr, addr+0x10));
                    new_screen.push_str("║\n║");
                    new_screen.push_str(&self.ddram_to_string(addr+0x40, addr+0x50));
                }
            },
        }

        new_screen.push_str("║\n╚════════════════╝");
    
        self.screen = new_screen;
        log!(self.tx_log_msgs, "\n{}", self.screen);
        self.tx_to_gui.send(ToGuiMessage::LcdScreen(self.screen.clone()))
            .expect("GUI thread has hung up");
    }
    
    fn cursor_display_shift(&mut self, shift_dir: ShiftDir, display_behavior: DisplayBehavior) {
        if display_behavior != DisplayBehavior::ShiftDisplay {
            match shift_dir {
                ShiftDir::Left => self.ddram_addr = self.ddram_addr.wrapping_sub(1),
                ShiftDir::Right => self.ddram_addr += 1,
            }
            match (&self.config.nb_lines, self.ddram_addr) {
                (&NbLines::One, 0xff) => self.ddram_addr = 0x4f,
                (&NbLines::One, 0x50) => self.ddram_addr = 0x00,
                (&NbLines::Two, 0xff) => self.ddram_addr = 0x67,
                (&NbLines::Two, 0x28) => self.ddram_addr = 0x40,
                (&NbLines::Two, 0x3f) => self.ddram_addr = 0x27,
                (&NbLines::Two, 0x68) => self.ddram_addr = 0x00,
                (_, _) => {},
            }
        }

        if display_behavior != DisplayBehavior::MoveCursor {
            match shift_dir {
                ShiftDir::Left => self.display_addr = self.display_addr.wrapping_sub(1),
                ShiftDir::Right => self.display_addr += 1,
            }
            match (&self.config.nb_lines, self.display_addr) {
                (&NbLines::One, 0xff) => self.display_addr = 0x4f,
                (&NbLines::One, 0x50) => self.display_addr = 0x00,
                (&NbLines::Two, 0xff) => self.display_addr = 0x27,
                (&NbLines::Two, 0x28) => self.display_addr = 0x00,
                (_, _) => {},
            }
        }

        self.update_screen();
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
                        DisplayBehavior::Both
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
                    self.config.blink_state = if self.pins.data & 0b0000_0001 == 0 {
                        self.current_blink_state = BlinkState::Off;
                        BlinkState::Off
                    } else {
                        self.cycles_before_blink = 102_400;
                        BlinkState::On
                    };
                    self.update_screen();
                },
                // Cursor or display shift
                3 => {
                    match (self.pins.data & 0b0000_1000, self.pins.data & 0b0000_0100) {
                        // S/C=0, R/L=0
                        (0, 0) => self.cursor_display_shift(ShiftDir::Left, DisplayBehavior::MoveCursor),
                        // S/C=0, R/L=1
                        (0, _) => self.cursor_display_shift(ShiftDir::Right, DisplayBehavior::MoveCursor),
                        // S/C=1, R/L=0
                        (_, 0) => self.cursor_display_shift(ShiftDir::Left, DisplayBehavior::ShiftDisplay),
                        // S/C=1, R/L=1
                        (_, _) => self.cursor_display_shift(ShiftDir::Right, DisplayBehavior::ShiftDisplay),
                    };
                    self.update_screen();
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
                match &self.addr_counter {
                    &AddrCounter::Ddram => {
                        self.ddram_data[self.ddram_addr as usize] = self.pins.data;
                        self.cursor_display_shift(self.config.shift_dir.clone(),
                            self.config.display_behavior.clone());
                    },
                    &AddrCounter::Cgram => todo!(),
                }
            },
            // Read DDRAM or CGRAM
            (true, true) => todo!(),
        }
    }
}
