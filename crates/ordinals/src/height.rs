use super::*;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Display, FromStr)]
pub struct Height(pub u32);

impl Height {
  pub fn n(self) -> u32 {
    self.0
  }

  pub fn subsidy(self) -> u64 {
    get_block_subsidy(self.0) * COIN_VALUE
  }

  pub fn starting_sat(self) -> Sat {
    SatsSubsidy::sat_from_height(self)
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
