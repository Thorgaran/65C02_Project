use w65c02s::{System, W65C02S, State};
use std::{thread::{self, JoinHandle}, time};
use std::sync::mpsc::{self, Sender, Receiver, TryRecvError};
use std::cell::RefCell;
use crate::{Config, LogMessage, ToGuiMessage};
use crate::lcd::{LCD, SysToLcdMessage, LCDPins};

// Default waiting time between steps when running, in milliseconds
pub const DEFAULT_STEP_WAIT: usize = 50;

const OPCODES: [&str; 256] = [
    "BRK",     "ORA i_zp_x", "<invalid: NOP imm>", "<invalid: NOP>", "TSB zp",              "ORA zp",    "ASL zp",    "RMB0 zp", 
    "PHP",     "ORA imm",    "ASL",                "<invalid: NOP>", "TSB abs",             "ORA abs",   "ASL abs",   "BBR0 zp_rel",
    "BPL rel", "ORA i_zp_y", "ORA i_zp",           "<invalid: NOP>", "TRB zp",              "ORA zp_x",  "ASL zp_x",  "RMB1 zp", 
    "CLC",     "ORA abs_y",  "INC",                "<invalid: NOP>", "TRB abs",             "ORA abs_x", "ASL abs_x", "BBR1 zp_rel",
    "JSR abs", "AND i_zp_y", "<invalid: NOP imm>", "<invalid: NOP>", "BIT zp",              "AND zp",    "ROL zp",    "RMB2 zp", 
    "PLP",     "AND imm",    "ROL",                "<invalid: NOP>", "BIT abs",             "AND abs",   "ROL abs",   "BBR2 zp_rel",
    "BMI rel", "AND i_zp_y", "AND i_zp",           "<invalid: NOP>", "BIT zp_x",            "AND zp_x",  "ROL zp_x",  "RMB3 zp", 
    "SEC",     "AND abs_y",  "DEC",                "<invalid: NOP>", "BIT abs_x",           "AND abs_x", "ROL abs_x", "BBR3 zp_rel",
    "RTI",     "EOR i_zp_x", "<invalid: NOP imm>", "<invalid: NOP>", "<invalid: NOP zp>",   "EOR zp",    "LSR zp",    "RMB4 zp", 
    "PHA",     "EOR imm",    "LSR",                "<invalid: NOP>", "JMP abs",             "EOR abs",   "LSR abs",   "BBR4 zp_rel",
    "BVC rel", "EOR i_zp_y", "EOR i_zp",           "<invalid: NOP>", "<invalid: NOP zp_x>", "EOR zp_x",  "LSR zp_x",  "RMB5 zp", 
    "CLI",     "EOR abs_y",  "PHY",                "<invalid: NOP>", "<invalid: NOP abs>",  "EOR abs_x", "LSR abs_x", "BBR5 zp_rel",
    "RTS",     "ADC i_zp_x", "<invalid: NOP imm>", "<invalid: NOP>", "STZ zp",              "ADC zp",    "ROR zp",    "RMB6 zp", 
    "PLA",     "ADC imm",    "ROR",                "<invalid: NOP>", "JMP ind",             "ADC abs",   "ROR abs",   "BBR6 zp_rel",
    "BVS rel", "ADC i_zp_y", "ADC i_zp",           "<invalid: NOP>", "STZ zp_x",            "ADC zp_x",  "ROR zp_x",  "RMB7 zp", 
    "SEI",     "ADC abs_y",  "PLY",                "<invalid: NOP>", "JMP ind_x",           "ADC abs_x", "ROR abs_x", "BBR7 zp_rel",
    "BRA rel", "STA i_zp_x", "<invalid: NOP imm>", "<invalid: NOP>", "STY zp",              "STA zp",    "STX zp",    "SMB0 zp", 
    "DEY",     "BIT imm",    "TXA",                "<invalid: NOP>", "STY abs",             "STA abs",   "STX abs",   "BBS0 zp_rel",
    "BCC rel", "STA i_zp_y", "STA i_zp",           "<invalid: NOP>", "STY zp_x",            "STA zp_x",  "STX zp_y",  "SMB1 zp", 
    "TYA",     "STA abs_y",  "TXS",                "<invalid: NOP>", "STZ abs",             "STA abs_x", "STZ abs_x", "BBS1 zp_rel",
    "LDY imm", "LDA i_zp_x", "LDX imm",            "<invalid: NOP>", "LDY zp",              "LDA zp",    "LDX zp",    "SMB2 zp", 
    "TAY",     "LDA imm",    "TAX",                "<invalid: NOP>", "LDY abs",             "LDA abs",   "LDX abs",   "BBS2 zp_rel",
    "BCS rel", "LDA i_zp_y", "LDA i_zp",           "<invalid: NOP>", "LDY zp_x",            "LDA zp_x",  "LDX zp_y",  "SMB3 zp", 
    "CLV",     "LDA abs_y",  "TSX",                "<invalid: NOP>", "LDY abs_x",           "LDA abs_x", "LDX abs_y", "BBS3 zp_rel",
    "CPY imm", "CMP i_zp_x", "<invalid: NOP imm>", "<invalid: NOP>", "CPY zp",              "CMP zp",    "DEC zp",    "SMB4 zp", 
    "INY",     "CMP imm",    "DEX",                "WAI",            "CPY abs",             "CMP abs",   "DEC abs",   "BBS4 zp_rel",
    "BNE rel", "CMP i_zp_y", "CMP i_zp",           "<invalid: NOP>", "<invalid: NOP zp_x>", "CMP zp_x",  "DEC zp_x",  "SMB5 zp", 
    "CLD",     "CMP abs_y",  "PHX",                "STP",            "<invalid: NOP abs>",  "CMP abs_x", "DEC abs_x", "BBS5 zp_rel",
    "CPX imm", "SBC i_zp_x", "<invalid: NOP imm>", "<invalid: NOP>", "CPX zp",              "SBC zp",    "INC zp",    "SMB6 zp", 
    "INX",     "SBC imm",    "NOP",                "<invalid: NOP>", "CPX abs",             "SBC abs",   "INC abs",   "BBS6 zp_rel",
    "BEQ rel", "SBC i_zp_y", "SBC i_zp",           "<invalid: NOP>", "<invalid: NOP zp_x>", "SBC zp_x",  "INC zp_x",  "SMB7 zp", 
    "SED",     "SBC abs_y",  "PLX",                "<invalid: NOP>", "<invalid: NOP abs>",  "SBC abs_x", "INC abs_x", "BBS7 zp_rel",
];

