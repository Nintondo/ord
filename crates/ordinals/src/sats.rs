use io::Read;

use super::*;

lazy_static! {
  static ref FILE_CONTENT: HashMap<u32, u64> = {
    String::from_utf8(decompress_brotli().unwrap())
      .unwrap()
      .lines()
      .enumerate()
      .filter_map(|(idx, line)| line.parse::<u64>().ok().map(|x| (idx as u32, x)))
      .collect()
  };
}

fn decompress_brotli() -> io::Result<Vec<u8>> {
  let compressed_file = include_bytes!("./sats.txt.br");
  let mut compressed_file_cursor = std::io::Cursor::new(compressed_file);
  let mut decompressed_data = Vec::new();
  let mut decompressor = brotli::Decompressor::new(&mut compressed_file_cursor, 4096);
  decompressor.read_to_end(&mut decompressed_data)?;

  Ok(decompressed_data)
}

#[repr(transparent)]
pub struct SatsSubsidy;

impl SatsSubsidy {
  pub fn height_from_sat(value: Sat) -> Height {
    let mut low = 0;
    let mut high = 800_000;

    while low <= high {
      let mid = (low + high) / 2;
      let mid_height = Height(mid as u32);
      let mid_value = Self::sat_from_height(mid_height).0;

      if mid_value == value.0 {
        return mid_height;
      } else if mid_value < value.0 {
        low = mid + 1;
      } else {
        high = mid - 1;
      }
    }

    // Return the highest height that does not exceed the value
    if high < 0 {
      Height(0)
    } else {
      Height(high as u32)
    }
  }

  pub fn sat_from_height(value: Height) -> Sat {
    Sat(*FILE_CONTENT.get(&value.0).unwrap() * COIN_VALUE)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sat_from_height() {
    assert_eq!(SatsSubsidy::sat_from_height(Height(0)), 0);
    assert_eq!(SatsSubsidy::sat_from_height(Height(1)), 88 * COIN_VALUE);
    assert_eq!(SatsSubsidy::sat_from_height(Height(124)), 3588 * COIN_VALUE);
    assert_eq!(SatsSubsidy::sat_from_height(Height(125)), 3638 * COIN_VALUE);
  }

  #[test]
  fn test_height_from_sat() {
    assert_eq!(SatsSubsidy::height_from_sat(Sat(363799999999)).0, 124);
    assert_eq!(SatsSubsidy::height_from_sat(Sat(363800000000)).0, 125);
    assert_eq!(SatsSubsidy::height_from_sat(Sat(363800000001)).0, 125);
    assert_eq!(SatsSubsidy::height_from_sat(Sat(0)).0, 0);
    assert_eq!(SatsSubsidy::height_from_sat(Sat(8800000000)).0, 1);
    assert_eq!(SatsSubsidy::height_from_sat(Sat(8799999999)).0, 0);
    assert_eq!(SatsSubsidy::height_from_sat(Sat(8799999998)).0, 0);
  }
}
