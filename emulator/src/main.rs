extern crate w65c02s;
extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;
extern crate chrono;
#[macro_use]
extern crate clap;
extern crate timer;
extern crate spin_sleep;
use chrono::prelude::*;
use std::path::Path;
use std::fs::{File, OpenOptions};  
use std::io::{Read, Write};
use std::sync::mpsc::{self, Sender};

#[macro_use]
pub mod logger;
pub mod system;
pub mod lcd;
pub mod gui;

use logger::{Logger, LogMessage};
use system::{DEFAULT_STEP_WAIT, ToSysMessage, PhysSystem};
use gui::ToGuiMessage;

fn main() {
    let matches = clap_app!(emulator =>
        (version: "0.5.2")
        (author: "Thorgaran <thorgaran1@gmail.com>")
        (about: "Emulate a physical w65c02s system to run, test and debug assembly programs")
        (@arg INPUT: +required "Sets the input file to use")
        (@arg log_dir_path: -l --log +takes_value "Save the logs in a file. Takes a path to the folder the log will be put in")
        (@arg disable_lcd: -d --disablelcd "Disable the LCD screen")
    ).get_matches();

    let bin_path = Path::new(matches.value_of("INPUT").unwrap());

    let mut program: [u8; 32_768] = [0x00; 32_768];
    File::open(bin_path)
        .expect("Failed to open binary file (make sure you typed the name properly)")
        .read(&mut program)
        .expect("Failed to read binary file as a 32 768 bytes array");

    let log_file = if let Some(log_dir_path) = matches.value_of("log_dir_path") {
        let log_dir_path = Path::new(log_dir_path);
        assert!(log_dir_path.is_dir(), 
            "Invalid directory path: {}", 
            log_dir_path.display()
        );
        
        let time_str = Local::now().format("%Y-%m-%d_%Hh%Mm%Ss");
        let mut log_file = OpenOptions::new()
            .append(true)
            .create_new(true)
            .open(format!("{}/log_{}.txt", log_dir_path.display(), time_str))
            .expect("Unable to create file");

        log_file.write(format!("Bin file: \"{}\"\n", 
            bin_path.file_name().unwrap().to_str().unwrap()
        ).as_bytes()).expect("Failed to write file name in log");

        Some(log_file)
    } else {
        None
    };

    let lcd_enabled = if matches.is_present("disable_lcd") { false } else { true };

    let (tx_log_msgs, rx_log_msgs) = mpsc::channel();

    let logger = Logger::new(log_file, rx_log_msgs);
    let logger_handle = logger.run();

    let (tx_sys_msgs, rx_sys_msgs) = mpsc::channel();
    let (tx_gui_msgs, rx_gui_msgs) = mpsc::channel();

    let system = PhysSystem::new(program, lcd_enabled, Sender::clone(&tx_log_msgs), 
        tx_gui_msgs, Sender::clone(&tx_sys_msgs), rx_sys_msgs);
    let system_handle = system.run();

    gui::run(tx_sys_msgs, rx_gui_msgs, String::from(bin_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
    ));

    system_handle.join().unwrap();

    tx_log_msgs.send(LogMessage::Exit).expect("Logger thread has hung up");
    logger_handle.join().unwrap();
}