pub enum ToSysMessage {
    Run,
    Stop,
    Step,
    ChangeWaitTime(usize),
    ShowLog(bool),
    Breakpoint(bool),
    Exit,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Data<T: Clone + Copy> {
    pub data: T,
    pub is_garbage: bool,
}

impl Data<u8> {
    fn new_garbage() -> Data<u8> {
        Data {
            data: rand::random(),
            is_garbage: true,
        }
    }
}

impl Data<u16> {
    fn new_garbage() -> Data<u16> {
        Data {
            data: rand::random(),
            is_garbage: true,
        }
    }
}

impl<T: Clone + Copy> Data<T> {
    fn read(&self, allow_garbage: bool, 
        tx_log_msgs: &Sender<LogMessage>,
        garbage_msg: &str,
    ) -> T {
        if self.is_garbage {
            if allow_garbage {
                log!(tx_log_msgs, garbage_msg);
                self.data
            }
            else {
                panic!(String::from(garbage_msg));
            }
        }
        else {
            self.data
        }
    }

    fn write_valid(&mut self, data: T) {
        self.data = data;
        self.is_garbage = false;
    }
}

pub struct PhysSystem {
    prgm_config: Config,
    mem: [Data<u8>; 65_536],
    via: RefCell<W65C22S>,
    step_wait_time: usize,
    opcode_fetching: bool,
    cycle_count: usize,
    sent_cycle_count: usize,
    gui_port_update_allowed: RefCell<bool>,
    step_count: usize,
    currently_running: bool,
    pa_as_breakpoint: bool,
    tx_log_msgs: Sender<LogMessage>,
    tx_gui_msgs: Sender<ToGuiMessage>,
    tx_sys_msgs: Sender<ToSysMessage>,
    rx_sys_msgs: Receiver<ToSysMessage>,
    tx_to_lcd: Option<Sender<SysToLcdMessage>>,
    lcd_handle: Option<JoinHandle<()>>,
}

impl Default for PhysSystem {
    fn default() -> Self {
        let (tx_log_msgs, _) = mpsc::channel();
        let (tx_gui_msgs, _) = mpsc::channel();
        let (tx_sys_msgs, rx_sys_msgs) = mpsc::channel();
        PhysSystem {
            prgm_config: Config {
                lcd_enabled: false,
                allow_garbage: false,
            },
            mem: [Data { data: 0xff, is_garbage: true }; 65_536],
            via: RefCell::new(W65C22S::new()),
            step_wait_time: DEFAULT_STEP_WAIT * 1000,
            opcode_fetching: false,
            cycle_count: 0,
            sent_cycle_count: 0,
            gui_port_update_allowed: RefCell::new(true),
            step_count: 0,
            currently_running: false,
            pa_as_breakpoint: true,
            tx_log_msgs,
            tx_gui_msgs,
            tx_sys_msgs,
            rx_sys_msgs,
            tx_to_lcd: None,
            lcd_handle: None,
        }
    }
}

/// A system with 16K of RAM, 32K of programmable (EEP)ROM,
/// and a 6522 mapped to $6000.
impl PhysSystem {
    pub fn new(
        prgm_config: Config,
        program: [u8; 32_768],
        tx_log_msgs: Sender<LogMessage>,
        tx_gui_msgs: Sender<ToGuiMessage>,
        tx_sys_msgs: Sender<ToSysMessage>,
        rx_sys_msgs: Receiver<ToSysMessage>
    ) -> PhysSystem {
        let (tx_to_lcd, lcd_handle) = if prgm_config.lcd_enabled {
            let (tx_to_lcd, rx_from_sys) = mpsc::channel();
            let lcd = LCD::new(Sender::clone(&tx_log_msgs), Sender::clone(&tx_gui_msgs));

            (Some(tx_to_lcd), Some(lcd.run(rx_from_sys)))
        } else {
            (None, None)
        };
        
        let mut mem: [Data<u8>; 65_536] = [Data::<u8>::new_garbage(); 65_536];
        for i in 0x8000..=0xffff {
            mem[i].data = program[i & 0x7fff];
            mem[i].is_garbage = false;
        }

        PhysSystem {
            prgm_config,
            mem,
            tx_log_msgs,
            tx_gui_msgs,
            tx_sys_msgs,
            rx_sys_msgs,
            tx_to_lcd,
            lcd_handle,
            ..Default::default()
        }
    }

