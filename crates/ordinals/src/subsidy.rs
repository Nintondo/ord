use super::*;

const AUXPOW_START_HEIGHT: u32 = 100_000;
const AUXPOW_THRESHOLD: u32 = 1000;

fn generate_mt_random(s: u32) -> u32 {
  Mt19937GenRand32::new(s).gen_range(1..1000)
}

#[repr(transparent)]
pub struct SatSubsidy(pub u64);

impl From<Sat> for SatSubsidy {
  fn from(value: Sat) -> Self {
    match value.0 {
      0..=100 => Self(100),
      101..=129600 => Self(50),
      129601..=259200 => Self(25),
      259201..=518400 => Self(5),
      _ => Self(1),
    }
  }
}

pub fn get_block_subsidy(n_height: u32) -> u64 {
  let mut n_subsidy = 2;

  if n_height < 101 {
    return n_subsidy; // First 100 blocks have minimal rewards.
  } else if n_height >= AUXPOW_START_HEIGHT && n_height < AUXPOW_START_HEIGHT + AUXPOW_THRESHOLD {
    return n_subsidy;
  } else {
    let rand = generate_mt_random(n_height);
    if n_height < 129600 {
      n_subsidy = match rand {
        990..=999 => 10000,
        940..=989 => 1000,
        840..=939 => 500,
        700..=839 => 250,
        500..=699 => 100,
        _ => 50,
      };
    } else if n_height < 259200 {
      n_subsidy = match rand {
        990..=999 => 5000,
        940..=989 => 500,
        840..=939 => 250,
        700..=839 => 125,
        500..=699 => 50,
        _ => 25,
      };
    } else if n_height < 518400 {
      n_subsidy = match rand {
        990..=999 => 500,
        940..=989 => 50,
        840..=939 => 25,
        500..=699 => 10,
        _ => 5,
      };
    }
  }

  n_subsidy
}
