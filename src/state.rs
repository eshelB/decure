// use std::collections::{BTreeMap, HashMap};

use cosmwasm_std::{HumanAddr, StdError, StdResult, Storage, Uint128};
use cosmwasm_storage::{
    PrefixedStorage, ReadonlyPrefixedStorage, ReadonlyTypedStorage, TypedStorage,
};
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

pub fn initialize_businesses<S: Storage>(store: &mut S) -> StdResult<()> {
    // let mut store = TypedStorage::<_, BTreeMap<String, Business>>::new(store);
    // let mut businesses: BTreeMap<String, Business> = BTreeMap::new();
    // businesses.insert(
    //     "First Business".to_string(),
    //     Business {
    //         name: "first".to_string(),
    //         description: "desc".to_string(),
    //         average_rating: 0,
    //         reviews_count: 0,
    //         total_weight: Default::default(),
    //     },
    // );

    let mut store = TypedStorage::<_, Business>::new(store);
    store.save(
        KEY_BUSINESSES,
        &Business {
            name: "first".to_string(),
            description: "desc".to_string(),
            average_rating: 0,
            reviews_count: 0,
            total_weight: Default::default(),
        },
    )
}

pub fn create_business<S: Storage>(
    store: &mut S,
    business: Business,
    address: HumanAddr,
) -> StdResult<()> {
    let mut store = TypedStorage::<_, HashMap<String, Business>>::new(store);
    let mut businesses = store.load(KEY_BUSINESSES).map_err(|err| {
        StdError::generic_err(
            "Couldn't load businesses, probably the contract was not initialized correctly",
        )
    })?;

    let existing_business = businesses.get(address.as_str());

    match existing_business {
        Some(..) => Err(StdError::generic_err(format!(
            "A business is already registered on that address",
        ))),
        None => {
            // todo remove print
            println!("saving business {:?} on address {:?}", business, address);
            businesses.insert(address.as_str().to_string(), business);
            Ok(())
        }
    }
}

pub fn get_business_by_address<S: Storage>(
    store: &S,
    address: &HumanAddr,
) -> StdResult<Option<Business>> {
    let store = ReadonlyTypedStorage::<_, HashMap<String, Business>>::new(store);
    let businesses = store.load(KEY_BUSINESSES).unwrap();

    // todo remove print
    println!("getting business address {:?}", address);
    let requested_business = businesses.get(address.as_str());
    if let None = requested_business {
        Ok(None)
    } else {
        Ok(Some(requested_business.unwrap().clone()))
    }
}