    pub fn run(mut self) -> thread::JoinHandle<()> {
        let mut cpu = W65C02S::new();
        let mut gui_running = true;
        self.update_gui();

        thread::Builder::new().name("SYS thread".to_string()).spawn(move || {
            'sys_thread_main: loop {
                let sys_message = match self.currently_running {
                    true => {
                        if self.step(&mut cpu) == State::Stopped {
                            break 'sys_thread_main;
                        };
                        spin_sleep::sleep(time::Duration::from_micros(self.step_wait_time as u64));

                        let sys_message = self.rx_sys_msgs.try_recv();
                        if let Err(err) = sys_message { match err {
                            TryRecvError::Disconnected => panic!("GUI thread has hung up"),
                            TryRecvError::Empty => continue 'sys_thread_main,
                        }};
                        sys_message.unwrap()
                    },
                    false => self.rx_sys_msgs.recv().expect("GUI thread has hung up"),
                };
                
                match (sys_message, self.currently_running) {
                    (ToSysMessage::Run, false) => self.currently_running = true,
                    (ToSysMessage::Stop, true) => {
                        self.currently_running = false;
                        self.update_gui();
                    },
                    (ToSysMessage::Step, false) => if self.step(&mut cpu) == State::Stopped {
                        break 'sys_thread_main;
                    },
                    (ToSysMessage::ChangeWaitTime(new_wait_time), _) => self.step_wait_time = new_wait_time,
                    (ToSysMessage::ShowLog(print_log), _) => self.tx_log_msgs.send(
                        LogMessage::ChangePrintLog(print_log)
                    ).expect("Logger thread has hung up"),
                    (ToSysMessage::Breakpoint(pa_as_breakpoint), _) => self.pa_as_breakpoint = pa_as_breakpoint,
                    (ToSysMessage::Exit, _) => {
                        gui_running = false;
                        break 'sys_thread_main;
                    },
                    _ => {},
                };
            };

            log!(self.tx_log_msgs, "\n\nTotal cycle count: {}", self.cycle_count);

            if gui_running {
                self.update_gui();
                self.send_gui_msg(ToGuiMessage::Stopped);
            }

            if self.prgm_config.lcd_enabled {
                self.tx_to_lcd.unwrap().send(SysToLcdMessage::Exit).expect("LCD thread has hung up");
                print!("Waiting for LCD thread to end... ");
                self.lcd_handle.unwrap().join().unwrap();
                println!("LCD thread ended");
            }
        }).unwrap()
    }

    fn step(&mut self, cpu: &mut W65C02S) -> State {
        let required_delta = 
            if self.step_wait_time == 0 { 100_000 }
            else if self.step_wait_time <= 100 { 10_000 }
            else if self.step_wait_time <= 1_000 { 1_000 }
            else if self.step_wait_time <= 10_000 { 100 }
            else { 0 };

        if self.cycle_count > self.sent_cycle_count + required_delta || !self.currently_running {
            self.sent_cycle_count = self.cycle_count;
            self.send_gui_msg(ToGuiMessage::CycleCount(self.cycle_count));

            *self.gui_port_update_allowed.borrow_mut() = true;
            if self.prgm_config.lcd_enabled {
                self.tx_to_lcd.as_ref().unwrap().send(SysToLcdMessage::AllowGuiUpdate)
                    .expect("LCD thread has hung up");
            }
        }

        self.opcode_fetching = true;
        log!(self.tx_log_msgs, "\nStep {}:", self.step_count);
        self.step_count += 1;
        cpu.step(self)
    }

    fn update_gui(&self) {
        self.send_gui_msg(ToGuiMessage::CycleCount(self.cycle_count));
        self.send_gui_msg(ToGuiMessage::PortB(self.via.borrow().pb));
        self.send_gui_msg(ToGuiMessage::PortA(self.via.borrow().pa));

        if self.prgm_config.lcd_enabled {
            self.tx_to_lcd.as_ref().unwrap().send(SysToLcdMessage::ForceGuiUpdate)
            .expect("LCD thread has hung up");
        }
    }

