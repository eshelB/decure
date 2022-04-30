use cosmwasm_std::{HumanAddr, StdError, StdResult, Storage, Uint128};
use cosmwasm_storage::{bucket, bucket_read, ReadonlyBucket};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static KEY_BUSINESSES: &[u8] = b"businesses";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Business {
    pub name: String,
    pub description: String,
    pub average_rating: i32, // max - maxint, min - 0
    pub reviews_count: i32,
    pub total_weight: Uint128,
}

pub fn create_business<S: Storage>(
    store: &mut S,
    business: Business,
    address: HumanAddr,
) -> StdResult<()> {
    let mut all_businesses = bucket(KEY_BUSINESSES, store);
    let existing_business = all_businesses
        .may_load(address.as_str().as_bytes())
        .map_err(|_| StdError::generic_err("couldn't load businesses"))?;

    match existing_business {
        Some(..) => Err(StdError::generic_err(format!(
            "A business is already registered on that address",
        ))),
        None => {
            // todo remove print
            println!("saving business {:?} on address {:?}", business, address);
            all_businesses.save(address.as_str().as_bytes(), &business)?;
            Ok(())
        }
    }
}

pub fn get_businesses_bucket<S: Storage>(store: &S) -> ReadonlyBucket<S, Business> {
    bucket_read(KEY_BUSINESSES, store)
}

pub fn get_business_by_address<S: Storage>(
    store: &S,
    address: &HumanAddr,
) -> StdResult<Option<Business>> {
    let all_businesses = bucket_read(KEY_BUSINESSES, store);
    let existing_business = all_businesses.may_load(address.as_str().as_bytes());

    println!("getting business address {:?}", address);
    existing_business
}
