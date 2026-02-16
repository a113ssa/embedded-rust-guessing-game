use embassy_stm32::{Peri, i2c::{Config, I2c, Master}, mode::Blocking, peripherals, time::Hertz};
use embassy_time::Delay;
use hd44780_driver::{HD44780, bus::I2CBus};
use heapless::String;

static I2C_FREEQ: u32 = 100_000;
static I2C_ADDRESS: u8 = 0x27;
static FIRST_LINE_POS: u8 = 0;
static SECOND_LINE_POS: u8 = 42;
static GUESS_WELCOME_TITLE: &str = "Guess the number";
static EMPTY_LINE: &str = "                ";
static EMPTY_CHAR: &str = " ";
pub static ANSWER_LENGTH: usize = 4;

// type aliases to simplify long type hint
type LcdI2c = I2c<'static, Blocking, Master>;
type LcdBus = I2CBus<LcdI2c>;
type LcdDriver<'a> = HD44780<LcdBus>;

pub struct LcdModule {
    driver: LcdDriver<'static> ,
}

impl LcdModule {
    pub fn new(
        i2c_pin: Peri<'static, peripherals::I2C1>,
        scl_pin: Peri<'static, peripherals::PB8>,
        sda_pin: Peri<'static, peripherals::PB9>,
    ) -> Self {
        Self {
            driver: Self::init_driver(i2c_pin, scl_pin, sda_pin)
        }
    }

    fn init_driver(
        i2c_pin: Peri<'static, peripherals::I2C1>,
        scl_pin: Peri<'static, peripherals::PB8>,
        sda_pin: Peri<'static, peripherals::PB9>,
    ) -> LcdDriver<'static> {
        let mut i2c_config: Config = Config::default();
        i2c_config.frequency = Hertz(I2C_FREEQ);

        // hd44780-driver crate only supports blocking I2C
        let i2c: I2c<'_, Blocking, Master> = I2c::new_blocking(
            i2c_pin,
            scl_pin,
            sda_pin,
            i2c_config,
        );

        let mut delay: Delay = Delay;

        return HD44780::new_i2c(
            i2c,
            I2C_ADDRESS,
            &mut delay,
        ).expect("Failed to init LCD");
    }

    pub fn erase(&mut self) {
        let mut delay = Delay;
        self.driver.set_cursor_pos(FIRST_LINE_POS, &mut delay).unwrap();
        self.driver.write_str(GUESS_WELCOME_TITLE, &mut delay).unwrap();
        self.driver.set_cursor_pos(SECOND_LINE_POS, &mut delay).unwrap();
        self.driver.write_str(EMPTY_LINE, &mut delay).unwrap();
    }

    pub fn write(&mut self, s: &String<ANSWER_LENGTH>) {
        let mut delay = Delay;
        self.driver.set_cursor_pos(SECOND_LINE_POS, &mut delay).unwrap();

        self.driver.write_str(s, &mut delay).unwrap();

        // remove ghosted chars
        let remaining = ANSWER_LENGTH - s.len();
        for _ in 0..remaining {
            self.driver.write_str(EMPTY_CHAR, &mut delay).unwrap();
        }
    }
}
