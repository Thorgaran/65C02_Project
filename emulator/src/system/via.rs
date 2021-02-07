pub trait ViaSystem {
    // Only the input bits matter
    fn read_port_b(&mut self, via: &mut W65C22S) -> u8;

    // This read gets the PA value even when the bit is set as an output
    fn read_port_a(&mut self, via: &mut W65C22S) -> u8;
    
    /// Called by the VIA to change the level of an output port B bit, where `bit` is that bit's position 
    /// as a number between 0 (least significant bit) and 7 (most significant bit).
    /// 
    /// `level` is the new electrical level of that bit, `true` being positive voltage,
    /// and `false` being ground.
    fn write_port_b(&mut self, via: &mut W65C22S, bit: u8, level: bool);

    /// Called by the VIA to change the level of an output port A bit, where `bit` is that bit's position 
    /// as a number between 0 (least significant bit) and 7 (most significant bit).
    /// 
    /// `level` is the new electrical level of that bit, `true` being positive voltage,
    /// and `false` being ground.
    fn write_port_a(&mut self, via: &mut W65C22S, bit: u8, level: bool);

    /// Called by the VIA to change the level of the CB2 pin.
    fn write_cb2(&mut self, via: &mut W65C22S, level: bool);

    // fn read_cb1(&mut self, via: &mut W65C22S) -> bool;
    
    /// Receive an update from the `IRQB` pin.
    /// The value recieved is the *logical* one, not the *electrical* one!
    /// Thus, `true` means some interrupt is pending, while `false` means no interrupt pending.
    /// That way, when used with the crate `w65c02s`, 
    /// irq can simply be forwarded to the `set_irq` function of the processor.
    /// 
    /// Also, when using multiple devices that can trigger an interrupt,
    /// this IRQ value must be logically ORed with that of other devices
    /// before sending it to the processor.
    fn update_irq(&mut self, via: &mut W65C22S, irq: bool);
}

// fn step(&mut self (W65C22S), chip_select: bool) {}

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
const PCR: u8 = 0xc;
const IFR: u8 = 0xd;
const IER: u8 = 0xe;

#[derive(Clone, Copy)]
pub struct W65C22S {
    ddra: u8,
    ddrb: u8,
    ora: u8,
    orb: u8,
    ira: u8,
    irb: u8,
    cb1: bool,
    cb2: bool,
    ca1: bool,
    ca2: bool,
    t1_l: u16,
    t1_c: u16,
    t1_is_running: bool, // Maybe it's actually running all the time and there's an interrupt disable flag instead, to test
    t2_l: u8,
    t2_c: u16,
    t2_trigger_interrupt: bool,
    acr: u8,
    pcr: u8,
    ifr: u8,
    ier: u8,
}

impl W65C22S {
    pub fn new() -> W65C22S {
        // Default values tested on hardware
        // Except PORT B (= ORB) before sending it data, and ira/irb when input latching (prob garbage)
        W65C22S {
            ddra: 0x00,
            ddrb: 0x00,
            ora: 0x00,
            orb: 0x00,
            ira: rand::random(),
            irb: rand::random(),
            cb2: false, // Initial value unknown, to test if it's really floating
            cb1: false, // Initial value unknown, to test
            ca1: false, // Initial value unknown, to test
            ca2: false, // Initial value unknown, to test
            t1_l: 0xbaaa, // This one is weird, the value didn't change on 5 different occasions, to try again
            t1_c: rand::random(), // Test what's in there multiple times in a row and see if it changes
            t1_is_running: false,
            t2_l: 0x00, // Actually unknown, to test by starting the timer and reading the low byte
            t2_c: rand::random(), // Same as t1_c
            t2_trigger_interrupt: false,
            acr: 0x00,
            pcr: 0x00,
            ifr: 0x00,
            ier: 0x00,
        }
    }