    fn send_gui_msg(&self, msg: ToGuiMessage) {
        self.tx_gui_msgs.send(msg).expect("GUI thread has hung up");
    }
}

impl System for PhysSystem {
    fn read(&mut self, cpu: &mut W65C02S, addr: u16) -> u8 {
        self.cycle_count += 1;
        self.via.borrow_mut().clock_pulse(cpu);
        let value = match addr {
            // read from STACK (don't trigger panic on garbage read)
            0x0100..=0x01ff => self.mem[addr as usize].data,
            // read from RAM
            0x0000..=0x00ff | 0x0200..=0x3fff => self.mem[addr as usize].read(self.prgm_config.allow_garbage, 
                &self.tx_log_msgs, &format!("\nCPU reading garbage RAM data at addr {:04x}!", addr)),
            // read from VIA
            0x6000..=0x600f => self.via.borrow_mut().read(self, cpu, (addr as u8) & 0b0000_1111),
            // read from ROM
            0x8000..=0xffff => self.mem[addr as usize].read(self.prgm_config.allow_garbage, 
                &self.tx_log_msgs, &format!("\nCPU reading garbage ROM data at addr {:04x}!", addr)),
            _ => {
                log!(self.tx_log_msgs, "\nCPU reading garbage ROM data at addr {:04x}!", addr);
                if self.prgm_config.allow_garbage {
                    rand::random()
                } else {
                    panic!("CPU reading garbage ROM data at addr {:04x}!", addr)
                }
            },
        };
        log!(self.tx_log_msgs, "\n    READ  {:02x} at {:04x}", value, addr);
        if self.opcode_fetching {
            self.opcode_fetching = false;
            log!(self.tx_log_msgs, " {}", OPCODES[value as usize]);
        }
        value
    }

    fn write(&mut self, cpu: &mut W65C02S, addr: u16, value: u8) {
        self.cycle_count += 1;
        self.via.borrow_mut().clock_pulse(cpu);
        log!(self.tx_log_msgs, "\n    WRITE {:02x} at {:04x}", value, addr);
        match addr {
            // write to RAM (note that writes to 4000-7fff are useless but still happen on the physical system)
            0x0000..=0x5fff | 0x6010..=0x7fff => self.mem[addr as usize].write_valid(value),
            // write to VIA
            0x6000..=0x600f => {
                self.mem[addr as usize].write_valid(value);
                self.via.borrow_mut().write(self, cpu, (addr as u8) & 0b0000_1111, value);
            },
            // the write is useless
            _ => {},
        };
    }
}

const PORTB: u8 = 0x0;
const PORTA: u8 = 0x1;
const DDRB: u8 = 0x2;
const DDRA: u8 = 0x3;
const T1C_L: u8 = 0x4;
const T1C_H: u8 = 0x5;
const T1L_L: u8 = 0x6;
const T1L_H: u8 = 0x7;
const T2C_L: u8 = 0x8;
const T2C_H: u8 = 0x9;
const ACR: u8 = 0xb;
const IFR: u8 = 0xd;
const IER: u8 = 0xe;

struct W65C22S {
    ddra: u8,
    ddrb: u8,
    ora: u8,
    orb: u8,
    ira: Data<u8>,
    irb: Data<u8>,
    pa: Data<u8>, // Change to support individual garbage bits
    pb: Data<u8>, // Change to support individual garbage bits
    t1_l: u16,
    t1_c: Data<u16>,
    t2_l: u8,
    t2_c: Data<u16>,
    t2_trigger_interrupt: bool,
    acr: u8,
    ifr: u8,
    ier: u8,
}

impl W65C22S {
    fn new() -> W65C22S {
        // Default values tested on hardware
        // Except PORT B (= ORB) before sending it data, and ira/irb when input latching (prob garbage)
        W65C22S {
            ddra: 0x00,
            ddrb: 0x00,
            ora: 0x00,
            orb: 0x00,
            ira: Data::<u8>::new_garbage(),
            irb: Data::<u8>::new_garbage(),
            pa: Data::<u8>::new_garbage(),
            pb: Data::<u8>::new_garbage(),
            t1_l: 0xbaaa, // This one is weird, the value didn't change on 5 different occasions, to try again
            t1_c: Data::<u16>::new_garbage(), // Test what's in there multiple times in a row and see if it changes
            t2_l: 0x00, // Actually unknown, to test by starting the timer and reading the low byte
            t2_c: Data::<u16>::new_garbage(), // Same as t1_c
            t2_trigger_interrupt: false,
            acr: 0x00,
            ifr: 0x00,
            ier: 0x00,
        }
    }

