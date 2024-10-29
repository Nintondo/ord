use super::*;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Display, FromStr)]
pub struct Height(pub u32);

impl Height {
  pub fn n(self) -> u32 {
    self.0
  }

  pub fn subsidy(self) -> u64 {
    get_block_subsidy(self.0)
  }

  pub fn starting_sat(self) -> Sat {
    let epoch = Epoch::from(self);
    let epoch_starting_subsidy = SatsSubsidy::sat_from_height(epoch.starting_height());
    let current_subsidy = SatsSubsidy::sat_from_height(self);
    Sat(current_subsidy.0 - epoch_starting_subsidy.0)
  }
}

impl Add<u32> for Height {
  type Output = Self;

  fn add(self, other: u32) -> Height {
    Self(self.0 + other)
  }
}

impl Sub<u32> for Height {
  type Output = Self;

  fn sub(self, other: u32) -> Height {
    Self(self.0 - other)
  }
}

impl PartialEq<u32> for Height {
  fn eq(&self, other: &u32) -> bool {
    self.0 == *other
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn block_subsidy() {
    assert_eq!(get_block_subsidy(1241), 10000);
    assert_eq!(get_block_subsidy(101), 100);
    assert_eq!(get_block_subsidy(102), 100);
    assert_eq!(get_block_subsidy(213214), 5000);
    assert_eq!(get_block_subsidy(4314), 100);
    assert_eq!(get_block_subsidy(129600), 25);
    assert_eq!(get_block_subsidy(259200), 5);
  }

  #[test]
  fn n() {
    assert_eq!(Height(0).n(), 0);
    assert_eq!(Height(1).n(), 1);
  }

  #[test]
  fn add() {
    assert_eq!(Height(0) + 1, 1);
    assert_eq!(Height(1) + 100, 101);
  }

  #[test]
  fn sub() {
    assert_eq!(Height(1) - 1, 0);
    assert_eq!(Height(100) - 50, 50);
  }

  #[test]
  fn eq() {
    assert_eq!(Height(0), 0);
    assert_eq!(Height(100), 100);
  }

  #[test]
  fn from_str() {
    assert_eq!("0".parse::<Height>().unwrap(), 0);
    assert!("foo".parse::<Height>().is_err());
  }

  #[test]
  fn subsidy() {
    assert_eq!(Height(0).subsidy(), 5000000000);
    assert_eq!(Height(1).subsidy(), 5000000000);
  }

  #[test]
  fn starting_sat() {
    assert_eq!(Height(0).starting_sat(), 0);
    assert_eq!(Height(1).starting_sat(), 5000000000);
    assert_eq!(
      Height(u32::MAX).starting_sat(),
      *Epoch::STARTING_SATS.last().unwrap()
    );
  }
}
