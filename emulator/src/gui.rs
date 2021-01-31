use nwg::NativeUi;
use nwd::NwgUi;
use std::sync::mpsc::{self, Sender, Receiver, TryRecvError};
use std::cell::RefCell;
use crate::{GuiMessage, CpuMessage, DEFAULT_STEP_WAIT};

struct UIChannels {
    tx: Sender<GuiMessage>,
    rx: Receiver<CpuMessage>,
}

impl Default for UIChannels {
    fn default() -> Self {
        // This default code shouldn't be used
        let (tx, _) = mpsc::channel();
        let (_, rx) = mpsc::channel();
        UIChannels { tx, rx }
    }
}

#[derive(Default)]
struct UIData {
    bin_name: String,
    cur_wait_time: usize,
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
    
    // Refresh CPU data (~30 FPS)
    #[nwg_control(parent: window, interval: 33, stopped: false)]
    #[nwg_events(OnTimerTick: [EmulatorGui::update_cpu_data])]
    refresh_timer: nwg::Timer,

    #[nwg_layout(parent: window, spacing: 1)]
    grid: nwg::GridLayout,

    #[nwg_control(text: "")]
    #[nwg_layout_item(layout: grid, row: 0, col: 0, col_span: 6)]
    bin_name_lbl: nwg::Label,

    #[nwg_control(text: "Run", check_state: RadioButtonState::Unchecked)]
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
    #[nwg_layout_item(layout: grid, row: 2, col: 0, row_span: 2, col_span: 5)]
    step_wait_time_lbl: nwg::Label,

    #[nwg_control(text: "✓")]
    #[nwg_layout_item(layout: grid, row: 3, col: 5)]
    #[nwg_events( OnButtonClick: [EmulatorGui::send_wait_time] )]
    wait_time_button: nwg::Button,

    #[nwg_control(flags: "TICK_TOP|VISIBLE")]
    #[nwg_layout_item(layout: grid, row: 4, col: 0, col_span: 6)]
    #[nwg_events( OnHorizontalScroll: [EmulatorGui::step_wait_time_tb_change] )]
    step_wait_time_tb: nwg::TrackBar,

    #[nwg_control(text: "Show log in console", 
        check_state: nwg::CheckBoxState::Unchecked)]
    #[nwg_layout_item(layout: grid, row: 5, col: 0, col_span: 4)]
    #[nwg_events( OnButtonClick: [EmulatorGui::send_show_log] )]
    show_log_cbox: nwg::CheckBox,

    #[nwg_control()]
    #[nwg_layout_item(layout: grid, row: 0, col: 6, row_span: 6, col_span: 6)]
    tabs_container: nwg::TabsContainer,

    #[nwg_control(parent: tabs_container, text: "Leds")]
    tab_leds: nwg::Tab,

    #[nwg_control(parent: tabs_container, text: "LCD")]
    tab_lcd: nwg::Tab,
    
    #[nwg_resource(source_file: Some("./imgs/led_on.bmp"))]
    led_on_bmp: nwg::Bitmap,

    #[nwg_resource(source_file: Some("./imgs/led_off.bmp"))]
    led_off_bmp: nwg::Bitmap,

    #[nwg_layout(parent: tab_leds, spacing: 1)]
    led_grid: nwg::GridLayout,

    #[nwg_control(parent: tab_leds, text: "Port B: 0x00 (0)")]
    #[nwg_layout_item(layout: led_grid, row: 0, col: 0)]
    port_b_lbl: nwg::Label,

    #[nwg_control(parent: tab_leds)]
    #[nwg_layout_item(layout: led_grid, row: 1, col: 0, row_span: 2)]
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

    #[nwg_control(parent: tab_leds)]
    #[nwg_layout_item(layout: led_grid, row: 4, col: 0, row_span: 2)]
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
}

impl EmulatorGui {
    fn init(&self) {
        self.bin_name_lbl.set_text(&format!("Executing file: {}.bin", self.data.borrow().bin_name));
        
        // The trackbar values cannot be set when it is created, according to the winapi docs,
        // so they are set in this init function
        self.step_wait_time_tb.set_range_min(0);
        self.step_wait_time_tb.set_range_max(1000);
        self.step_wait_time_tb.set_pos(DEFAULT_STEP_WAIT);
        
        // The above triggers a step_wait_time_tb_change, so we need to disable the button afterwards
        self.wait_time_button.set_enabled(false);
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

    fn update_cpu_data(&self) {
        loop { match self.channels.rx.try_recv() {
            Err(TryRecvError::Disconnected) => panic!("Processor thread has hung up"),
            Err(TryRecvError::Empty) => break,
            Ok(msg) => match msg {
                CpuMessage::PortB(mut port_b_data) => {
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
                CpuMessage::PortA(mut port_a_data) => {
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
            },
        }};
    }

    fn send_run(&self) {
        self.step_button.set_enabled(false);
        self.channels.tx.send(GuiMessage::Run).expect("Processor thread has hung up");
    }

    fn send_stop(&self) {
        self.step_button.set_enabled(true);
        self.channels.tx.send(GuiMessage::Stop).expect("Processor thread has hung up");
    }

    fn send_step(&self) {
        self.channels.tx.send(GuiMessage::Step).expect("Processor thread has hung up");
    }

    fn update_step_wait_time_lbl(&self, cur: usize, sel: usize) {
        self.step_wait_time_lbl.set_text(&format!(
            "Wait time between steps:\nCurrent:  {}ms\nSelected: {}ms",
            cur,
            sel
        ));
    } 

    fn step_wait_time_tb_change(&self) {
        let data = self.data.borrow_mut();

        self.update_step_wait_time_lbl(data.cur_wait_time, self.step_wait_time_tb.pos());
        self.wait_time_button.set_enabled(true);
    }

    fn send_wait_time(&self) {
        let new_wait_time = self.step_wait_time_tb.pos();

        self.data.borrow_mut().cur_wait_time = new_wait_time;
        self.update_step_wait_time_lbl(new_wait_time, new_wait_time);
        self.wait_time_button.set_enabled(false);

        self.channels.tx.send(GuiMessage::ChangeWaitTime(new_wait_time))
            .expect("Processor thread has hung up");
    }

    fn send_show_log(&self) {
        self.channels.tx.send(GuiMessage::ShowLog(match self.show_log_cbox.check_state() {
            nwg::CheckBoxState::Checked => true,
            nwg::CheckBoxState::Unchecked => false,
            nwg::CheckBoxState::Indeterminate => panic!("CheckBox in indeterminate state"),
        })).expect("Processor thread has hung up");
    }
}

pub fn run(
    tx: Sender<GuiMessage>, 
    rx: Receiver<CpuMessage>,
    bin_name: String
) {
    let channels = UIChannels { tx, rx };
    let data = RefCell::new(UIData { 
        bin_name, 
        cur_wait_time: DEFAULT_STEP_WAIT,
    });

    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");
    let app: EmulatorGui = EmulatorGui { channels, data, ..Default::default() };
    let _ui = EmulatorGui::build_ui(app).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}