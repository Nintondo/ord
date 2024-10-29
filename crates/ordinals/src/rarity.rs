use super::*;

#[derive(Debug, PartialEq, PartialOrd, Copy, Clone, DeserializeFromStr, SerializeDisplay)]
pub enum Rarity {
  Common,
  Uncommon,
  Epic,
  Mythic,
}

impl From<Rarity> for u8 {
  fn from(rarity: Rarity) -> Self {
    rarity as u8
  }
}

impl TryFrom<u8> for Rarity {
  type Error = u8;

  fn try_from(rarity: u8) -> Result<Self, u8> {
    match rarity {
      0 => Ok(Self::Common),
      1 => Ok(Self::Uncommon),
      3 => Ok(Self::Epic),
      5 => Ok(Self::Mythic),
      n => Err(n),
    }
  }
}

impl Display for Rarity {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(
      f,
      "{}",
      match self {
        Self::Common => "common",
        Self::Uncommon => "uncommon",
        Self::Epic => "epic",
        Self::Mythic => "mythic",
      }
    )
  }
}

impl From<Sat> for Rarity {
  fn from(sat: Sat) -> Self {
    let epoch = Epoch::from(sat);
    let block_first_sat = SatsSubsidy::height_from_sat(sat).starting_sat();

    if sat.0 == 0 {
      Self::Mythic
    } else if sat.0 == epoch.starting_sat().0 {
      Self::Epic
    } else if block_first_sat.0 == sat.0 {
      Self::Uncommon
    } else {
      Self::Common
    }
  }
}

impl FromStr for Rarity {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "common" => Ok(Self::Common),
      "uncommon" => Ok(Self::Uncommon),
      "epic" => Ok(Self::Epic),
      "mythic" => Ok(Self::Mythic),
      _ => Err(format!("invalid rarity `{s}`")),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rarity() {
    assert_eq!(Sat(0).rarity(), Rarity::Mythic);
    assert_eq!(Sat(1).rarity(), Rarity::Common);

    assert_eq!(Sat(50 * COIN_VALUE - 1).rarity(), Rarity::Common);
    assert_eq!(Sat(50 * COIN_VALUE).rarity(), Rarity::Uncommon);
    assert_eq!(Sat(50 * COIN_VALUE + 1).rarity(), Rarity::Common);

    assert_eq!(Sat(2067187500000000 - 1).rarity(), Rarity::Common);
    assert_eq!(Sat(2067187500000000 + 1).rarity(), Rarity::Common);
  }

  #[test]
  fn from_str_and_deserialize_ok() {
    #[track_caller]
    fn case(s: &str, expected: Rarity) {
      let actual = s.parse::<Rarity>().unwrap();
      assert_eq!(actual, expected);
      let round_trip = actual.to_string().parse::<Rarity>().unwrap();
      assert_eq!(round_trip, expected);
      let serialized = serde_json::to_string(&expected).unwrap();
      assert!(serde_json::from_str::<Rarity>(&serialized).is_ok());
    }

    case("common", Rarity::Common);
    case("uncommon", Rarity::Uncommon);
    case("epic", Rarity::Epic);
    case("mythic", Rarity::Mythic);
  }

  #[test]
  fn conversions_with_u8() {
    for &expected in &[
      Rarity::Common,
      Rarity::Uncommon,
      Rarity::Epic,
      Rarity::Mythic,
    ] {
      let n: u8 = expected.into();
      let actual = Rarity::try_from(n).unwrap();
      assert_eq!(actual, expected);
    }

    assert_eq!(Rarity::try_from(6), Err(6));
  }

  #[test]
  fn error() {
    assert_eq!("foo".parse::<Rarity>().unwrap_err(), "invalid rarity `foo`");
  }
}
