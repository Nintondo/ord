//! Types for interoperating with ordinals, inscriptions, and runes.
#![allow(clippy::large_enum_variant)]
#[macro_use]
extern crate lazy_static;

use {
  bellscoin::{
    consensus::{Decodable, Encodable},
    constants::{COIN_VALUE, MAX_SCRIPT_ELEMENT_SIZE, SUBSIDY_HALVING_INTERVAL},
    opcodes,
    script::{self, Instruction},
    Network, OutPoint, ScriptBuf, Transaction,
  },
  derive_more::{Display, FromStr},
  sats::SatsSubsidy,
  serde::{Deserialize, Serialize},
  serde_with::{DeserializeFromStr, SerializeDisplay},
  std::{
    cmp,
    collections::{HashMap, VecDeque},
    fmt::{self, Formatter},
    io,
    num::ParseIntError,
    ops::{Add, AddAssign, Sub},
  },
  subsidy::get_block_subsidy,
  thiserror::Error,
};

pub use {
  artifact::Artifact, cenotaph::Cenotaph, charm::Charm, decimal_sat::DecimalSat, edict::Edict,
  epoch::Epoch, etching::Etching, flaw::Flaw, height::Height, pile::Pile, rarity::Rarity,
  rune::Rune, rune_id::RuneId, runestone::Runestone, sat::Sat, sat_point::SatPoint,
  spaced_rune::SpacedRune, terms::Terms,
};

pub const CYCLE_EPOCHS: u32 = 6;

fn default<T: Default>() -> T {
  Default::default()
}

mod artifact;
mod cenotaph;
mod charm;
mod decimal_sat;
mod edict;
mod epoch;
mod etching;
mod flaw;
mod height;
mod pile;
mod rarity;
mod rune;
mod rune_id;
mod runestone;
pub mod sat;
pub mod sat_point;
pub mod sats;
pub mod spaced_rune;
mod subsidy;
mod terms;
pub mod varint;
