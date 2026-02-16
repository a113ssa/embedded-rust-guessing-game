#![no_std]
#![no_main]

use embassy_executor::{Spawner};
use embassy_stm32::rcc::{Pll, PllMul, PllPreDiv, PllRDiv};
use embassy_stm32::timer::{CaptureCompareInterruptHandler, Channel};
use embassy_stm32::timer::input_capture::{CapturePin, InputCapture};
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::{Config, Peripherals, bind_interrupts, peripherals, Peri};
use embassy_stm32::gpio::{Pull};
use embassy_stm32::rcc::PllSource;
use embassy_stm32::rcc::Sysclk::{PLL1_R};
use embassy_stm32::rcc::MSIRange::{RANGE4M};
use embassy_time::{Delay, Duration, Instant};
use embassy_stm32::time::{Hertz};
use embassy_sync::channel::{Channel as SyncChannel, Sender};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use heapless::String;
use infrared::Receiver;
use infrared::protocol::nec::{Nec16Command,};
use infrared::receiver::{NoPin};
use infrared::protocol::{Nec16};

use lcd::{LcdModule, ANSWER_LENGTH};

use defmt_rtt as _;
use panic_probe as _;

use defmt::info;

static CHANNEL: SyncChannel<CriticalSectionRawMutex, char, 8> = SyncChannel::new();

static RECEIVER_FREQ: u32 = 1_000_000;

static DEBOUNCE_THRESHHOLD: u64 = 300;


bind_interrupts!(struct Tim2Interrupt {
    TIM2 => CaptureCompareInterruptHandler<embassy_stm32::peripherals::TIM2>;
});


#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p: Peripherals = init_peripherals();

    let mut lcd_module: LcdModule = LcdModule::new(p.I2C1, p.PB8, p.PB9);

    let rc =  Receiver::<Nec16, NoPin, u32, Nec16Command>::new(RECEIVER_FREQ);

    spawner.spawn(ir_decoder_task(rc, p.TIM2, p.PA0, CHANNEL.sender())).unwrap();

    let mut digits: String<ANSWER_LENGTH> = String::new();

    lcd_module.erase();

    loop {
        let digit = CHANNEL.receive().await;

        if digit == 'd' {
            digits.pop();
        } else {
            digits.push(digit).unwrap();
        }

        if digits.len() > ANSWER_LENGTH - 1 {
            lcd_module.erase();
            digits.clear();
        }

        lcd_module.write(&digits);
    }
}

#[embassy_executor::task]
async fn ir_decoder_task(
    mut rc: Receiver<Nec16, NoPin, u32, Nec16Command>,
    tim2: Peri<'static, peripherals::TIM2>,
    pa0: Peri<'static, peripherals::PA0>,
    sender: Sender<'static, CriticalSectionRawMutex, char, 8>,
) -> ! {
    let mut ic = InputCapture::new(
        tim2,
        Some(CapturePin::new(pa0, Pull::Up)),
        None,
        None,
        None,
        Tim2Interrupt,
        Hertz(RECEIVER_FREQ),
        CountingMode::EdgeAlignedUp,
    );

    let mut last_capture: u32 = 0;
    let mut edge = true;

    let mut last_processed_time = Instant::now();
    let debounce_threshhold = Duration::from_millis(DEBOUNCE_THRESHHOLD);

    loop {
        let now = ic.wait_for_any_edge(Channel::Ch1).await;
        let dt = now.wrapping_sub(last_capture);
        last_capture = now;

        match rc.event(dt, edge) {
            Ok(Some(cmd)) => {
                let current_processed_time = Instant::now();
                if current_processed_time.duration_since(last_processed_time) >= debounce_threshhold {
                    last_processed_time = Instant::now();
                    let digit: Option<char> = match cmd.cmd {
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

                    if let Some(d) = digit {
                        sender.send(d).await;
                    }
                } else {}
            }

            Ok(None) => {}
            Err(_e) => {}
        }

        edge = !edge;
    }
}

fn init_peripherals() -> Peripherals {
    let mut config: Config = Config::default();

    config.rcc.msi = Some(RANGE4M);
    config.rcc.sys = PLL1_R;
    config.rcc.pll = Some(Pll{
        source: PllSource::MSI,
        prediv: PllPreDiv::DIV1,
        mul: PllMul::MUL40,
        divp: None,
        divq: None,
        divr: Some(PllRDiv::DIV2)
    });

    return embassy_stm32::init(config);
}

mod lcd;
