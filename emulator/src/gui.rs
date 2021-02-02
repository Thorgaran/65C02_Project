use nwg::NativeUi;
use nwd::NwgUi;
use std::sync::mpsc::{self, Sender, Receiver, TryRecvError};
use std::cell::RefCell;
use crate::{ToSysMessage, DEFAULT_STEP_WAIT};

pub enum ToGuiMessage {
    PortB(u8),
    PortA(u8),
    CycleCount(usize),
    LcdScreen(String),
    Paused,
    Stopped,
}

struct UIChannels {
    tx: Sender<ToSysMessage>,
    rx: Receiver<ToGuiMessage>,
}

impl Default for UIChannels {
    fn default() -> Self {
        // This default code shouldn't be used
        let (tx, _) = mpsc::channel();
        let (_, rx) = mpsc::channel();
        UIChannels { tx, rx }
    }
}

struct WaitTime {
    time: usize,
    unit: WaitTimeUnit,
}

#[derive(Clone)]
enum WaitTimeUnit {
    Milli,
    TensOfMicro,
}

impl WaitTime {
    fn to_micro(&self) -> usize {
        match self.unit {
            WaitTimeUnit::Milli => self.time * 1000,
            WaitTimeUnit::TensOfMicro => self.time * 10,
        }
    }
}

use std::fmt;
impl fmt::Display for WaitTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.unit {
            WaitTimeUnit::Milli => write!(f, "{}ms", self.time),
            WaitTimeUnit::TensOfMicro => write!(f, "{}ms", (self.time as f64) / 100.0),
        }
    }
}

impl Default for WaitTime {
    fn default() -> Self {
        WaitTime {
            time: DEFAULT_STEP_WAIT,
            unit: WaitTimeUnit::Milli,
        }
    }
}

#[derive(Default)]
struct UIData {
    bin_name: String,
    cur_wait_time: WaitTime,
    sys_running: bool,
}

#[derive(Default, NwgUi)]
pub struct EmulatorGui {
    channels: UIChannels,
    data: RefCell<UIData>,