    fn clock_pulse(&mut self, cpu: &mut W65C02S) {
        // Input latching
        self.irb = self.pb;
        self.ira = self.pa;

        // T1 operation
        if !self.t1_c.is_garbage {
            let mut t1_c = self.t1_c.data;
            if t1_c != 0 {
                t1_c -= 1;
                if t1_c == 0 {
                    self.change_interrupt_flag(cpu, Some(6), true);
                    
                    // Restart the timer if in free-run mode
                    if self.acr & 0b0100_0000 != 0 {
                        t1_c = self.t1_l;
                    }
    
                    // TEST WHAT THE "SINGLE NEGATIVE PULSE" EXACTLY IS
                    // ACTUALLY, THOROUGHLY TEST THE EFFECT OF THE TIMER ON PB7
                    // ALSO TEST WHAT'S IN ORB AFTER THAT
                    if self.acr & 0b1000_0000 != 0 {
                        self.pb.data = self.pb.data ^ 0b1000_0000;
                    }
                }
                self.t1_c.data = t1_c;
            }
        }

        // T2 operation
        if self.acr & 0b0010_0000 == 0 || self.pb.data & 0b0100_0000 == 0 {
            self.t2_c.data = self.t2_c.data.wrapping_sub(1);

            if self.t2_c.data == 0 && self.t2_trigger_interrupt {
                self.change_interrupt_flag(cpu, Some(5), true);

                self.t2_trigger_interrupt = false;
            }
        }
    }

    // Updates the status of IRQB, and if <flag> is Some(u32),
    // sets IFRx to <logic_level>, where x is a <flag> number between 0 and 6 
    fn change_interrupt_flag(&mut self, cpu: &mut W65C02S, flag: Option<u32>, logic_level: bool) {
        if let Some(flag) = flag {
            assert!(flag < 7, "Illegal flag value!!");
        
            let mask = match logic_level {
                true => 0b0000_0001u8,
                false => 0b0000_0000u8,
            };
            self.ifr = (self.ifr & 0b1111_1110u8.rotate_left(flag)) | mask.rotate_left(flag);
        }

        // Compute IFR7 and set interrupt request on the CPU accordingly
        let irq = match (self.ifr & 0b0111_1111) & (self.ier & 0b0111_1111) {
            0 => {
                cpu.set_irq(false);
                0b0000_0000
            },
            _ => {
                cpu.set_irq(true);
                0b1000_0000
            },
        };
        self.ifr = (self.ifr & 0b0111_1111) | irq;
    }

    fn read(&mut self, system: &PhysSystem, cpu: &mut W65C02S, addr: u8) -> u8 {
        match addr {
            PORTB => if self.acr & 0b0000_0010 == 0b0 { // If input latching is disabled
                // Read PB when DDRB = 0 (input), read ORB otherwise
                (self.orb | self.ddrb) & (self.pb.read(system.prgm_config.allow_garbage, 
                    &system.tx_log_msgs, "\nVIA reading garbage in PB!") | !self.ddrb)
            } else {
                // Read IRB when DDRB = 0 (input), read ORB otherwise
                (self.orb | self.ddrb) & (self.irb.read(system.prgm_config.allow_garbage, 
                    &system.tx_log_msgs, "\nVIA reading garbage in IRB!") | !self.ddrb)
            },
            PORTA => if self.acr & 0b0000_0001 == 0b0 { // If input latching is disabled
                self.pa.read(system.prgm_config.allow_garbage, &system.tx_log_msgs, "\nVIA reading garbage in PA!")
            } else {
                self.ira.read(system.prgm_config.allow_garbage, &system.tx_log_msgs, "\nVIA reading garbage in IRA!")
            },
            DDRB => self.ddrb,
            DDRA => self.ddra,
            T1C_L => {
                // "T1 interrupt flag IFR6 is reset"
                self.change_interrupt_flag(cpu, Some(6), false);
                // "8 bits from T1 low order counter transferred to MPU"
                (self.t1_c.read(system.prgm_config.allow_garbage, &system.tx_log_msgs, 
                    "VIA reading garbage in T1 low order counter!") & 0x00ff) as u8
            },
            T1C_H => ((self.t1_c.read(system.prgm_config.allow_garbage, &system.tx_log_msgs, 
                "VIA reading garbage in T1 high order counter!") & 0xff00) >> 8) as u8,
            T1L_L => (self.t1_l & 0x00ff) as u8,
            T1L_H => ((self.t1_l & 0xff00) >> 8) as u8,
            T2C_L => {
                // "IFR5 is reset"
                self.change_interrupt_flag(cpu, Some(5), false);
                // "8 bits from T2 low order counter transferred to MPU"
                (self.t2_c.read(system.prgm_config.allow_garbage, &system.tx_log_msgs, 
                    "VIA reading garbage in T2 low order counter!") & 0x00ff) as u8
            },
            T2C_H => ((self.t2_c.read(system.prgm_config.allow_garbage, &system.tx_log_msgs, 
                "VIA reading garbage in T2 high order counter!") & 0xff00) >> 8) as u8,
            ACR => self.acr,
            IFR => self.ifr,
            IER => self.ier | 0b1000_0000,
            0x10..=0xff => panic!("Invalid address"),
            _ => todo!(),
        }
    }

