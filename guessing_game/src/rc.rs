use embassy_stm32::{
    Peri, bind_interrupts,
    gpio::Pull,
    peripherals,
    time::Hertz,
    timer::{
        CaptureCompareInterruptHandler, Channel,
        input_capture::{CapturePin, InputCapture},
        low_level::CountingMode,
    },
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Sender};
use embassy_time::{Duration, Instant};
use infrared::{
    Receiver,
    protocol::{Nec16, nec::Nec16Command},
    receiver::NoPin,
};

static RECEIVER_FREQ: u32 = 1_000_000;
static DEBOUNCE_THRESHHOLD: u64 = 300;
static DEBOUNCE_DURATION: Duration = Duration::from_millis(DEBOUNCE_THRESHHOLD);

bind_interrupts!(struct Tim2Interrupt {
    TIM2 => CaptureCompareInterruptHandler<embassy_stm32::peripherals::TIM2>;
});

pub struct RcModule {
    receiver: Receiver<Nec16, NoPin, u32, Nec16Command>,
    input_capture: InputCapture<'static, peripherals::TIM2>,
}

impl RcModule {
    pub fn new(
        input_capture_pin: Peri<'static, peripherals::PA0>,
        tim2: Peri<'static, peripherals::TIM2>,
    ) -> Self {
        Self {
            receiver: Receiver::<Nec16, NoPin, u32, Nec16Command>::new(RECEIVER_FREQ),
            input_capture: InputCapture::new(
                tim2,
                Some(CapturePin::new(input_capture_pin, Pull::Up)),
                None,
                None,
                None,
                Tim2Interrupt,
                Hertz(RECEIVER_FREQ),
                CountingMode::EdgeAlignedUp,
            ),
        }
    }
}

#[embassy_executor::task]
pub async fn ir_decoder_task(
    mut rc_module: RcModule,
    sender: Sender<'static, CriticalSectionRawMutex, char, 8>,
) -> ! {
    let mut last_capture: u32 = 0;
    let mut last_edge: bool = true;
    let mut last_processed_time: Instant = Instant::now();

    loop {
        let current_capture: u32 = rc_module
            .input_capture
            .wait_for_any_edge(Channel::Ch1)
            .await;
        let delta_time: u32 = current_capture.wrapping_sub(last_capture);
        last_capture = current_capture;

        match &mut rc_module.receiver.event(delta_time, last_edge) {
            Ok(Some(cmd)) => {
                let current_processed_time = Instant::now();

                if current_processed_time.duration_since(last_processed_time) >= DEBOUNCE_DURATION {
                    last_processed_time = Instant::now();

                    let command: Option<char> = map_command(cmd.cmd);

                    match command {
                        Some(c) => sender.send(c).await,
                        _ => {}
                    }
                }
            }
            Ok(None) => {}
            Err(_e) => {}
        }

        last_edge = !last_edge;
    }
}

fn map_command(command: u8) -> Option<char> {
    return match command {
        22 => Some('0'),
        12 => Some('1'),
        24 => Some('2'),
        94 => Some('3'),
        8 => Some('4'),
        28 => Some('5'),
        90 => Some('6'),
        66 => Some('7'),
        82 => Some('8'),
        74 => Some('9'),
        68 => Some('d'),
        64 => Some('e'),
        _ => None,
    };
}