    #[nwg_control(size: (600, 200), position: (300, 300), 
        title: "65C02_Project Emulator GUI", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnInit: [EmulatorGui::init], 
        OnWindowClose: [EmulatorGui::exit] )]
    window: nwg::Window,
    
    // Refresh SYS data (~30 FPS)
    #[nwg_control(parent: window, interval: 33, stopped: false)]
    #[nwg_events(OnTimerTick: [EmulatorGui::listen_gui_msgs])]
    refresh_timer: nwg::Timer,

    #[nwg_layout(parent: window, spacing: 1)]
    grid: nwg::GridLayout,

    #[nwg_control(text: "")]
    #[nwg_layout_item(layout: grid, row: 0, col: 0, col_span: 6)]
    bin_name_lbl: nwg::Label,

    #[nwg_control(text: "Run", check_state: RadioButtonState::Unchecked,
        flags: "VISIBLE|GROUP")]
    #[nwg_layout_item(layout: grid, row: 1, col: 0, col_span: 2)]
    #[nwg_events( OnButtonClick: [EmulatorGui::send_run] )]
    run_rbutton: nwg::RadioButton,

    #[nwg_control(text: "Stop", check_state: RadioButtonState::Checked)]
    #[nwg_layout_item(layout: grid, row: 1, col: 2, col_span: 2)]
    #[nwg_events( OnButtonClick: [EmulatorGui::send_stop] )]
    stop_rbutton: nwg::RadioButton,

    #[nwg_control(text: "Step")]
    #[nwg_layout_item(layout: grid, row: 1, col: 4, col_span: 2, focus: true)]
    #[nwg_events( OnButtonClick: [EmulatorGui::send_step] )]
    step_button: nwg::Button,

    #[nwg_control(text: &format!(
        "Wait time between steps:\nCurrent:  {}ms\nSelected: {}ms",
        DEFAULT_STEP_WAIT,
        DEFAULT_STEP_WAIT
    ))]
    #[nwg_layout_item(layout: grid, row: 2, col: 0, row_span: 2, col_span: 4)]
    step_wait_time_lbl: nwg::Label,

    #[nwg_control(text: "µs (x10)", check_state: RadioButtonState::Unchecked,
        flags: "VISIBLE|GROUP")]
    #[nwg_layout_item(layout: grid, row: 2, col: 4, col_span: 2)]
    #[nwg_events( OnButtonClick: [EmulatorGui::set_wait_time_micro] )]
    wait_time_micro_rbutton: nwg::RadioButton,

    #[nwg_control(text: "ms", check_state: RadioButtonState::Checked)]
    #[nwg_layout_item(layout: grid, row: 3, col: 4)]
    #[nwg_events( OnButtonClick: [EmulatorGui::set_wait_time_milli] )]
    wait_time_milli_rbutton: nwg::RadioButton,

    #[nwg_control(text: "✓")]
    #[nwg_layout_item(layout: grid, row: 3, col: 5)]
    #[nwg_events( OnButtonClick: [EmulatorGui::send_wait_time] )]
    wait_time_button: nwg::Button,

    #[nwg_control(flags: "TICK_TOP|VISIBLE")]
    #[nwg_layout_item(layout: grid, row: 4, col: 0, col_span: 6)]
    #[nwg_events( OnHorizontalScroll: [EmulatorGui::step_wait_time_tb_change] )]
    step_wait_time_tb: nwg::TrackBar,

    #[nwg_resource(family: "Segoe UI", size: 16)]
    segoe_small: nwg::Font,

    #[nwg_control(text: "Show log in console", font: Some(&data.segoe_small),
        check_state: nwg::CheckBoxState::Unchecked)]
    #[nwg_layout_item(layout: grid, row: 5, col: 0, col_span: 3)]
    #[nwg_events( OnButtonClick: [EmulatorGui::send_print_log] )]
    print_log_cbox: nwg::CheckBox,

    #[nwg_control(text: "Cycles: 0")]
    #[nwg_layout_item(layout: grid, row: 5, col: 3, col_span: 3)]
    cycle_count_lbl: nwg::Label,

    #[nwg_control()]
    #[nwg_layout_item(layout: grid, row: 0, col: 6, row_span: 6, col_span: 6)]
    tabs_container: nwg::TabsContainer,

    #[nwg_control(parent: tabs_container, text: "Leds")]
    tab_leds: nwg::Tab,

    #[nwg_control(parent: tabs_container, text: "LCD")]
    tab_lcd: nwg::Tab,
    
    #[nwg_resource(source_bin: Some(include_bytes!("../imgs/led_on.bmp")))]
    led_on_bmp: nwg::Bitmap,

    #[nwg_resource(source_bin: Some(include_bytes!("../imgs/led_off.bmp")))]
    led_off_bmp: nwg::Bitmap,

    #[nwg_layout(parent: tab_leds, spacing: 1)]
    led_grid: nwg::GridLayout,

    #[nwg_control(parent: tab_leds, text: "Port B: 0x00 (0)")]
    #[nwg_layout_item(layout: led_grid, row: 0, col: 0)]
    port_b_lbl: nwg::Label,

    #[nwg_control(parent: tab_leds)]
    #[nwg_layout_item(layout: led_grid, row: 1, col: 0, row_span: 2, col_span: 2)]
    port_b_frame: nwg::Frame,

    #[nwg_layout(parent: port_b_frame, spacing: 1)]
    port_b_grid: nwg::GridLayout,

    #[nwg_control(parent: port_b_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_b_grid, row: 0, col: 0)]
    led7b: nwg::ImageFrame,

    #[nwg_control(parent: port_b_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_b_grid, row: 0, col: 1)]
    led6b: nwg::ImageFrame,

    #[nwg_control(parent: port_b_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_b_grid, row: 0, col: 2)]
    led5b: nwg::ImageFrame,

    #[nwg_control(parent: port_b_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_b_grid, row: 0, col: 3)]
    led4b: nwg::ImageFrame,

    #[nwg_control(parent: port_b_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_b_grid, row: 0, col: 4)]
    led3b: nwg::ImageFrame,

    #[nwg_control(parent: port_b_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_b_grid, row: 0, col: 5)]
    led2b: nwg::ImageFrame,

    #[nwg_control(parent: port_b_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_b_grid, row: 0, col: 6)]
    led1b: nwg::ImageFrame,

    #[nwg_control(parent: port_b_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_b_grid, row: 0, col: 7)]
    led0b: nwg::ImageFrame,

    #[nwg_control(parent: tab_leds, text: "Port A: 0x00 (0)")]
    #[nwg_layout_item(layout: led_grid, row: 3, col: 0)]
    port_a_lbl: nwg::Label,

    #[nwg_control(parent: tab_leds, text: "Use as breakpoint", font: Some(&data.segoe_small),
        check_state: nwg::CheckBoxState::Checked)]
    #[nwg_layout_item(layout: led_grid, row: 3, col: 1)]
    port_a_breakpoint_cbox: nwg::CheckBox,

    #[nwg_control(parent: tab_leds)]
    #[nwg_layout_item(layout: led_grid, row: 4, col: 0, row_span: 2, col_span: 2)]
    #[nwg_events( OnButtonClick: [EmulatorGui::send_breakpoint] )]
    port_a_frame: nwg::Frame,

    #[nwg_layout(parent: port_a_frame, spacing: 1)]
    port_a_grid: nwg::GridLayout,

    #[nwg_control(parent: port_a_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_a_grid, row: 0, col: 0)]
    led7a: nwg::ImageFrame,

    #[nwg_control(parent: port_a_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_a_grid, row: 0, col: 1)]
    led6a: nwg::ImageFrame,

    #[nwg_control(parent: port_a_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_a_grid, row: 0, col: 2)]
    led5a: nwg::ImageFrame,

    #[nwg_control(parent: port_a_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_a_grid, row: 0, col: 3)]
    led4a: nwg::ImageFrame,

    #[nwg_control(parent: port_a_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_a_grid, row: 0, col: 4)]
    led3a: nwg::ImageFrame,

    #[nwg_control(parent: port_a_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_a_grid, row: 0, col: 5)]
    led2a: nwg::ImageFrame,

    #[nwg_control(parent: port_a_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_a_grid, row: 0, col: 6)]
    led1a: nwg::ImageFrame,

    #[nwg_control(parent: port_a_frame, bitmap: Some(&data.led_off_bmp))]
    #[nwg_layout_item(layout: port_a_grid, row: 0, col: 7)]
    led0a: nwg::ImageFrame,

    #[nwg_layout(parent: tab_lcd, spacing: 1)]
    lcd_grid: nwg::GridLayout,

    #[nwg_resource(family: "Courier New", size: 28)]
    courier_new: nwg::Font,

    #[nwg_control(parent: tab_lcd, font: Some(&data.courier_new), 
        h_align: nwg::HTextAlign::Center, text: 
        "╔════════════════╗\n║This screen     ║\n║    is disabled.║\n╚════════════════╝")]
    #[nwg_layout_item(layout: lcd_grid, row: 0, col: 0)] 
    lcd_screen_lbl: nwg::Label,
}

