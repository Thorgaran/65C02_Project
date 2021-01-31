extern crate w65c02s;
extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;
extern crate chrono;
use chrono::prelude::*;
use std::fs::{File, OpenOptions};  
use std::io::{self, Read, Write};
use std::sync::mpsc;

pub mod processor;
pub mod gui;

// Default waiting time between steps when running, in milliseconds
const DEFAULT_STEP_WAIT: usize = 500;

pub enum GuiMessage {
    Run,
    Stop,
    Step,
    ChangeWaitTime(usize),
    ShowLog(bool),
}

pub enum CpuMessage {
    PortB(u8),
    PortA(u8),
}

fn main() {
    println!("Enter the name of the binary file to execute (without the .bin):");
    let mut bin_name = String::new();
    io::stdin()
            .read_line(&mut bin_name)
            .expect("Failed to read line");

    let bin_name = bin_name.trim();
    let bin_path = format!("../Binaries/{}.bin", bin_name);

    let mut program: [u8; 32_768] = [0x00; 32_768];
    File::open(bin_path)
        .expect("Failed to open binary file (make sure you typed the name properly)")
        .read(&mut program)
        .expect("Failed to read binary file as a 32 768 bytes array");

    println!("Do you want to save the log in a file? [y/N]");
    let mut save_log = String::new();
    io::stdin()
            .read_line(&mut save_log)
            .expect("Failed to read line");

    let log_file = match save_log.trim() {
        "y" | "Y" => {
            let time_str = Local::now().format("%Y-%m-%d_%Hh%Mm%Ss");
            let mut log_file = OpenOptions::new()
                .append(true)
                .create_new(true)
                .open(format!("logs/log_{}.txt", time_str))
                .expect("Unable to create file");
            log_file.write(format!("Bin file: \"{}.bin\"\n", bin_name).as_bytes())
                .expect("Failed to write file name in log");
            Some(log_file)
        },
        _ => None,
    };

    let (tx_gui_msgs, rx_gui_msgs) = mpsc::channel();
    let (tx_cpu_msgs, rx_cpu_msgs) = mpsc::channel();

    let system = processor::PhysSystem::new(program, log_file, tx_cpu_msgs, rx_gui_msgs);

    system.run();

    gui::run(tx_gui_msgs, rx_cpu_msgs, String::from(bin_name));
}