    /// To call on PHI2 falling edge, *before* calling `read` or `write`
    pub fn clock_pulse<S: ViaSystem>(&mut self, via_system: &mut S) {
        // Input latching
        self.irb = via_system.read_port_b(self);
        self.ira = via_system.read_port_a(self);

        // T1 operation
        if self.t1_is_running {
            self.t1_c -= 1;

            if self.t1_c == 0 {
                self.change_interrupt_flag(via_system, Some(6), true);
                
                // Restart the timer if in free-run mode
                if self.acr & 0b0100_0000 != 0 {
                    self.t1_c = self.t1_l;
                } else {
                    self.t1_is_running = false;
                }

                // TEST WHAT THE "SINGLE NEGATIVE PULSE" EXACTLY IS
                // ACTUALLY, THOROUGHLY TEST THE EFFECT OF THE TIMER ON PB7 (WHAT HAPPENS WHEN PB7 IS AN INPUT?)
                // ALSO TEST WHAT'S IN ORB AFTER THAT
                if self.acr & 0b1000_0000 != 0 {
                    // Invert PB7
                    self.orb ^= 0b1000_0000;
                    via_system.write_port_b(self, 7, self.orb & 0b1000_0000 != 0);
                }
            }
        }

        // T2 operation
        if self.acr & 0b0010_0000 == 0 || self.irb & 0b0100_0000 == 0 {
            self.t2_c = self.t2_c.wrapping_sub(1);

            if self.t2_c == 0 && self.t2_trigger_interrupt {
                self.change_interrupt_flag(via_system, Some(5), true);

                self.t2_trigger_interrupt = false;
            }
        }
    }

    /// Ask the VIA for data at the specified register.
    /// 
    /// Note: this function will panic if the upper 4 bits of register_select are non-zero.
    pub fn read<S: ViaSystem>(&mut self, via_system: &mut S, register_select: u8) -> u8 {
        match register_select {
            PORTB => if self.acr & 0b0000_0010 == 0 { // If input latching is disabled
                // Read PB when DDRB = 0 (input), read ORB otherwise
                (self.orb & self.ddrb) | (via_system.read_port_b(self) & !self.ddrb)
            } else {
                // Read IRB when DDRB = 0 (input), read ORB otherwise
                (self.orb & self.ddrb) | (self.irb & !self.ddrb)
            },
            PORTA => if self.acr & 0b0000_0001 == 0 { // If input latching is disabled
                via_system.read_port_a(self)
            } else {
                self.ira
            },
            DDRB => self.ddrb,
            DDRA => self.ddra,
            T1C_L => {
                // "T1 interrupt flag IFR6 is reset"
                self.change_interrupt_flag(via_system, Some(6), false);
                // "8 bits from T1 low order counter transferred to MPU"
                (self.t1_c & 0x00ff) as u8
            },
            T1C_H => ((self.t1_c & 0xff00) >> 8) as u8,
            T1L_L => (self.t1_l & 0x00ff) as u8,
            T1L_H => ((self.t1_l & 0xff00) >> 8) as u8,
            T2C_L => {
                // "IFR5 is reset"
                self.change_interrupt_flag(via_system, Some(5), false);
                // "8 bits from T2 low order counter transferred to MPU"
                (self.t2_c & 0x00ff) as u8
            },
            T2C_H => ((self.t2_c & 0xff00) >> 8) as u8,
            ACR => self.acr,
            PCR => self.pcr,
            IFR => self.ifr,
            IER => self.ier | 0b1000_0000,
            0x10..=0xff => panic!("Illegal Register Select value! (Expected: 0 <= RS <= 15)"),
            _ => todo!(),
        }
    }