impl EmulatorGui {
    fn init(&self) {
        self.bin_name_lbl.set_text(&format!("Executing file: {}", self.data.borrow().bin_name));
        
        // The trackbar values cannot be set when it is created, according to the winapi docs,
        // so they are set in this init function
        self.step_wait_time_tb.set_range_min(0);
        self.step_wait_time_tb.set_range_max(1000);
        self.step_wait_time_tb.set_pos(DEFAULT_STEP_WAIT);
        
        // The above triggers a step_wait_time_tb_change, so we need to disable the button afterwards
        self.wait_time_button.set_enabled(false);
    }

    fn exit(&self) {
        if self.data.borrow().sys_running {
            self.send_gui_msg(ToSysMessage::Exit);
        }
        nwg::stop_thread_dispatch();
    }

    fn listen_gui_msgs(&self) {
        loop { match self.channels.rx.try_recv() {
            Err(TryRecvError::Disconnected) => if self.data.borrow().sys_running {
                panic!("SYS thread has hung up")
            } else {
                break;
            },
            Err(TryRecvError::Empty) => break,
            Ok(msg) => match msg {
                ToGuiMessage::PortB(mut port_b_data) => {
                    self.port_b_lbl.set_text(&format!("Port B: {:#04x} ({})",
                        &port_b_data,
                        &port_b_data
                    ));

                    let mut bitmaps = vec![];
                    for _i in 0..8 {
                        if port_b_data & 0b0000_0001 == 0 {
                            bitmaps.push(Some(&self.led_off_bmp));
                        } else {
                            bitmaps.push(Some(&self.led_on_bmp));
                        }
                        port_b_data >>= 1;
                    }
                    self.led7b.set_bitmap(bitmaps.pop().unwrap());
                    self.led6b.set_bitmap(bitmaps.pop().unwrap());
                    self.led5b.set_bitmap(bitmaps.pop().unwrap());
                    self.led4b.set_bitmap(bitmaps.pop().unwrap());
                    self.led3b.set_bitmap(bitmaps.pop().unwrap());
                    self.led2b.set_bitmap(bitmaps.pop().unwrap());
                    self.led1b.set_bitmap(bitmaps.pop().unwrap());
                    self.led0b.set_bitmap(bitmaps.pop().unwrap());
                },
                ToGuiMessage::PortA(mut port_a_data) => {
                    self.port_a_lbl.set_text(&format!("Port A: {:#04x} ({})",
                        &port_a_data,
                        &port_a_data
                    ));

                    let mut bitmaps = vec![];
                    for _i in 0..8 {
                        if port_a_data & 0b0000_0001 == 0 {
                            bitmaps.push(Some(&self.led_off_bmp));
                        } else {
                            bitmaps.push(Some(&self.led_on_bmp));
                        }
                        port_a_data >>= 1;
                    }
                    self.led7a.set_bitmap(bitmaps.pop().unwrap());
                    self.led6a.set_bitmap(bitmaps.pop().unwrap());
                    self.led5a.set_bitmap(bitmaps.pop().unwrap());
                    self.led4a.set_bitmap(bitmaps.pop().unwrap());
                    self.led3a.set_bitmap(bitmaps.pop().unwrap());
                    self.led2a.set_bitmap(bitmaps.pop().unwrap());
                    self.led1a.set_bitmap(bitmaps.pop().unwrap());
                    self.led0a.set_bitmap(bitmaps.pop().unwrap());
                },
                ToGuiMessage::CycleCount(cycle_count) => self.cycle_count_lbl
                    .set_text(&format!("Cycles: {}", cycle_count)),
                ToGuiMessage::LcdScreen(lcd_screen) => self.lcd_screen_lbl
                    .set_text(&lcd_screen),
                ToGuiMessage::Paused => {
                    self.run_rbutton.set_check_state(nwg::RadioButtonState::Unchecked);
                    self.stop_rbutton.set_check_state(nwg::RadioButtonState::Checked);
                    self.step_button.set_enabled(true);
                }
                ToGuiMessage::Stopped => {
                    self.refresh_timer.stop();

                    self.data.borrow_mut().sys_running = false;

                    self.run_rbutton.set_enabled(false);
                    self.stop_rbutton.set_enabled(false);
                    self.step_button.set_enabled(false);
                    self.wait_time_button.set_enabled(false);
                    self.step_wait_time_tb.set_enabled(false);
                    self.print_log_cbox.set_enabled(false);

                    nwg::modal_info_message(&self.window, "CPU stopped", 
                        "The CPU is done executing the program.\nClose the main window to exit."
                    );
                },
            },
        }};
    }

