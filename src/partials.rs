use super::*;

#[derive(Serialize, Deserialize)]
pub struct Partial {
  pub vout: u32,
  pub inscription_idx: u32,
  pub outpoints: Vec<OutPoint>,
}

#[derive(Debug, PartialEq)]
pub enum ParsedInscription {
  None,
  Partial,
  Complete(Inscription),
}

pub struct InscriptionParser {}

impl InscriptionParser {
  pub fn parse(sig_scripts: Vec<&Script>) -> ParsedInscription {
    let sig_script = &sig_scripts[0];

    let mut push_datas_vec = match Self::decode_push_datas(sig_script) {
      Some(push_datas) => push_datas,
      None => return ParsedInscription::None,
    };

    let mut push_datas = push_datas_vec.as_slice();

    // read protocol

    if push_datas.len() < 3 {
      return ParsedInscription::None;
    }

    let protocol = &push_datas[0];

    if *protocol != PROTOCOL_ID {
      return ParsedInscription::None;
    }

    // read npieces

    let mut npieces = match Self::push_data_to_number(&push_datas[1]) {
      Some(n) => n,
      None => return ParsedInscription::None,
    };

    if npieces == 0 {
      return ParsedInscription::None;
    }

    // read content type

    let Result::Ok(content_type) = String::from_utf8(push_datas[2].clone()) else {
      return ParsedInscription::None;
    };

    push_datas = &push_datas[3..];

    // read body

    let mut body = vec![];

    let mut sig_scripts = sig_scripts.as_slice();

    // loop over transactions
    loop {
      // loop over chunks
      loop {
        if npieces == 0 {
          let inscription = Inscription {
            content_type: Some(content_type.as_bytes().to_vec()),
            body: Some(body),
            content_encoding: None,
            delegate: None,
            duplicate_field: false,
            incomplete_field: false,
            metadata: None,
            metaprotocol: None,
            parents: vec![],
            pointer: None,
            rune: None,
            unrecognized_even_field: false,
          };

          return ParsedInscription::Complete(inscription);
        }

        if push_datas.len() < 2 {
          break;
        }

        let next = match Self::push_data_to_number(&push_datas[0]) {
          Some(n) => n,
          None => break,
        };

        if next != npieces - 1 {
          break;
        }

        body.append(&mut push_datas[1].clone());

        push_datas = &push_datas[2..];
        npieces -= 1;
      }

      if sig_scripts.len() <= 1 {
        return ParsedInscription::Partial;
      }

      sig_scripts = &sig_scripts[1..];

      push_datas_vec = match Self::decode_push_datas(sig_scripts[0]) {
        Some(push_datas) => push_datas,
        None => return ParsedInscription::None,
      };

      if push_datas_vec.len() < 2 {
        return ParsedInscription::None;
      }

      let next = match Self::push_data_to_number(&push_datas_vec[0]) {
        Some(n) => n,
        None => return ParsedInscription::None,
      };

      if next != npieces - 1 {
        return ParsedInscription::None;
      }

      push_datas = push_datas_vec.as_slice();
    }
  }

  fn decode_push_datas(script: &Script) -> Option<Vec<Vec<u8>>> {
    let mut bytes = script.as_bytes();
    let mut push_datas = vec![];

    while !bytes.is_empty() {
      // op_0
      if bytes[0] == 0 {
        push_datas.push(vec![]);
        bytes = &bytes[1..];
        continue;
      }

      // op_1 - op_16
      if bytes[0] >= 81 && bytes[0] <= 96 {
        push_datas.push(vec![bytes[0] - 80]);
        bytes = &bytes[1..];
        continue;
      }

      // op_push 1-75
      if bytes[0] >= 1 && bytes[0] <= 75 {
        let len = bytes[0] as usize;
        if bytes.len() < 1 + len {
          return None;
        }
        push_datas.push(bytes[1..1 + len].to_vec());
        bytes = &bytes[1 + len..];
        continue;
      }

      // op_pushdata1
      if bytes[0] == 76 {
        if bytes.len() < 2 {
          return None;
        }
        let len = bytes[1] as usize;
        if bytes.len() < 2 + len {
          return None;
        }
        push_datas.push(bytes[2..2 + len].to_vec());
        bytes = &bytes[2 + len..];
        continue;
      }

      // op_pushdata2
      if bytes[0] == 77 {
        if bytes.len() < 3 {
          return None;
        }
        let len = ((bytes[1] as usize) << 8) + (bytes[0] as usize);
        if bytes.len() < 3 + len {
          return None;
        }
        push_datas.push(bytes[3..3 + len].to_vec());
        bytes = &bytes[3 + len..];
        continue;
      }

      // op_pushdata4
      if bytes[0] == 78 {
        if bytes.len() < 5 {
          return None;
        }
        let len = ((bytes[3] as usize) << 24)
          + ((bytes[2] as usize) << 16)
          + ((bytes[1] as usize) << 8)
          + (bytes[0] as usize);
        if bytes.len() < 5 + len {
          return None;
        }
        push_datas.push(bytes[5..5 + len].to_vec());
        bytes = &bytes[5 + len..];
        continue;
      }

      return None;
    }

    Some(push_datas)
  }

  fn push_data_to_number(data: &[u8]) -> Option<u64> {
    if data.is_empty() {
      return Some(0);
    }

    if data.len() > 8 {
      return None;
    }

    let mut n: u64 = 0;
    let mut m: u64 = 0;

    for i in data {
      n += u64::from(*i) << m;
      m += 8;
    }

    Some(n)
  }
}
