#![no_std]
#![no_main]

use embassy_executor::{Spawner};
use embassy_stm32::{Config, Peripherals};
use embassy_stm32::rcc::{Pll, PllMul, PllPreDiv, PllRDiv};
use embassy_stm32::rcc::PllSource;
use embassy_stm32::rcc::Sysclk::{PLL1_R};
use embassy_stm32::rcc::MSIRange::{RANGE4M};
use embassy_sync::channel::{Channel};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use heapless::String;

use lcd::{LcdModule, ANSWER_LENGTH};
use rc::{RcModule, ir_decoder_task};

use defmt_rtt as _;
use panic_probe as _;

use defmt::info;

static CHANNEL: Channel<CriticalSectionRawMutex, char, 8> = Channel::new();


#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p: Peripherals = init_peripherals();

    let mut lcd_module: LcdModule = LcdModule::new(p.I2C1, p.PB8, p.PB9);
    lcd_module.erase();

    let rc_module: RcModule =  RcModule::new(p.PA0, p.TIM2);
    spawner.spawn(ir_decoder_task(rc_module, CHANNEL.sender())).unwrap();

    let mut answer: String<ANSWER_LENGTH> = String::new();

    loop {
        let command: char = CHANNEL.receive().await;

        if command == 'd' {
            answer.pop();
        } else {
            answer.push(command).unwrap();
        }

        if answer.len() > ANSWER_LENGTH - 1 {
            lcd_module.erase();
            answer.clear();
        }

        lcd_module.write(&answer);
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
mod rc;