    fn write(&mut self, 
        system: &PhysSystem,
        cpu: &mut W65C02S,
        addr: u8, 
        value: u8
    ) {
        match addr {
            PORTB => {
                self.orb = value;
                // Only change the bit of PB when DDRB = 1 (output)
                self.pb.write_valid((self.pb.data | self.ddrb) & (value | !self.ddrb));
                
                if *system.gui_port_update_allowed.borrow() {
                    system.send_gui_msg(ToGuiMessage::PortB(self.pb));
                    *system.gui_port_update_allowed.borrow_mut() = false;
                }

                log!(system.tx_log_msgs, " -> PORT B: {:#010b} {:#04x} {}", 
                    self.pb.data, self.pb.data, self.pb.data);
                
                // Only send data to the LCD when the enable pin is set
                if self.pb.data & 0b0010_0000 == 0b0010_0000 {
                    let lcd_pins = LCDPins {
                        rs: if self.pb.data & 0b1000_0000 == 0 { false } else { true },
                        rw: if self.pb.data & 0b0100_0000 == 0 { false } else { true },
                        data: (self.pb.data & 0b0000_1111) << 4,
                    };
                    if let Some(tx) = &system.tx_to_lcd {
                        tx.send(SysToLcdMessage::PinChange(lcd_pins))
                            .expect("LCD thread has hung up");
                    }
                }
            },
            PORTA => {
                self.ora = value;
                // Only change the bit of PA when DDRA = 1 (output)
                self.pa.write_valid((self.pa.data | self.ddra) & (value | !self.ddra));

                if *system.gui_port_update_allowed.borrow() {
                    system.send_gui_msg(ToGuiMessage::PortA(self.pa));
                    *system.gui_port_update_allowed.borrow_mut() = false;
                }

                log!(system.tx_log_msgs, " -> PORT A: {:#010b} {:#04x} {}", 
                    self.pa.data, self.pa.data, self.pa.data);
                
                // Breakpoint mecanism
                if system.pa_as_breakpoint && system.currently_running {
                    system.tx_sys_msgs.send(ToSysMessage::Stop).expect("Self SYS thread has hung up!?!");
                    system.send_gui_msg(ToGuiMessage::Paused);
                }
            },
            DDRB => {
                self.ddrb = value;
                // Update PB based on ORB when DDRB = 1 (output)
                self.pb.write_valid((self.pb.data | self.ddrb) & (self.orb | !self.ddrb));
            },
            DDRA => {
                self.ddra = value;
                // Update PA based on ORA when DDRA = 1 (output)
                self.pa.write_valid((self.pa.data | self.ddra) & (self.ora | !self.ddra));
            },
            T1C_H => {
                // "8 bits loaded into T1 high order latches"
                self.t1_l = (self.t1_l & 0x00ff) | ((value as u16) << 8);
                // "Also, both high and low order latches are transferred 
                // into T1 counter and this initiates countdown"
                self.t1_c.write_valid(self.t1_l);
                // "T1 interrupt flag IFR6 is reset"
                self.change_interrupt_flag(cpu, Some(6), false);
            },
            T1C_L | T1L_L => {
                // "8 bits loaded into T1 low order latches"
                self.t1_l = (self.t1_l & 0xff00) | value as u16;
            },
            T1L_H => {
                // "8 bits loaded into T1 high order latches"
                self.t1_l = (self.t1_l & 0x00ff) | ((value as u16) << 8);
                // "T1 interrupt flag IFR6 is reset"
                self.change_interrupt_flag(cpu, Some(6), false);
            },
            T2C_L => {
                // "8 bits loaded into T2 low order latches"
                self.t2_l = value;
            },
            T2C_H => {
                // "8 bits loaded into T2 high order counter. 
                // Also, low order latches are transferred to low order counter"
                self.t2_c.write_valid(((value as u16) << 8) | self.t2_l as u16);
                // "IFR5 is reset"
                self.change_interrupt_flag(cpu, Some(5), false);
                self.t2_trigger_interrupt = true;
            },
            ACR => self.acr = value,
            IFR => {
                self.ifr = self.ifr & (value ^ 0xff);
                self.change_interrupt_flag(cpu, None, true);
            },
            IER => {
                self.ier = match value & 0b1000_0000 {
                    0 => self.ier & (value ^ 0xff),
                    _ => self.ier | value,
                };
                self.change_interrupt_flag(cpu, None, true);
            },
            0x10..=0xff => panic!("Invalid address"),
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sys() -> (PhysSystem, W65C02S, Receiver<LogMessage>) {
        let (tx_log_msgs, rx_log_msgs) = mpsc::channel();
        (PhysSystem { 
            gui_port_update_allowed: RefCell::new(false),
            tx_log_msgs,
            ..Default::default() 
        }, W65C02S::new(), rx_log_msgs)
    }

    #[test]
    fn simple_write_pb() {
        let (sys, mut cpu, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via.borrow_mut();

        via.write(&sys, &mut cpu, DDRB, 0b1111_1111);
        via.write(&sys, &mut cpu, PORTB, 0x42);

        assert_eq!(0x42, via.pb.data);
    }

    #[test]
    fn complex_write_pb() {
        let (sys, mut cpu, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via.borrow_mut();
        
        via.pb = Data { data: 0b1100_0011, is_garbage: false };

        via.write(&sys, &mut cpu, DDRB, 0b0110_1001);
        via.write(&sys, &mut cpu, PORTB, 0xa7);

        assert_eq!(0b1010_0011, via.pb.data);
    }

    #[test]
    fn change_ddrb() {
        let (sys, mut cpu, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via.borrow_mut();
        
        via.pb = Data { data: 0xc3, is_garbage: false };

        via.write(&sys, &mut cpu, DDRB, 0x00);
        via.write(&sys, &mut cpu, PORTB, 0x42);

        assert_eq!(0xc3, via.pb.data);

        via.write(&sys, &mut cpu, DDRB, 0x0f);

        assert_eq!(0xc2, via.pb.data);

        via.write(&sys, &mut cpu, DDRB, 0xff);

        assert_eq!(0x42, via.pb.data);
    }

    #[test]
    fn t1_one_shot() {
        let (sys, mut cpu, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via.borrow_mut();
        
        // PB7 disabled, one-shot mode
        via.write(&sys, &mut cpu, ACR, 0b0000_0011);
        // Set IER6
        via.write(&sys, &mut cpu, IER, 0b1100_0000);

        via.write(&sys, &mut cpu, T1L_L, 0x37);
        via.write(&sys, &mut cpu, T1L_H, 0x13);
        assert_eq!(0x1337, via.t1_l);

        via.write(&sys, &mut cpu, T1C_L, 0x02);
        via.write(&sys, &mut cpu, T1C_H, 0x00);

        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        via.clock_pulse(&mut cpu);
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        via.clock_pulse(&mut cpu);
        assert_eq!(0b1100_0000, via.ifr, "IFR6 should be set");
        
        via.clock_pulse(&mut cpu);
        assert_eq!(Data { data: 0x0000u16, is_garbage: false }, 
            via.t1_c, "Timer shouldn't be running anymore");
    }

    #[test]
    fn t1_os_long_timer() {
        let (sys, mut cpu, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via.borrow_mut();
        
        // PB7 disabled, one-shot mode
        via.write(&sys, &mut cpu, ACR, 0b0000_0011);
        // Set IER6
        via.write(&sys, &mut cpu, IER, 0b1100_0000);

        via.write(&sys, &mut cpu, T1C_L, (500u16 & 0x00ff) as u8);
        via.write(&sys, &mut cpu, T1C_H, ((500u16 & 0xff00) >> 8) as u8);
        
        for _i in 0..500 {
            assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
            via.clock_pulse(&mut cpu);
        }
        assert_eq!(0b1100_0000, via.ifr, "IFR6 should be set");
        
        via.clock_pulse(&mut cpu);
        assert_eq!(Data { data: 0x0000u16, is_garbage: false }, 
            via.t1_c, "Timer shouldn't be running anymore");
    }
        
    #[test]
    fn t1_freerun() {
        let (sys, mut cpu, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via.borrow_mut();

        // PB7 disabled, freerun mode
        via.write(&sys, &mut cpu, ACR, 0b0100_0011);
        // Set IER6
        via.write(&sys, &mut cpu, IER, 0b1100_0000);

        via.write(&sys, &mut cpu, T1C_L, 0x02);
        via.write(&sys, &mut cpu, T1C_H, 0x00);

        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        via.clock_pulse(&mut cpu);
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        via.clock_pulse(&mut cpu);
        assert_eq!(0b1100_0000, via.ifr, "IFR6 should be set");

        via.clock_pulse(&mut cpu);

        // Clear IFR6
        via.write(&sys, &mut cpu, IFR, 0b0100_0000);
        assert_eq!(0x00, via.ifr, "Interrupt flags should have been reset");

        via.clock_pulse(&mut cpu);
        assert_eq!(0b1100_0000, via.ifr, "IFR6 should be set");
    }

    #[test]
    fn t2_one_shot() {
        let (sys, mut cpu, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via.borrow_mut();
        
        // One-shot mode
        via.write(&sys, &mut cpu, ACR, 0b0000_0011);
        // Set IER5
        via.write(&sys, &mut cpu, IER, 0b1010_0000);

        via.write(&sys, &mut cpu, T2C_L, 0x02);
        via.write(&sys, &mut cpu, T2C_H, 0x00);

        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        via.clock_pulse(&mut cpu);
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        via.clock_pulse(&mut cpu);
        assert_eq!(0b1010_0000, via.ifr, "IFR5 should be set");
        
        via.clock_pulse(&mut cpu);
        assert_eq!(0b1010_0000, via.ifr, "IFR5 should still be set");
        assert_eq!(Data { data: 0xffffu16, is_garbage: false }, 
            via.t2_c, "Timer should still be running");

        // Clear IFR5
        via.write(&sys, &mut cpu, IFR, 0b0010_1000);
        via.clock_pulse(&mut cpu);
        assert_eq!(0b0000_0000, via.ifr, "IFR5 should be reset");
        assert_eq!(Data { data: 0xfffeu16, is_garbage: false }, 
            via.t2_c, "Timer should still be running");
    }

    #[test]
    fn t2_full_cycle() {
        let (sys, mut cpu, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via.borrow_mut();
        
        // One-shot mode
        via.write(&sys, &mut cpu, ACR, 0b0000_0011);
        // Set IER5
        via.write(&sys, &mut cpu, IER, 0b1010_0000);

        via.write(&sys, &mut cpu, T2C_L, 0x02);
        via.write(&sys, &mut cpu, T2C_H, 0x00);

        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        via.clock_pulse(&mut cpu);
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        via.clock_pulse(&mut cpu);
        assert_eq!(0b1010_0000, via.ifr, "IFR5 should be set");
        
        // Clear IFR5
        via.write(&sys, &mut cpu, IFR, 0b0010_0000);
        assert_eq!(0b0000_0000, via.ifr, "IFR5 should be reset");
        
        for _i in 0..65_539 {
            assert_eq!(0x00, via.ifr, "No interrupt flags should be set");
            via.clock_pulse(&mut cpu);
        }
        assert_eq!(Data { data: 0xfffdu16, is_garbage: false }, 
            via.t2_c, "Timer should still be running");
    }

    #[test]
    fn t2_pulse_count() {
        let (sys, mut cpu, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via.borrow_mut();

        via.pb.write_valid(0xff);

        // Pulse counting mode
        via.write(&sys, &mut cpu, ACR, 0b0010_0011);
        // Set IER5
        via.write(&sys, &mut cpu, IER, 0b1010_0000);

        via.write(&sys, &mut cpu, T2C_L, 0x03);
        via.write(&sys, &mut cpu, T2C_H, 0x00);

        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        via.clock_pulse(&mut cpu);
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        via.clock_pulse(&mut cpu);
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        
        via.pb.data = via.pb.data & 0b1011_1111;
        via.clock_pulse(&mut cpu);
        via.clock_pulse(&mut cpu);
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        
        via.pb.data = via.pb.data | 0b0100_0000;
        via.clock_pulse(&mut cpu);
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");

        via.pb.data = via.pb.data & 0b1011_1111;
        via.clock_pulse(&mut cpu);
        assert_eq!(0b1010_0000, via.ifr, "IFR5 should be set");

        via.clock_pulse(&mut cpu);
        via.clock_pulse(&mut cpu);
        assert_eq!(Data { data: 0xfffeu16, is_garbage: false }, 
            via.t2_c, "Timer should still be running");
    }


    #[test]
    fn interrupts() {
        let (sys, mut cpu, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via.borrow_mut();

        assert_eq!(0x00, via.ifr, "No interrupts should be enabled yet");
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");
        
        via.change_interrupt_flag(&mut cpu, None, true);
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");

        via.change_interrupt_flag(&mut cpu, Some(0), false);
        assert_eq!(0x00, via.ifr, "No interrupt flags should be set yet");

        via.change_interrupt_flag(&mut cpu, Some(0), true);
        via.change_interrupt_flag(&mut cpu, Some(5), true);
        via.change_interrupt_flag(&mut cpu, Some(6), true);
        assert_eq!(0b0110_0001, via.ifr, "IFR6, IFR5 and IFR0 should be set");

        via.write(&sys, &mut cpu, IER, 0b1000_0010);
        assert_eq!(0b0110_0001, via.ifr, "IFR7 should still be reset");
        
        via.write(&sys, &mut cpu, IER, 0b1000_1001);
        assert_eq!(0b1000_1011, via.ier);
        assert_eq!(0b1110_0001, via.ifr, "IFR7 should be set");
        
        via.write(&sys, &mut cpu, IFR, 0b1000_0000);
        assert_eq!(0b1110_0001, via.ifr, "Writing to IFR7 shouldn't clear it");
        
        via.write(&sys, &mut cpu, IFR, 0b0100_1000);
        assert_eq!(0b1010_0001, via.ifr, "IFR6 should be cleared");
        
        via.write(&sys, &mut cpu, IFR, 0b1001_0001);
        assert_eq!(0b0010_0000, via.ifr, "IFR7 should be cleared");
        
        via.change_interrupt_flag(&mut cpu, Some(3), true);
        assert_eq!(0b1010_1000, via.ifr, "IFR7 should be set");

        via.write(&sys, &mut cpu, IER, 0b0010_1001);
        assert_eq!(0b1000_0010, via.ier);
        assert_eq!(0b0010_1000, via.ifr, "IFR7 should be reset");
    }
}