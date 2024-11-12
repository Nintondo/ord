use super::*;

#[derive(PartialEq, Debug)]
pub struct DecimalSat {
  pub height: Height,
  pub offset: u64,
}

impl DecimalSat {
  pub fn from_sat(sat: Sat, network: Network) -> Self {
    Self {
      height: sat.height(),
      offset: sat.third(network),
    }
  }
}

impl Display for DecimalSat {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}.{}", self.height, self.offset)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn decimal() {
    assert_eq!(
      Sat(0).decimal(Network::Bellscoin),
      DecimalSat {
        height: Height(0),
        offset: 0
      }
    );
    assert_eq!(
      Sat(1).decimal(Network::Bellscoin),
      DecimalSat {
        height: Height(0),
        offset: 1
      }
    );
    assert_eq!(
      Sat(2099999997689999).decimal(Network::Bellscoin),
      DecimalSat {
        height: Height(6929999),
        offset: 0
      }
    );
  }
}