    fn send_run(&self) {
        self.step_button.set_enabled(false);
        self.send_gui_msg(ToSysMessage::Run);
    }

    fn send_stop(&self) {
        self.step_button.set_enabled(true);
        self.send_gui_msg(ToSysMessage::Stop);
    }

    fn send_step(&self) {
        self.send_gui_msg(ToSysMessage::Step);
    }

    fn update_step_wait_time_lbl(&self, cur: &WaitTime, sel: &WaitTime) {
        self.step_wait_time_lbl.set_text(&format!(
            "Wait time between steps:\nCurrent:  {}\nSelected: {}",
            cur,
            sel
        ));
    }

    fn get_next_wait_time_unit(&self) -> WaitTimeUnit {
        match self.wait_time_milli_rbutton.check_state() {
            nwg::RadioButtonState::Checked => WaitTimeUnit::Milli,
            nwg::RadioButtonState::Unchecked => WaitTimeUnit::TensOfMicro,
        }
    }

    fn step_wait_time_tb_change(&self) {
        let data = self.data.borrow();
        let cur = &data.cur_wait_time;

        self.update_step_wait_time_lbl(cur, &WaitTime {
            time: self.step_wait_time_tb.pos(),
            unit: self.get_next_wait_time_unit(),
        });
        self.wait_time_button.set_enabled(true);
    }

