use w65c02s::{System, W65C02S, State};
use std::{thread::{self, JoinHandle}, time};
use std::sync::mpsc::{self, Sender, Receiver, TryRecvError};
use crate::{Config, LogMessage, ToGuiMessage};

mod lcd;
mod via;
use lcd::{LCD, SysToLcdMessage};

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
    via: via::W65C22S,
    via_pb: u8,
    pb_changed: bool,
    via_pa: u8,
    pa_changed: bool,
    irq: bool,
    step_wait_time: usize,
    opcode_fetching: bool,
    cycle_count: usize,
    sent_cycle_count: usize,
    screen_update_period: usize,
    step_count: usize,
    currently_running: bool,
    pa_as_breakpoint: bool,
    tx_log_msgs: Sender<LogMessage>,
    tx_gui_msgs: Sender<ToGuiMessage>,
    rx_sys_msgs: Receiver<ToSysMessage>,
    tx_to_lcd: Option<Sender<SysToLcdMessage>>,
    lcd_handle: Option<JoinHandle<()>>,
}

impl Default for PhysSystem {
    fn default() -> Self {
        let (tx_log_msgs, _) = mpsc::channel();
        let (tx_gui_msgs, _) = mpsc::channel();
        let (_, rx_sys_msgs) = mpsc::channel();
        PhysSystem {
            prgm_config: Config {
                lcd_enabled: false,
                allow_garbage: false,
            },
            mem: [Data { data: 0xff, is_garbage: true }; 65_536],
            via: via::W65C22S::new(),
            via_pb: rand::random(),
            pb_changed: false,
            via_pa: rand::random(),
            pa_changed: false,
            irq: false,
            step_wait_time: DEFAULT_STEP_WAIT * 1000,
            opcode_fetching: false,
            cycle_count: 0,
            sent_cycle_count: 0,
            screen_update_period: 0,
            step_count: 0,
            currently_running: false,
            pa_as_breakpoint: true,
            tx_log_msgs,
            tx_gui_msgs,
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
                        cpu.set_irq(self.irq);
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
                    (ToSysMessage::Run, false) => {
                        self.currently_running = true;

                        self.send_lcd_msg(SysToLcdMessage::AllowOneUpdate);
                    },
                    (ToSysMessage::Stop, true) => {
                        self.currently_running = false;

                        self.update_gui();
                        self.send_lcd_msg(SysToLcdMessage::AllowAllUpdates);
                    },
                    (ToSysMessage::Step, false) => {
                        cpu.set_irq(self.irq);
                        if self.step(&mut cpu) == State::Stopped {
                            break 'sys_thread_main;
                        }
                    }, 
                    (ToSysMessage::ChangeWaitTime(new_wait_time), _) => {
                        self.step_wait_time = new_wait_time;

                        self.screen_update_period =  
                            if self.step_wait_time == 0 { 100_000 }
                            else if self.step_wait_time <= 100 { 10_000 }
                            else if self.step_wait_time <= 1_000 { 1_000 }
                            else if self.step_wait_time <= 10_000 { 100 }
                            else { 0 };

                        if self.screen_update_period == 0 {
                            self.send_lcd_msg(SysToLcdMessage::AllowAllUpdates);
                        } else {
                            self.send_lcd_msg(SysToLcdMessage::AllowOneUpdate);
                        }
                    },
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
                self.send_lcd_msg(SysToLcdMessage::Exit);
                print!("Waiting for LCD thread to end... ");
                self.lcd_handle.unwrap().join().unwrap();
                println!("LCD thread ended");
            }
        }).unwrap()
    }

    fn step(&mut self, cpu: &mut W65C02S) -> State {
        if self.cycle_count > self.sent_cycle_count + self.screen_update_period || !self.currently_running {
            self.sent_cycle_count = self.cycle_count;
            
            self.update_gui();
        }

        self.opcode_fetching = true;
        log!(self.tx_log_msgs, "\nStep {}:", self.step_count);
        self.step_count += 1;
        cpu.step(self)
    }

    fn update_gui(&self) {
        self.send_gui_msg(ToGuiMessage::CycleCount(self.cycle_count));

        self.send_gui_msg(ToGuiMessage::PortB(self.via_pb));
        self.send_gui_msg(ToGuiMessage::PortA(self.via_pa));
        
        self.send_lcd_msg(SysToLcdMessage::AllowOneUpdate);
    }

    fn send_gui_msg(&self, msg: ToGuiMessage) {
        self.tx_gui_msgs.send(msg).expect("GUI thread has hung up");
    }

    fn send_lcd_msg(&self, msg: SysToLcdMessage) {
        if let Some(tx) = &self.tx_to_lcd {
            tx.send(msg).expect("LCD thread has hung up");
        }
    }
}

