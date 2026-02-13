#![no_std]
#![no_main]

use embassy_executor::{Spawner};
use embassy_stm32::mode::Blocking;
use embassy_stm32::rcc::{Pll, PllMul, PllPreDiv, PllRDiv};
use embassy_stm32::{Config, Peripherals, bind_interrupts, peripherals, Peri};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Pull};
use embassy_stm32::exti::InterruptHandler;
use embassy_stm32::i2c::{Config as I2cConfig, I2c, Master};
use embassy_stm32::rcc::PllSource;
use embassy_stm32::rcc::Sysclk::{PLL1_R};
use embassy_stm32::rcc::MSIRange::{RANGE4M};
use embassy_time::{Delay, Instant};
use embassy_stm32::time::{Hertz};
use embassy_sync::channel::{Channel, Sender};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use hd44780_driver::bus::I2CBus;
use infrared::Receiver;
use infrared::protocol::nec::{Nec16Command,};
use infrared::receiver::{NoPin};
use infrared::protocol::{Nec16};
use hd44780_driver::{ HD44780 };

use defmt_rtt as _;
use panic_probe as _;

use defmt::info;

static CHANNEL: Channel<CriticalSectionRawMutex, char, 8> = Channel::new();

bind_interrupts!(struct Interrupts {
    EXTI0 => InterruptHandler<embassy_stm32::interrupt::typelevel::EXTI0>;
});

// type aliases to simplify long type hint
type LcdI2c = I2c<'static, Blocking, Master>;
type LcdBus = I2CBus<LcdI2c>;
type LcdDriver<'a> = HD44780<LcdBus>;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p: Peripherals = init_peripherals();

    let mut lcd: LcdDriver = init_lcd(p.I2C1, p.PB8, p.PB9);

    let ir_pin: ExtiInput<'_> = ExtiInput::new(p.PA0, p.EXTI0, Pull::Up, Interrupts);

    let rc =  Receiver::<Nec16, NoPin, u32, Nec16Command>::new(1_000_000);

    spawner.spawn(ir_decoder_task(rc, ir_pin, CHANNEL.sender())).unwrap();

    let mut number_buffer: [u8; 4] = [0u8; 4];
    let mut index = 0;
    let mut delay: Delay = Delay;

    lcd.clear(&mut delay).unwrap();
    lcd.write_str("Waiting...", &mut delay).unwrap();

    loop {
        let digit = CHANNEL.receive().await;
        info!("Digit: {}", digit);

        if number_buffer.len() >= 4 {
            number_buffer = [0u8; 4];
            index = 0;
            lcd.clear(&mut delay).unwrap();
        };

        number_buffer[index] = digit as u8;

        lcd.set_cursor_pos(0, &mut delay).unwrap();
        lcd.write_bytes(&number_buffer, &mut delay).unwrap();
    }
}

#[embassy_executor::task]
async fn ir_decoder_task(
    mut rc: Receiver<Nec16, NoPin, u32, Nec16Command>,
    mut ir_pin: ExtiInput<'static>,
    sender: Sender<'static, CriticalSectionRawMutex, char, 8>,
) -> ! {
    let mut last_tick = Instant::now();

    loop {
        ir_pin.wait_for_any_edge().await;

        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_micros() as u32;
        last_tick = now;

        let edge = ir_pin.is_low();

        match rc.event(dt, edge) {
            Ok(Some(cmd)) => {
                info!("Commad {}", cmd.cmd);
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
                    _ => None,
                };
                if let Some(d) = digit {
                    sender.send(d).await;
                }
            }
            Ok(None) => {}
            Err(_e) => {}
            };
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

fn init_lcd (
    i2c_p: impl Into<Peri<'static, peripherals::I2C1>>,
    scl: impl Into<Peri<'static, peripherals::PB8>>,
    sda: impl Into<Peri<'static, peripherals::PB9>>,
) -> LcdDriver<'static> {
    let mut i2c_config = I2cConfig::default();
    i2c_config.frequency = Hertz(100_000); // 100 kHz

    // hd44780-driver crate only supports blocking I2C
    let i2c = I2c::new_blocking(
        i2c_p.into(),
        scl.into(),
        sda.into(),
        i2c_config,
    );

    let mut delay: Delay = Delay;

    let i2c_address = 0x27;
    return HD44780::new_i2c(
        i2c,
        i2c_address,
        &mut delay
    ).expect("Failed to ini LCD");
}