    fn set_wait_time_milli(&self) {
        let data = self.data.borrow();
        let cur = &data.cur_wait_time;

        self.update_step_wait_time_lbl(cur, &WaitTime {
            time: self.step_wait_time_tb.pos(),
            unit: WaitTimeUnit::Milli,
        });
        self.wait_time_button.set_enabled(true);
    }

    fn set_wait_time_micro(&self) {
        let data = self.data.borrow();
        let cur = &data.cur_wait_time;

        self.update_step_wait_time_lbl(cur, &WaitTime {
            time: self.step_wait_time_tb.pos(),
            unit: WaitTimeUnit::TensOfMicro,
        });
        self.wait_time_button.set_enabled(true);
    }

    fn send_wait_time(&self) {
        let new_wait_time = WaitTime {
            time: self.step_wait_time_tb.pos(),
            unit: self.get_next_wait_time_unit(),
        };

        self.update_step_wait_time_lbl(&new_wait_time, &new_wait_time);
        self.wait_time_button.set_enabled(false);

        self.send_gui_msg(ToSysMessage::ChangeWaitTime(new_wait_time.to_micro()));

        self.data.borrow_mut().cur_wait_time = new_wait_time;
    }

    fn send_print_log(&self) {
        self.send_gui_msg(ToSysMessage::ShowLog(match self.print_log_cbox.check_state() {
            nwg::CheckBoxState::Checked => true,
            nwg::CheckBoxState::Unchecked => false,
            nwg::CheckBoxState::Indeterminate => panic!("CheckBox in indeterminate state"),
        }));
    }

    fn send_breakpoint(&self) {
        self.send_gui_msg(ToSysMessage::Breakpoint(match self.print_log_cbox.check_state() {
            nwg::CheckBoxState::Checked => true,
            nwg::CheckBoxState::Unchecked => false,
            nwg::CheckBoxState::Indeterminate => panic!("CheckBox in indeterminate state"),
        }));
    }

    fn send_gui_msg(&self, msg: ToSysMessage) {
        self.channels.tx.send(msg).expect("SYS thread has hung up");
    }
}

pub fn run(
    tx: Sender<ToSysMessage>, 
    rx: Receiver<ToGuiMessage>,
    bin_name: String
) {
    let channels = UIChannels { tx, rx };
    let data = RefCell::new(UIData { 
        bin_name, 
        cur_wait_time: Default::default(),
        sys_running: true,
    });

    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");
    let app: EmulatorGui = EmulatorGui { channels, data, ..Default::default() };
    let _ui = EmulatorGui::build_ui(app).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}
