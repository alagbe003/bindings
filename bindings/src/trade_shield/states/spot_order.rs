use crate::trade_shield::types::SpotOrder;
use cw_storage_plus::{Item, Map};

pub const SPOT_ORDER: Map<u64, SpotOrder> = Map::new("spot order");

pub const SPOT_ORDER_MAX_ID: Item<u64> = Item::new("spot order max id");

pub const PENDING_SPOT_ORDER: Map<u64, SpotOrder> = Map::new("unprocess spot order");
