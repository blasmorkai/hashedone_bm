use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Decimal};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct State {
    pub donators: u64,
    pub incremental_donation: Coin,
    pub collective_ratio: Decimal,
    pub manager: Addr           //Manager is the donation-manager
}

pub const STATE: Item<State> = Item::new("state");
pub const OWNER: Item<Addr> = Item::new("owner");
