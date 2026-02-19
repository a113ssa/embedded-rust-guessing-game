use rand::{RngExt, SeedableRng, rngs::SmallRng};

static GREATER_TITLE: &str = "Number is greater";
static LOWER_TITLE: &str = "Number is lower";
static RIGHT_TITILE: &str = "Right! Congratz!";

static RANGE: u8 = 100;

pub struct Game {
    random_number: u8,
}

impl Game {
    pub fn new() -> Self {
        Self {
            random_number: Self::get_random_number(RANGE),
        }
    }

    pub fn check(&self, number: u8) -> &'static str {
        if number > self.random_number {
            return LOWER_TITLE;
        } else if number < self.random_number {
            return GREATER_TITLE;
        } else {
            return RIGHT_TITILE;
        }
    }

    fn get_random_number(range: u8) -> u8 {
        let mut rng = SmallRng::seed_from_u64(42);
        return rng.random_range(..=range);
    }
}