    /// Send data to the specified VIA register.
    /// 
    /// Note: this function will panic if the upper 4 bits of register_select are non-zero.
    pub fn write<S: ViaSystem>(&mut self, via_system: &mut S, register_select: u8, data: u8) {
        match register_select {
            PORTB => {
                self.orb = data;
                // Only change the bit of PB when DDRB = 1 (output)
                for i in 0..=7 {
                    if (self.ddrb >> i) & 1 == 1 {
                        via_system.write_port_b(self, i, (data >> i) & 1 == 1);
                    }
                }
            },
            PORTA => {
                self.ora = data;
                // Only change the bit of PA when DDRA = 1 (output)
                for i in 0..=7 {
                    if (self.ddra >> i) & 1 == 1 {
                        via_system.write_port_a(self, i, (data >> i) & 1 == 1);
                    }
                }
            },
            DDRB => {
                // Update PB when a DDRB bit goes from 0 (input) to 1 (output)
                for i in 0..=7 {
                    if (self.ddrb >> i) & 1 == 0 && (data >> i) & 1 == 1 {
                        via_system.write_port_b(self, i, (self.orb >> i) & 1 == 1);
                    }
                }
                self.ddrb = data;
            },
            DDRA => {
                // Update PA when a DDRA bit goes from 0 (input) to 1 (output)
                for i in 0..=7 {
                    if (self.ddra >> i) & 1 == 0 && (data >> i) & 1 == 1 {
                        via_system.write_port_a(self, i, (self.ora >> i) & 1 == 1);
                    }
                }
                self.ddra = data;
            },
            T1C_H => {
                // "8 bits loaded into T1 high order latches"
                self.t1_l = (self.t1_l & 0x00ff) | ((data as u16) << 8);
                // "Also, both high and low order latches are transferred 
                // into T1 counter and this initiates countdown"
                self.t1_c = self.t1_l;
                self.t1_is_running = true;
                // "T1 interrupt flag IFR6 is reset"
                self.change_interrupt_flag(via_system, Some(6), false);
            },
            T1C_L | T1L_L => {
                // "8 bits loaded into T1 low order latches"
                self.t1_l = (self.t1_l & 0xff00) | data as u16;
            },
            T1L_H => {
                // "8 bits loaded into T1 high order latches"
                self.t1_l = ((data as u16) << 8) | (self.t1_l & 0x00ff);
                // "T1 interrupt flag IFR6 is reset"
                self.change_interrupt_flag(via_system, Some(6), false);
            },
            T2C_L => {
                // "8 bits loaded into T2 low order latches"
                self.t2_l = data;
            },
            T2C_H => {
                // "8 bits loaded into T2 high order counter.
                // Also, low order latches are transferred to low order counter"
                self.t2_c = ((data as u16) << 8) | self.t2_l as u16;
                // "IFR5 is reset"
                self.change_interrupt_flag(via_system, Some(5), false);
                self.t2_trigger_interrupt = true;
            },
            ACR => self.acr = data,
            PCR => {
                self.pcr = data;
                
                // If CB2 is in output mode
                if data >> 7 == 1 {
                    if data >> 5 == 0b110 {
                        via_system.write_cb2(self, false);
                    } else {
                        via_system.write_cb2(self, true);
                    }
                }
            },
            IFR => {
                self.ifr = self.ifr & !data;
                self.change_interrupt_flag(via_system, None, true);
            },
            IER => {
                self.ier = match data & 0b1000_0000 {
                    0 => self.ier & !data,
                    _ => self.ier | data,
                };
                self.change_interrupt_flag(via_system, None, true);
            },
            0x10..=0xff => panic!("Illegal Register Select value! (Expected: 0 <= RS <= 15)"),
            _ => todo!(),
        }
    }

