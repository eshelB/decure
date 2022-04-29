// use std::collections::{BTreeMap, HashMap};

use cosmwasm_std::{HumanAddr, StdError, StdResult, Storage, Uint128};
use cosmwasm_std_regular::Storage as Cstorage;
use cosmwasm_storage::{
    PrefixedStorage, ReadonlyPrefixedStorage, ReadonlyTypedStorage, TypedStorage,
};
use cw_storage_plus::Map;
use schemars::JsonSchema;
// use secret_toolkit::storage::AppendStoreMut;
use serde::{Deserialize, Serialize};

pub static KEY_BUSINESSES: &[u8] = b"businesses";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Business {
    pub name: String,
    pub description: String,
    pub average_rating: i32, // max - maxint-i32 min-0
    pub reviews_count: i32,
    pub total_weight: Uint128,
}

const BUSINESSES: Map<&str, Business> = Map::new("KEY_BUSINESSES");

// pub fn create_business<S: Storage>(
pub fn create_business<S: Storage + Cstorage>(
    store: &mut S,
    business: Business,
    address: HumanAddr,
) -> StdResult<()> {
    let existing_business = BUSINESSES.may_load(store, address.as_str());

    match existing_business {
        Some(..) => Err(StdError::generic_err(format!(
            "A business is already registered on that address",
        ))),
        None => {
            // todo remove print
            println!("saving business {:?} on address {:?}", business, address);
            BUSINESSES.save(store, address.as_str(), &business);
            Ok(())
        }
    }
}

pub fn get_business_by_address<S: Storage>(
    store: &S,
    address: &HumanAddr,
) -> StdResult<Option<Business>> {
    let existing_business = BUSINESSES.may_load(store, address.as_str());

    // todo remove print
    println!("getting business address {:?}", address);
    Ok(existing_business.unwrap())
}
