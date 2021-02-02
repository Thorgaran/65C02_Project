use w65c02s::{System, W65C02S, State};
use std::{thread::{self, JoinHandle}, time};
use std::sync::mpsc::{self, Sender, Receiver, TryRecvError};
use crate::{GuiToCpuMessage, ToGuiMessage, DEFAULT_STEP_WAIT};
use crate::logger::LogMessage;
use crate::lcd::{LCD, CpuToLcdMessage, LCDPins};

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

pub struct PhysSystem {
    mem: [u8; 65_536],
    via: W65C22S,
    step_wait_time: usize,
    opcode_fetching: bool,
    cycle_count: usize,
    step_count: usize,
    tx_log_msgs: Sender<LogMessage>,
    tx_to_gui: Sender<ToGuiMessage>,
    rx_from_gui: Receiver<GuiToCpuMessage>,
    lcd_enabled: bool,
    tx_to_lcd: Option<Sender<CpuToLcdMessage>>,
    lcd_handle: Option<JoinHandle<()>>,
}

/// A system with 16K of RAM, 32K of programmable (EEP)ROM,
/// and a 6522 mapped to $6000.
impl PhysSystem {
    pub fn new(
        program: [u8; 32_768],
        lcd_enabled: bool,
        tx_log_msgs: Sender<LogMessage>,
        tx_to_gui: Sender<ToGuiMessage>,
        rx_from_gui: Receiver<GuiToCpuMessage>
    ) -> PhysSystem {
        let (tx_to_lcd, lcd_handle) = if lcd_enabled {
            let (tx_to_lcd, rx_from_cpu) = mpsc::channel();
            let lcd = LCD::new(Sender::clone(&tx_log_msgs), Sender::clone(&tx_to_gui));

            (Some(tx_to_lcd), Some(lcd.run(rx_from_cpu)))
        } else {
            (None, None)
        };
        
        let mut mem: [u8; 65_536] = [0xFF; 65_536];
        // Insert the program into the second half of mem
        mem[0x8000..].copy_from_slice(&program);

        PhysSystem { 
            mem,
            via: W65C22S::new(),
            step_wait_time: DEFAULT_STEP_WAIT * 1000,
            opcode_fetching: false,
            cycle_count: 0,
            step_count: 0,
            tx_log_msgs,
            tx_to_gui,
            rx_from_gui,
            lcd_enabled,
            tx_to_lcd,
            lcd_handle,
        }
    }

    pub fn run(mut self) -> thread::JoinHandle<()> {
        let mut cpu = W65C02S::new();

        thread::Builder::new().name("CPU thread".to_string()).spawn(move || {
            'cpu_thread_main: loop {
                match self.rx_from_gui.recv().expect("GUI thread has hung up") {
                    GuiToCpuMessage::Run => 'run: loop {
                        match self.rx_from_gui.try_recv() {
                            Err(TryRecvError::Disconnected) => panic!("GUI thread has hung up"),
                            Ok(GuiToCpuMessage::Stop) => break 'run,
                            Ok(GuiToCpuMessage::ChangeWaitTime(new_wait_time)) => self.step_wait_time = new_wait_time,
                            Ok(GuiToCpuMessage::ShowLog(print_log)) => self.tx_log_msgs.send(
                                LogMessage::ChangePrintLog(print_log)
                            ).expect("Logger thread has hung up"),
                            Ok(GuiToCpuMessage::Exit) => break 'cpu_thread_main,
                            // If there are no messages or the message is "step" or "run", continue running
                            _ => {},
                        }
                        if self.step(&mut cpu) == State::Stopped {
                            break 'cpu_thread_main;
                        }
                        spin_sleep::sleep(time::Duration::from_micros(self.step_wait_time as u64));
                    },
                    GuiToCpuMessage::Step => if self.step(&mut cpu) == State::Stopped {
                        break 'cpu_thread_main;
                    },
                    GuiToCpuMessage::ChangeWaitTime(new_wait_time) => self.step_wait_time = new_wait_time,
                    GuiToCpuMessage::ShowLog(print_log) => self.tx_log_msgs.send(
                        LogMessage::ChangePrintLog(print_log)
                    ).expect("Logger thread has hung up"),
                    GuiToCpuMessage::Exit => break 'cpu_thread_main,
                    _ => {},
                }
            };

            log!(self.tx_log_msgs, "\n\nTotal cycle count: {}", self.cycle_count);

            send_cpu_msg(&self.tx_to_gui, ToGuiMessage::CycleCount(self.cycle_count));
            send_cpu_msg(&self.tx_to_gui, ToGuiMessage::Stopped);

            if self.lcd_enabled {
                self.tx_to_lcd.unwrap().send(CpuToLcdMessage::Exit).expect("LCD thread has hung up");
                self.lcd_handle.unwrap().join().unwrap();
            }

            thread::sleep(time::Duration::from_millis(500));
        }).unwrap()
    }

    fn step(&mut self, cpu: &mut W65C02S) -> State {
        send_cpu_msg(&self.tx_to_gui, ToGuiMessage::CycleCount(self.cycle_count));
        self.opcode_fetching = true;
        log!(self.tx_log_msgs, "\nStep {}:", self.step_count);
        self.step_count += 1;
        cpu.step(self)
    }
}

