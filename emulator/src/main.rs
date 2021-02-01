extern crate w65c02s;
extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;
extern crate chrono;
#[macro_use]
extern crate clap;
extern crate timer;
use chrono::prelude::*;
use std::path::Path;
use std::fs::{File, OpenOptions};  
use std::io::{Read, Write};
use std::sync::mpsc::{self, Sender};

#[macro_use]
pub mod logger;
pub mod processor;
pub mod lcd;
pub mod gui;

use logger::{Logger, LogMessage};
use processor::PhysSystem;

// Default waiting time between steps when running, in milliseconds
const DEFAULT_STEP_WAIT: usize = 50;

pub enum GuiToCpuMessage {
    Run,
    Stop,
    Step,
    ChangeWaitTime(usize),
    ShowLog(bool),
    Exit,
}

pub enum ToGuiMessage {
    PortB(u8),
    PortA(u8),
    CycleCount(usize),
    LcdScreen(String),
    Stopped,
}

fn main() {
    let matches = clap_app!(emulator =>
        (version: "0.1.0")
        (author: "Thorgaran <thorgaran1@gmail.com>")
        (about: "Emulate a physical w65c02s system to run, test and debug assembly programs")
        (@arg INPUT: +required "Sets the input file to use")
        (@arg log: -l --log "Save the logs in a file")
    ).get_matches();
    
    // The goal of this is to set the current working directory to emulator/
    {
        use std::{env, ffi::OsStr};

        let current_exe = env::current_exe().expect("Unable to read current executable path");
        let mut path = current_exe.parent().expect("Unable to get executable parent");
        
        while path.file_name() != Some(OsStr::new("emulator")) {
            path = path.parent().expect("Failed to get to the emulator dir");
        }

        env::set_current_dir(&path).expect("Unable to set the current working dir");
    }

    let bin_path = Path::new(matches.value_of("INPUT").unwrap());

    let mut program: [u8; 32_768] = [0x00; 32_768];
    File::open(bin_path)
        .expect("Failed to open binary file (make sure you typed the name properly)")
        .read(&mut program)
        .expect("Failed to read binary file as a 32 768 bytes array");

    let log_file = if matches.is_present("log") {
        let time_str = Local::now().format("%Y-%m-%d_%Hh%Mm%Ss");
        let mut log_file = OpenOptions::new()
            .append(true)
            .create_new(true)
            .open(format!("logs/log_{}.txt", time_str))
            .expect("Unable to create file");
        log_file.write(format!("Bin file: \"{}\"\n", 
            bin_path.file_name().unwrap().to_str().unwrap()
        ).as_bytes()).expect("Failed to write file name in log");
        Some(log_file)
    } else {
        None
    };

    let (tx_log_msgs, rx_log_msgs) = mpsc::channel();

    let logger = Logger::new(log_file, rx_log_msgs);
    let logger_handle = logger.run();

    let (tx_gui_msgs, rx_gui_msgs) = mpsc::channel();
    let (tx_cpu_msgs, rx_cpu_msgs) = mpsc::channel();

    let system = PhysSystem::new(program, Sender::clone(&tx_log_msgs), tx_cpu_msgs, rx_gui_msgs);
    let system_handle = system.run();

    gui::run(tx_gui_msgs, rx_cpu_msgs, String::from(bin_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
    ));

    system_handle.join().unwrap();

    tx_log_msgs.send(LogMessage::Exit).expect("Logger thread has hung up");
    logger_handle.join().unwrap();
}