    // Updates the status of IRQB, and if <flag> is Some(u32),
    // sets IFRx to <logic_level>, where x is a <flag> number between 0 and 6 
    fn change_interrupt_flag<S: ViaSystem>(&mut self, 
        via_system: &mut S, 
        flag: Option<u32>, 
        logic_level: bool
    ) {
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
                via_system.update_irq(self, false);
                0b0000_0000
            },
            _ => {
                via_system.update_irq(self, true);
                0b1000_0000
            },
        };
        self.ifr = (self.ifr & 0b0111_1111) | irq;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use std::sync::mpsc::{self, Receiver};
    use crate::LogMessage;
    use crate::system::PhysSystem;

    fn create_test_sys() -> (PhysSystem, Receiver<LogMessage>) {
        let (tx_log_msgs, rx_log_msgs) = mpsc::channel();
        (PhysSystem {
            tx_log_msgs,
            ..Default::default() 
        }, rx_log_msgs)
    }

    #[test]
    fn simple_write_pb() {
        let (mut sys, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via;

        via.write(&mut sys, DDRB, 0b1111_1111);
        via.write(&mut sys, PORTB, 0x42);

        assert_eq!(0x42, sys.via_pb);
    }

    #[test]
    fn complex_write_pb() {
        let (mut sys, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via;
        
        via.write(&mut sys, PORTB, 0b1100_0011);
        
        via.write(&mut sys, DDRB, 0b0110_1001);
        via.write(&mut sys, PORTB, 0b1010_0111);
        
        assert_eq!(0b0010_0001, sys.via_pb & 0b0110_1001);
    }

    #[test]
    fn change_ddrb() {
        let (mut sys, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via;
        
        via.write(&mut sys, DDRB, 0xff);
        via.write(&mut sys, PORTB, 0xc3);

        assert_eq!(0xc3, sys.via_pb);

        via.write(&mut sys, DDRB, 0x00);
        via.write(&mut sys, PORTB, 0x42);

        assert_eq!(0xc3, sys.via_pb);

        via.write(&mut sys, DDRB, 0x0f);

        assert_eq!(0xc2, sys.via_pb);

        via.write(&mut sys, DDRB, 0xff);

        assert_eq!(0x42, sys.via_pb);
    }

    #[test]
    fn t1_one_shot() {
        let (mut sys, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via;
        
        // PB7 disabled, one-shot mode
        via.write(&mut sys, ACR, 0b0000_0011);
        // Set IER6
        via.write(&mut sys, IER, 0b1100_0000);

        via.write(&mut sys, T1L_L, 0x37);
        via.write(&mut sys, T1L_H, 0x13);
        assert_eq!(0x37, via.read(&mut sys, T1L_L));
        assert_eq!(0x13, via.read(&mut sys, T1L_H));

        via.write(&mut sys, T1C_L, 0x02);
        via.write(&mut sys, T1C_H, 0x00);

        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        via.clock_pulse(&mut sys);
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        via.clock_pulse(&mut sys);
        assert_eq!(0b1100_0000, via.read(&mut sys, IFR), "IFR6 should be set");
        
        via.clock_pulse(&mut sys);
        assert_eq!(0x00, via.read(&mut sys, T1C_L), "Timer shouldn't be running anymore");
        assert_eq!(0x00, via.read(&mut sys, T1C_H), "Timer shouldn't be running anymore");
    }

    #[test]
    fn t1_os_long_timer() {
        let (mut sys, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via;
        
        // PB7 disabled, one-shot mode
        via.write(&mut sys, ACR, 0b0000_0011);
        // Set IER6
        via.write(&mut sys, IER, 0b1100_0000);

        via.write(&mut sys, T1C_L, (500u16 & 0x00ff) as u8);
        via.write(&mut sys, T1C_H, ((500u16 & 0xff00) >> 8) as u8);
        
        for _i in 0..500 {
            assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
            via.clock_pulse(&mut sys);
        }
        assert_eq!(0b1100_0000, via.read(&mut sys, IFR), "IFR6 should be set");
        
        via.clock_pulse(&mut sys);
        assert_eq!(0x00, via.read(&mut sys, T1C_L), "Timer shouldn't be running anymore");
        assert_eq!(0x00, via.read(&mut sys, T1C_H), "Timer shouldn't be running anymore");
    }
        
    #[test]
    fn t1_freerun() {
        let (mut sys, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via;

        // PB7 disabled, freerun mode
        via.write(&mut sys, ACR, 0b0100_0011);
        // Set IER6
        via.write(&mut sys, IER, 0b1100_0000);

        via.write(&mut sys, T1C_L, 0x02);
        via.write(&mut sys, T1C_H, 0x00);

        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        via.clock_pulse(&mut sys);
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        via.clock_pulse(&mut sys);
        assert_eq!(0b1100_0000, via.read(&mut sys, IFR), "IFR6 should be set");

        via.clock_pulse(&mut sys);

        // Clear IFR6
        via.write(&mut sys, IFR, 0b0100_0000);
        assert_eq!(0x00, via.read(&mut sys, IFR), "Interrupt flags should have been reset");

        via.clock_pulse(&mut sys);
        assert_eq!(0b1100_0000, via.read(&mut sys, IFR), "IFR6 should be set");
    }

    #[test]
    fn t2_one_shot() {
        let (mut sys, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via;
        
        // One-shot mode
        via.write(&mut sys, ACR, 0b0000_0011);
        // Set IER5
        via.write(&mut sys, IER, 0b1010_0000);

        via.write(&mut sys, T2C_L, 0x02);
        via.write(&mut sys, T2C_H, 0x00);

        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        via.clock_pulse(&mut sys);
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        via.clock_pulse(&mut sys);
        assert_eq!(0b1010_0000, via.read(&mut sys, IFR), "IFR5 should be set");
        
        via.clock_pulse(&mut sys);
        assert_eq!(0b1010_0000, via.read(&mut sys, IFR), "IFR5 should still be set");
        
        assert_eq!(0xff, via.read(&mut sys, T2C_L), "Timer should still be running");
        assert_eq!(0xff, via.read(&mut sys, T2C_H), "Timer should still be running");
        assert_eq!(0b0000_0000, via.read(&mut sys, IFR), "IFR5 should be reset");
        
        via.clock_pulse(&mut sys);
        assert_eq!(0xfe, via.read(&mut sys, T2C_L), "Timer should still be running");
        assert_eq!(0xff, via.read(&mut sys, T2C_H), "Timer should still be running");
    }

    #[test]
    fn t2_full_cycle() {
        let (mut sys, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via;
        
        // One-shot mode
        via.write(&mut sys, ACR, 0b0000_0011);
        // Set IER5
        via.write(&mut sys, IER, 0b1010_0000);

        via.write(&mut sys, T2C_L, 0x02);
        via.write(&mut sys, T2C_H, 0x00);

        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        via.clock_pulse(&mut sys);
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        via.clock_pulse(&mut sys);
        assert_eq!(0b1010_0000, via.read(&mut sys, IFR), "IFR5 should be set");
        
        // Clear IFR5
        via.write(&mut sys, IFR, 0b0010_0000);
        assert_eq!(0b0000_0000, via.read(&mut sys, IFR), "IFR5 should be reset");
        
        for _i in 0..65_539 {
            assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set");
            via.clock_pulse(&mut sys);
        }
        assert_eq!(0xfd, via.read(&mut sys, T2C_L), "Timer should still be running");
        assert_eq!(0xff, via.read(&mut sys, T2C_H), "Timer should still be running");
    }

    #[test]
    fn t2_pulse_count() {
        todo!("Fix pulse count test");

        let (mut sys, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via;

        via.write(&mut sys, PORTB, 0xff);

        // Pulse counting mode
        via.write(&mut sys, ACR, 0b0010_0011);
        // Set IER5
        via.write(&mut sys, IER, 0b1010_0000);

        via.write(&mut sys, T2C_L, 0x03);
        via.write(&mut sys, T2C_H, 0x00);

        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        via.clock_pulse(&mut sys);
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        via.clock_pulse(&mut sys);
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        
        via.orb = via.irb & 0b1011_1111; //ALLOW WRITE OF A SINGLE BIT...
        via.clock_pulse(&mut sys);
        via.clock_pulse(&mut sys);
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        
        via.orb = via.irb | 0b0100_0000;
        via.clock_pulse(&mut sys);
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");

        via.orb = via.irb & 0b1011_1111;
        via.clock_pulse(&mut sys);
        assert_eq!(0b1010_0000, via.read(&mut sys, IFR), "IFR5 should be set");

        via.clock_pulse(&mut sys);
        via.clock_pulse(&mut sys);
        assert_eq!(0xfe, via.read(&mut sys, T2C_L), "Timer should still be running");
        assert_eq!(0xff, via.read(&mut sys, T2C_H), "Timer should still be running");
    }


    #[test]
    fn interrupts() {
        let (mut sys, _rx_log_msgs) = create_test_sys();
        let mut via = sys.via;

        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupts should be enabled yet");
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");
        
        via.change_interrupt_flag(&mut sys, None, true);
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");

        via.change_interrupt_flag(&mut sys, Some(0), false);
        assert_eq!(0x00, via.read(&mut sys, IFR), "No interrupt flags should be set yet");

        via.change_interrupt_flag(&mut sys, Some(0), true);
        via.change_interrupt_flag(&mut sys, Some(5), true);
        via.change_interrupt_flag(&mut sys, Some(6), true);
        assert_eq!(0b0110_0001, via.read(&mut sys, IFR), "IFR6, IFR5 and IFR0 should be set");

        via.write(&mut sys, IER, 0b1000_0010);
        assert_eq!(0b0110_0001, via.read(&mut sys, IFR), "IFR7 should still be reset");
        
        via.write(&mut sys, IER, 0b1000_1001);
        assert_eq!(0b1000_1011, via.read(&mut sys, IER));
        assert_eq!(0b1110_0001, via.read(&mut sys, IFR), "IFR7 should be set");
        
        via.write(&mut sys, IFR, 0b1000_0000);
        assert_eq!(0b1110_0001, via.read(&mut sys, IFR), "Writing to IFR7 shouldn't clear it");
        
        via.write(&mut sys, IFR, 0b0100_1000);
        assert_eq!(0b1010_0001, via.read(&mut sys, IFR), "IFR6 should be cleared");
        
        via.write(&mut sys, IFR, 0b1001_0001);
        assert_eq!(0b0010_0000, via.read(&mut sys, IFR), "IFR7 should be cleared");
        
        via.change_interrupt_flag(&mut sys, Some(3), true);
        assert_eq!(0b1010_1000, via.read(&mut sys, IFR), "IFR7 should be set");

        via.write(&mut sys, IER, 0b0010_1001);
        assert_eq!(0b1000_0010, via.read(&mut sys, IER));
        assert_eq!(0b0010_1000, via.read(&mut sys, IFR), "IFR7 should be reset");
    }
}
