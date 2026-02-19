#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::rcc::MSIRange::RANGE4M;
use embassy_stm32::rcc::PllSource;
use embassy_stm32::rcc::Sysclk::PLL1_R;
use embassy_stm32::rcc::{Pll, PllMul, PllPreDiv, PllRDiv};
use embassy_stm32::{Config, Peripherals};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use heapless::String;

use game::Game;
use helper::convert_to_number;
use lcd::{ANSWER_LENGTH, LcdModule};
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

    let rc_module: RcModule = RcModule::new(p.PA0, p.TIM2);
    spawner
        .spawn(ir_decoder_task(rc_module, CHANNEL.sender()))
        .unwrap();

    let game: Game = Game::new();

    let mut answer: String<ANSWER_LENGTH> = String::new();

    loop {
        let command: char = CHANNEL.receive().await;

        process_command(command, &mut answer, &game, &mut lcd_module);
    }
}

fn process_command(
    command: char,
    answer: &mut String<ANSWER_LENGTH>,
    game: &Game,
    lcd_module: &mut LcdModule,
) {
    if answer.len() > ANSWER_LENGTH - 1 {
        lcd_module.erase();
        answer.clear();
        return;
    }

    match command {
        'd' => {
            answer.pop();
            lcd_module.write(&answer);
        }
        'e' => {
            if !answer.is_empty() {
                let answer_number: u8 = convert_to_number(answer);
                let answer_title: &str = &game.check(answer_number);
                lcd_module.write_title(&answer_title);
                lcd_module.erase_second_line();
                answer.clear();
            }
        }
        _ => {
            if answer.push(command).is_ok() {
                lcd_module.write(&answer);
            }
        }
    }
}

fn init_peripherals() -> Peripherals {
    let mut config: Config = Config::default();

    config.rcc.msi = Some(RANGE4M);
    config.rcc.sys = PLL1_R;
    config.rcc.pll = Some(Pll {
        source: PllSource::MSI,
        prediv: PllPreDiv::DIV1,
        mul: PllMul::MUL40,
        divp: None,
        divq: None,
        divr: Some(PllRDiv::DIV2),
    });

    return embassy_stm32::init(config);
}

mod game;
mod helper;
mod lcd;
mod rc;
