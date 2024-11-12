use bellscoin::Network;

const AUXPOW_START_HEIGHT: u32 = 144_000;
const AUXPOW_THRESHOLD: u32 = 1000;

struct MT19937 {
  mt: [u32; 624],
  index: usize,
}

impl MT19937 {
  fn new(seed: u32) -> MT19937 {
    let mut mt = MT19937 {
      mt: [0u32; 624],
      index: 624,
    };
    mt.seed_boost(seed);
    mt
  }

  fn seed_boost(&mut self, value: u32) {
    const N: usize = 624;
    const W: u32 = 32;
    const F: u32 = 1812433253;
    let mask = u32::MAX;

    self.mt[0] = value & mask;
    for i in 1..N {
      self.mt[i] = F
        .wrapping_mul(self.mt[i - 1] ^ (self.mt[i - 1] >> (W - 2)))
        .wrapping_add(i as u32)
        & mask;
    }
    self.index = N;
  }

  fn twist(&mut self) {
    const N: usize = 624;
    const M: usize = 397;
    const A: u32 = 0x9908b0df;
    const UPPER_MASK: u32 = 0x80000000; // Most significant bit
    const LOWER_MASK: u32 = 0x7fffffff; // Least significant 31 bits

    for i in 0..N {
      let x = (self.mt[i] & UPPER_MASK) + (self.mt[(i + 1) % N] & LOWER_MASK);
      let mut x_a = x >> 1;
      if x % 2 != 0 {
        x_a ^= A;
      }
      self.mt[i] = self.mt[(i + M) % N] ^ x_a;
    }
    self.index = 0;
  }

  fn next_u32(&mut self) -> u32 {
    if self.index >= 624 {
      self.twist();
    }

    let mut y = self.mt[self.index];
    self.index += 1;

    // Tempering
    y ^= y >> 11;
    y ^= (y << 7) & 0x9d2c5680;
    y ^= (y << 15) & 0xefc60000;
    y ^= y >> 18;

    y
  }
}

fn generate_mt_random(seed: u32, range: u32) -> u32 {
  let mut mt = MT19937::new(seed);

  let range_min = 1u64;
  let range_max = range as u64;
  let range_size = range_max - range_min + 1;

  // Generator's range
  let gen_min = 0u64;
  let gen_max = u32::MAX as u64;
  let r = gen_max - gen_min + 1; // Now r is u64 and will not overflow

  // Calculate bucket size and limit
  let bucket_size = r / range_size;
  let limit = bucket_size * range_size;

  loop {
    let rnum = mt.next_u32() as u64 - gen_min;
    if rnum < limit {
      let result = range_min + (rnum / bucket_size);
      return result as u32;
    }
    // Discard and generate a new number
  }
}

pub fn get_block_subsidy(n_height: u32, network: Network) -> u64 {
  let mut n_subsidy = 2;

  if n_height == 0 {
    return 88;
  }

  if n_height < 101 {
    return n_subsidy; // First 100 blocks have minimal rewards.
  }

  if network == Network::Bellscoin {
    if n_height >= AUXPOW_START_HEIGHT && n_height < AUXPOW_START_HEIGHT + AUXPOW_THRESHOLD {
      return n_subsidy;
    }
  }

  let rand = generate_mt_random(n_height, 1000);
  if n_height < 129600 {
    n_subsidy = match rand {
      990..=1000 => 10000,
      940..=989 => 1000,
      840..=939 => 500,
      700..=839 => 250,
      500..=699 => 100,
      _ => 50,
    };
  } else if n_height < 259200 {
    n_subsidy = match rand {
      990..=1000 => 5000,
      940..=989 => 500,
      840..=939 => 250,
      700..=839 => 125,
      500..=699 => 50,
      _ => 25,
    };
  } else if n_height < 518400 {
    n_subsidy = match rand {
      990..=1000 => 500,
      940..=989 => 50,
      840..=939 => 25,
      500..=839 => 10,
      _ => 5,
    };
  }

  n_subsidy
}

#[cfg(test)]
mod tests {
  use super::*;

  fn subsidy_mainnet(n_height: u32) -> u64 {
    get_block_subsidy(n_height, Network::Bellscoin)
  }

  #[test]
  fn block_subsidy() {
    assert_eq!(subsidy_mainnet(101), 100);
    assert_eq!(subsidy_mainnet(102), 100);
    assert_eq!(subsidy_mainnet(113), 500);
    assert_eq!(subsidy_mainnet(119), 500);
    assert_eq!(subsidy_mainnet(318), 100);
    assert_eq!(subsidy_mainnet(319), 10_000);
    assert_eq!(subsidy_mainnet(444), 500);
    assert_eq!(subsidy_mainnet(1241), 10000);
    assert_eq!(subsidy_mainnet(4314), 100);
    assert_eq!(subsidy_mainnet(100_000), 50);
    assert_eq!(subsidy_mainnet(129600), 25);
    assert_eq!(subsidy_mainnet(213214), 5000);
    assert_eq!(subsidy_mainnet(259200), 5);
    assert_eq!(subsidy_mainnet(259211), 10);
    assert_eq!(subsidy_mainnet(143877), 25)
  }
}
