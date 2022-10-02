use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Decimal};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub peer_code_id:u64,
    pub incremental_donation: Coin,
    pub collective_ratio: Decimal
}

pub const CONFIG: Item<Config> = Item::new("config");

// peer address -> owner_address
pub const MEMBERS: Map<Addr,Addr> = Map::new("members");

pub const PENDING_INSTANTIATION : Item<Addr> = Item::new("pending_instantiation");