impl System for PhysSystem {
    fn read(&mut self, _cpu: &mut W65C02S, addr: u16) -> u8 {
        let mut via = self.via;

        self.cycle_count += 1;
        via.clock_pulse(self);

        let value = match addr {
            // read from STACK (don't trigger panic on garbage read)
            0x0100..=0x01ff => self.mem[addr as usize].data,
            // read from RAM
            0x0000..=0x00ff | 0x0200..=0x3fff => self.mem[addr as usize].read(self.prgm_config.allow_garbage, 
                &self.tx_log_msgs, &format!("\nCPU reading garbage RAM data at addr {:04x}!", addr)),
            // read from VIA
            0x6000..=0x600f => via.read(self, (addr as u8) & 0b0000_1111),
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

        self.via = via;
        value
    }

    fn write(&mut self, _cpu: &mut W65C02S, addr: u16, value: u8) {
        let mut via = self.via;

        self.cycle_count += 1;
        via.clock_pulse(self);

        log!(self.tx_log_msgs, "\n    WRITE {:02x} at {:04x}", value, addr);

        match addr {
            // write to RAM (note that writes to 4000-7fff are useless but still happen on the physical system)
            0x0000..=0x5fff | 0x6010..=0x7fff => self.mem[addr as usize].write_valid(value),
            // write to VIA
            0x6000..=0x600f => {
                self.mem[addr as usize].write_valid(value);
                via.write(self, (addr as u8) & 0b0000_1111, value);
            },
            // the write is useless
            _ => {},
        };

        if self.pb_changed {
            self.pb_changed = false;
            log!(self.tx_log_msgs, " -> PORT B: {:#010b} {:#04x} {}", 
                self.via_pb, self.via_pb, self.via_pb);
        }

        if self.pa_changed {
            self.pa_changed = false;
            log!(self.tx_log_msgs, " -> PORT A: {:#010b} {:#04x} {}", 
                self.via_pa, self.via_pa, self.via_pa);

            // Breakpoint mecanism
            if self.pa_as_breakpoint && self.currently_running {
                self.currently_running = false;
                self.update_gui();
            }
        }

        self.via = via;
    }
}

impl via::ViaSystem for PhysSystem {
    fn read_port_b(&mut self, _via: &mut via::W65C22S) -> u8 {
        // lcd.read()
        rand::random()
    }
    
    fn read_port_a(&mut self, _via: &mut via::W65C22S) -> u8 {
        // Nothing connected to Port A yet, thus send random value on floating pins
        rand::random()
    }

    fn write_port_b(&mut self, _via: &mut via::W65C22S, bit: u8, level: bool) {
        // Update PB bus
        self.via_pb = match level {
            true => self.via_pb | (1 << bit),
            false => self.via_pb & !(1 << bit),
        };

        self.pb_changed = true;
        
        // If the LCD screen is enabled, send it data
        if let Some(tx) = &self.tx_to_lcd {
            match bit {
                0..=3 => tx.send(SysToLcdMessage::DataPinChange((bit + 4, level))),
                5 => tx.send(SysToLcdMessage::EnablePinChange(level)),
                6 => tx.send(SysToLcdMessage::ReadWritePinChange(level)),
                7 => tx.send(SysToLcdMessage::RegisterPinChange(level)),
                // PB4 isn't connected to the LCD
                _ => { Ok(()) },
            }.expect("LCD thread has hung up");
        }
    }

    fn write_port_a(&mut self, _via: &mut via::W65C22S, bit: u8, level: bool) {
        // Update PA bus
        self.via_pa = match level {
            true => self.via_pa | (1 << bit),
            false => self.via_pa & !(1 << bit),
        };

        self.pa_changed = true;
    }

    fn write_cb2(&mut self, _via: &mut via::W65C22S, _level: bool) {
        // Nothing connected to that yet
    }

    fn update_irq(&mut self, _via: &mut via::W65C22S, irq: bool) {
        self.irq = irq;
    }
}
