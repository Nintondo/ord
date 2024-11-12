use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Traits {
  #[arg(help = "Show traits for <SAT>.")]
  sat: Sat,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Output {
  pub number: u64,
  pub decimal: String,
  pub name: String,
  pub height: u32,
  pub cycle: u32,
  pub epoch: u32,
  pub offset: u64,
  pub rarity: Rarity,
}

impl Traits {
  pub(crate) fn run(self, settings: Settings) -> SubcommandResult {
    Ok(Some(Box::new(Output {
      number: self.sat.n(),
      decimal: self.sat.decimal(settings.chain().network()).to_string(),
      name: self.sat.name(),
      height: self.sat.height().0,
      cycle: self.sat.cycle(),
      epoch: self.sat.epoch().0,
      offset: self.sat.third(settings.chain().network()),
      rarity: self.sat.rarity(),
    })))
  }
}