impl System for PhysSystem {
    fn read(&mut self, _cpu: &mut W65C02S, addr: u16) -> u8 {
        self.cycle_count += 1;
        self.via.clock_pulse();
        let value = match addr {
            // read from RAM
            0x0000..=0x3fff => self.mem[addr as usize],
            // read from VIA
            0x6000..=0x600f => self.via.read((addr as u8) & 0b0000_1111),
            // read from ROM
            0x8000..=0xffff => self.mem[addr as usize],
            _ => {
                log!(self.tx_log_msgs, 
                    "\n    Undefined behavior! Processor trying to read garbage at address {:04x}.", 
                    addr
                );
                panic!("Processor trying to read garbage");
            },
        };
        log!(self.tx_log_msgs, "\n    READ  {:02x} at {:04x}", value, addr);
        if self.opcode_fetching {
            self.opcode_fetching = false;
            log!(self.tx_log_msgs, " {}", OPCODES[value as usize]);
        }
        value
    }

    fn write(&mut self, _cpu: &mut W65C02S, addr: u16, value: u8) {
        self.cycle_count += 1;
        self.via.clock_pulse();
        log!(self.tx_log_msgs, "\n    WRITE {:02x} at {:04x}", value, addr);
        match addr {
            // write to RAM (note that writes to 4000-7fff are useless but still happen on the physical system)
            0x0000..=0x5fff | 0x6010..=0x7fff => self.mem[addr as usize] = value,
            // write to VIA
            0x6000..=0x600f => {
                self.mem[addr as usize] = value;
                self.via.write(&self.tx_log_msgs, &self.tx_to_gui, 
                    &self.tx_to_lcd, (addr as u8) & 0b0000_1111, value);
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
const ACR: u8 = 0xb;

struct W65C22S {
    ddra: u8,
    ddrb: u8,
    ora: u8,
    orb: u8,
    ira: u8,
    irb: u8,
    pa: u8,
    pb: u8,
    acr: u8,
}

impl W65C22S {
    fn new() -> W65C22S {
        W65C22S {
            ddra: 0x0,
            ddrb: 0x0,
            ora: 0x0,
            orb: 0x0,
            ira: 0x0,
            irb: 0x0,
            pa: 0x0,
            pb: 0x0,
            acr: 0b0000_0011,
        }
    }

    fn clock_pulse(&mut self) {
        // Input latching
        self.irb = self.pb;
        self.ira = self.pa;
    }

    fn read(&mut self, addr: u8) -> u8 {
        match addr {
            PORTB => if self.acr & 0b0000_0010 == 0b0 { // If input latching is disabled
                // Read PB when DDRB = 0 (input), read ORB otherwise
                (self.orb | self.ddrb) & (self.pb | !self.ddrb)
            } else {
                // Read IRB when DDRB = 0 (input), read ORB otherwise
                (self.orb | self.ddrb) & (self.irb | !self.ddrb)
            },
            PORTA => if self.acr & 0b0000_0001 == 0b0 { // If input latching is disabled
                self.pa
            } else {
                self.ira
            },
            DDRB => self.ddrb,
            DDRA => self.ddra,
            ACR => self.acr,
            0x10..=0xff => panic!("Invalid address"),
            _ => todo!(),
        }
    }

    fn write(&mut self, 
        tx_log_msgs: &Sender<LogMessage>, 
        tx_to_gui: &Sender<ToGuiMessage>,
        tx_to_lcd: &Option<Sender<CpuToLcdMessage>>,
        addr: u8, 
        value: u8
    ) {
        match addr {
            PORTB => {
                self.orb = value;
                // Only change the bit of PB when DDRB = 1 (output)
                self.pb = (self.pb | self.ddrb) & (value | !self.ddrb);
                send_cpu_msg(tx_to_gui, ToGuiMessage::PortB(self.pb));
                log!(tx_log_msgs, " -> PORT B: {:#010b} {:#04x} {}", self.pb, self.pb, self.pb);
                // Only send data to the LCD when the enable pin is set
                if self.pb & 0b0010_0000 == 0b0010_0000 {
                    let lcd_pins = LCDPins {
                        rs: if self.pb & 0b1000_0000 == 0 { false } else { true },
                        rw: if self.pb & 0b0100_0000 == 0 { false } else { true },
                        data: (self.pb & 0b0000_1111) << 4,
                    };
                    if let Some(tx) = tx_to_lcd {
                        tx.send(CpuToLcdMessage::PinChange(lcd_pins))
                            .expect("LCD thread has hung up");
                    }
                }
            },
            PORTA => {
                self.ora = value;
                // Only change the bit of PA when DDRA = 1 (output)
                self.pa = (self.pa | self.ddra) & (value | !self.ddra);
                send_cpu_msg(tx_to_gui, ToGuiMessage::PortA(self.pa));
                log!(tx_log_msgs, " -> PORT A: {:#010b} {:#04x} {}", self.pa, self.pa, self.pa);
            },
            DDRB => {
                self.ddrb = value;
                // Update PB based on ORB when DDRB = 1 (output)
                self.pb = (self.pb | self.ddrb) & (self.orb | !self.ddrb);
            },
            DDRA => {
                self.ddra = value;
                // Update PA based on ORA when DDRA = 1 (output)
                self.pa = (self.pa | self.ddra) & (self.ora | !self.ddra);
            }
            ACR => self.acr = value,
            0x10..=0xff => panic!("Invalid address"),
            _ => todo!(),
        }
    }
}

fn send_cpu_msg(tx_to_gui: &Sender<ToGuiMessage>, msg: ToGuiMessage) {
    tx_to_gui.send(msg).expect("GUI thread has hung up");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_write_pb() {
        let mut via = W65C22S::new();

        via.write(DDRB, 0b1111_1111);
        via.write(PORTB, 0x42);

        assert_eq!(0x42, via.pb);
    }

    #[test]
    fn complex_write_pb() {
        let mut via = W65C22S::new();
        
        via.pb = 0b1100_0011;

        via.write(DDRB, 0b0110_1001);
        via.write(PORTB, 0xa7);

        assert_eq!(0b1010_0011, via.pb);
    }

    #[test]
    fn change_ddrb() {
        let mut via = W65C22S::new();
        
        via.pb = 0xc3;

        via.write(DDRB, 0x00);
        via.write(PORTB, 0x42);

        assert_eq!(0xc3, via.pb);

        via.write(DDRB, 0x0f);

        assert_eq!(0xc2, via.pb);

        via.write(DDRB, 0xff);

        assert_eq!(0x42, via.pb);
    }
}