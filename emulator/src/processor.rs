use w65c02s::{System, W65C02S, State};
use std::{thread, time};
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use crate::{GuiMessage, CpuMessage, DEFAULT_STEP_WAIT};

pub struct PhysSystem {
    mem: [u8; 65536],
    via: W65C22S,
    step_wait_time: usize,
    cycle_count: usize,
    step_count: usize,
    show_log: bool,
    log_file: Option<File>,
    tx: Sender<CpuMessage>,
    rx: Receiver<GuiMessage>,
}

/// A system with 16K of RAM, 32K of programmable (EEP)ROM,
/// and a 6522 mapped to $6000.
impl PhysSystem {
    pub fn new(
        program: [u8; 32_768],
        log_file: Option<File>,
        tx: Sender<CpuMessage>,
        rx: Receiver<GuiMessage>
    ) -> PhysSystem {
        let mut mem: [u8; 65_536] = [0xFF; 65_536];
        // Insert the program into the second half of mem
        mem[0x8000..].copy_from_slice(&program);

        PhysSystem { 
            mem,
            via: W65C22S::new(),
            step_wait_time: DEFAULT_STEP_WAIT,
            cycle_count: 0,
            step_count: 0,
            show_log: false,
            log_file,
            tx,
            rx,
        }
    }

    pub fn run(mut self) -> thread::JoinHandle<()> {
        let mut cpu = W65C02S::new();
        thread::Builder::new().name("CPU thread".to_string()).spawn(move || {
            'cpu_thread_main: loop {
                match self.rx.recv().expect("GUI thread has hung up") {
                    GuiMessage::Run => 'run: loop {
                        match self.rx.try_recv() {
                            Err(TryRecvError::Disconnected) => panic!("GUI thread has hung up"),
                            Ok(GuiMessage::Stop) => break 'run,
                            Ok(GuiMessage::ChangeWaitTime(new_wait_time)) => self.step_wait_time = new_wait_time,
                            Ok(GuiMessage::ShowLog(show_log)) => self.show_log = show_log,
                            Ok(GuiMessage::Exit) => break 'cpu_thread_main,
                            // If there are no messages or the message is "step" or "run", continue running
                            _ => {},
                        }
                        if self.step(&mut cpu) == State::Stopped {
                            break 'cpu_thread_main;
                        }
                        thread::sleep(time::Duration::from_millis(self.step_wait_time as u64));
                    },
                    GuiMessage::Step => if self.step(&mut cpu) == State::Stopped {
                        break 'cpu_thread_main;
                    },
                    GuiMessage::ChangeWaitTime(new_wait_time) => self.step_wait_time = new_wait_time,
                    GuiMessage::ShowLog(show_log) => self.show_log = show_log,
                    GuiMessage::Exit => break 'cpu_thread_main,
                    _ => {},
                }
            };
            log(&mut self.log_file, &self.show_log, format!(
                "\n\nTotal cycle count: {}", 
                self.cycle_count
            ));
            send_cpu_msg(&self.tx, CpuMessage::CycleCount(self.cycle_count));
            send_cpu_msg(&self.tx, CpuMessage::Stopped);
            thread::sleep(time::Duration::from_millis(500));
        }).unwrap()
    }

    fn step(&mut self, cpu: &mut W65C02S) -> State {
        send_cpu_msg(&self.tx, CpuMessage::CycleCount(self.cycle_count));
        log(&mut self.log_file, &self.show_log, format!("\nStep {}:", self.step_count));
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
                let err_msg = format!(
                    "\n    Undefined behavior! Processor trying to read garbage at address {:04x}.", 
                    addr
                );
                log(&mut self.log_file, &self.show_log, err_msg);
                panic!()
            },
        };
        log(&mut self.log_file, &self.show_log, format!("\n    READ  {:02x} at {:04x}", value, addr));
        value
    }

    fn write(&mut self, _cpu: &mut W65C02S, addr: u16, value: u8) {
        self.cycle_count += 1;
        self.via.clock_pulse();
        log(&mut self.log_file, &self.show_log, format!("\n    WRITE {:02x} to {:04x}", value, addr));
        match addr {
            // write to RAM (note that writes to 4000-7fff are useless but still happen on the physical system)
            0x0000..=0x5fff | 0x6010..=0x7fff => self.mem[addr as usize] = value,
            // write to VIA
            0x6000..=0x600f => {
                self.mem[addr as usize] = value;
                self.via.write(&mut self.log_file, &self.show_log, &self.tx, (addr as u8) & 0b0000_1111, value);
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
        log_file: &mut Option<File>, 
        show_log: &bool, 
        tx: &Sender<CpuMessage>, 
        addr: u8, 
        value: u8
    ) {
        match addr {
            PORTB => {
                self.orb = value;
                // Only change the bit of PB when DDRB = 1 (output)
                self.pb = (self.pb | self.ddrb) & (value | !self.ddrb);
                send_cpu_msg(tx, CpuMessage::PortB(self.pb));
                log(log_file, show_log, format!(" -> PORT B: {:#010b} {:#04x} {}", 
                    self.pb,
                    self.pb,
                    self.pb
                ))
            },
            PORTA => {
                self.ora = value;
                // Only change the bit of PA when DDRA = 1 (output)
                self.pa = (self.pa | self.ddra) & (value | !self.ddra);
                send_cpu_msg(tx, CpuMessage::PortA(self.pa));
                log(log_file, show_log, format!(" -> PORT A: {:#010b} {:#04x} {}", 
                    self.pa,
                    self.pa,
                    self.pa
                ))
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

fn send_cpu_msg(tx: &Sender<CpuMessage>, msg: CpuMessage) {
    tx.send(msg).expect("GUI thread has hung up");
}

fn log<T: std::fmt::Display>(log_file: &mut Option<File>, show_log: &bool, msg: T) {
    let msg = msg.to_string();
    if *show_log {
        print!("{}", msg);
    }
    if let Some(log_file) = log_file {
        log_file.write(msg.as_bytes())
            .expect("Failed to write log");
    }
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