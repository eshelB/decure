use cosmwasm_std::{CanonicalAddr, StdError, StdResult, Storage, Uint128};
use cosmwasm_storage::{
    singleton, singleton_read, PrefixedStorage, ReadonlySingleton, Singleton, TypedStorage,
};
use schemars::JsonSchema;
use secret_toolkit::storage::AppendStoreMut;
use serde::{Deserialize, Serialize};

pub static PREFIX_BUSINESSES: &[u8] = b"businesses";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Business {
    pub name: String,
    pub description: String,
    pub average_rating: i32, // max - maxint-i32 min-0
    pub reviews_count: i32,
    pub total_weight: Uint128,
}

fn create_business<S: Storage>(
    store: &mut S,
    business: &Business,
    address: &CanonicalAddr,
) -> StdResult<()> {
    let mut store = PrefixedStorage::new(&PREFIX_BUSINESSES, store);
    let mut store = TypedStorage::<_, Business>::new(&mut store);

    let existing_business = store.may_load(address.as_slice())?;

    match existing_business {
        Some(..) => Err(StdError::generic_err(format!(
            "A business is already registered on that address",
        ))),
        None(..) => store.save(address.as_slice(), &business),
    }
}

fn get_business_by_address<S: Storage>(
    store: &mut S,
    address: &CanonicalAddr,
) -> StdResult<Option<Business>> {
    let mut store = PrefixedStorage::new(&PREFIX_BUSINESSES, store);
    let mut store = TypedStorage::<_, Business>::new(&mut store);

    store.may_load(address.as_slice())
}
