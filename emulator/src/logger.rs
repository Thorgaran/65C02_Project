use std::thread::{self, JoinHandle};
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::Receiver;

#[macro_export]
macro_rules! log {
    ($tx:expr, $msg:expr $(,)?) => ({ 
        $tx.send(LogMessage::Log(String::from($msg))).expect("Logger thread has hung up");
    });
    ($tx:expr, $fmt:expr, $($arg:tt)+) => ({
        $tx.send(LogMessage::Log(format!($fmt, $($arg)+)))
            .expect("Logger thread has hung up");
    });
}

pub enum LogMessage {
    Log(String),
    ChangePrintLog(bool),
    Exit,
}

pub struct Logger {
    log_file: Option<File>,
    print_log: bool,
    rx_log_msgs: Receiver<LogMessage>,
}

impl Logger {
    pub fn new(log_file: Option<File>, rx_log_msgs: Receiver<LogMessage>) -> Logger {
        Logger {
            log_file,
            print_log: false,
            rx_log_msgs,
        }
    }

    pub fn run(mut self) -> JoinHandle<()> {
        thread::Builder::new().name("Logger thread".to_string()).spawn(move || {
            'logger_thread_main: loop {
                match self.rx_log_msgs.recv().expect("Every thread able to log has hung up") {
                    LogMessage::Log(msg) => {
                        let msg = msg.to_string();

                        if self.print_log {
                            print!("{}", msg);
                        }

                        if let Some(log_file) = &mut self.log_file {
                            log_file.write(msg.as_bytes())
                                .expect("Failed to write log");
                        }
                    },
                    LogMessage::ChangePrintLog(print_log) => self.print_log = print_log,
                    LogMessage::Exit => break 'logger_thread_main,
                }
            }
        }).unwrap()
    }
}
