use cosmwasm_std::{HumanAddr, ReadonlyStorage, StdError, StdResult, Storage, Uint128};
use schemars::JsonSchema;
use secret_toolkit::incubator::{CashMap, ReadOnlyCashMap};
use serde::{Deserialize, Serialize};

use crate::msg::DisplayedReview;

pub static KEY_BUSINESSES: &[u8] = b"businesses";
pub static PREFIX_REVIEWS: &str = "reviews";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Business {
    pub name: String,
    pub description: String,
    pub address: HumanAddr,
    pub average_rating: Uint128, // max - 5000, min - 0
    pub reviews_count: u32,

    pub total_weight: Uint128,
}

pub fn create_business<S: Storage>(store: &mut S, business: Business) -> StdResult<()> {
    let mut all_businesses = CashMap::init(KEY_BUSINESSES, store);
    let existing_business: Option<Business> =
        all_businesses.get(business.address.as_str().as_bytes());

    match existing_business {
        Some(..) => Err(StdError::generic_err(format!(
            "A business is already registered on that address",
        ))),
        None => {
            all_businesses.insert(business.address.as_str().as_bytes(), business.clone())?;
            Ok(())
        }
    }
}

pub fn apply_review_on_business<S: Storage>(
    store: &mut S,
    business_address: HumanAddr,
    new_total_weight: u128,
    new_average_rating: u128,
    is_new: u8,
) -> StdResult<()> {
    let mut all_businesses = CashMap::init(KEY_BUSINESSES, store);
    let business: Option<Business> = all_businesses.get(business_address.as_str().as_bytes());

    match business {
        Some(mut b) => {
            b.average_rating = Uint128::from(new_average_rating);
            //todo unite casting types
            b.total_weight = Uint128::from(new_total_weight);
            b.reviews_count += is_new as u32;
            all_businesses.insert(business_address.as_str().as_bytes(), b)?;
            Ok(())
        }
        None => Err(StdError::generic_err(
            "Critical failure updating existing business",
        )),
    }
}

pub fn get_businesses_page<S: ReadonlyStorage>(
    store: &S,
    start: Option<u32>,
    page_size: u32,
) -> StdResult<(Vec<Business>, u32)> {
    let all_businesses = ReadOnlyCashMap::init(KEY_BUSINESSES, store);

    let businesses_page: Vec<Business> = all_businesses.paging(start.unwrap_or(0), page_size)?;
    let businesses_len: u32 = all_businesses.len();

    Ok((businesses_page, businesses_len))
}

pub fn get_business_by_address<S: ReadonlyStorage>(
    store: &S,
    address: &HumanAddr,
) -> StdResult<Option<Business>> {
    let all_businesses = ReadOnlyCashMap::init(KEY_BUSINESSES, store);
    let existing_business = all_businesses.get(address.as_str().as_bytes());

    println!("getting business address {:?}", address);
    Ok(existing_business)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Review {
    pub title: String,
    pub content: String,
    pub rating: u8,                     // 0 to 5
    pub last_update_timestamp: Uint128, // todo implement

    // kept private
    pub weight: Uint128,
    pub tx_ids: Vec<u64>,
}

pub fn may_load_review<S: Storage>(
    store: &S,
    business_address: &HumanAddr,
    reviewer_address: &HumanAddr,
) -> Option<Review> {
    let mut namespace = String::from(PREFIX_REVIEWS);
    namespace.push_str(business_address.as_str());
    let namespace: &[u8] = namespace.as_bytes();

    let reviews_on_business: ReadOnlyCashMap<Review, S> = ReadOnlyCashMap::init(namespace, store);

    reviews_on_business.get(&reviewer_address.as_str().as_bytes())
}

pub fn create_review<S: Storage>(
    store: &mut S,
    business_address: &HumanAddr,
    reviewer_address: &HumanAddr,
    review: Review,
) -> StdResult<()> {
    let mut namespace = String::from(PREFIX_REVIEWS);
    namespace.push_str(business_address.as_str());
    let namespace: &[u8] = namespace.as_bytes();

    let mut reviews_on_business: CashMap<Review, S> = CashMap::init(namespace, store);

    reviews_on_business
        .insert(reviewer_address.as_str().as_bytes(), review)
        .map_err(|_| StdError::generic_err("couldn't save review for business"))
}

pub fn get_reviews_on_business<S: Storage>(
    store: &S,
    business_address: &HumanAddr,
    start: Option<u32>,
    page_size: u32,
) -> StdResult<(Vec<DisplayedReview>, u32)> {
    let mut namespace = String::from(PREFIX_REVIEWS);
    namespace.push_str(business_address.as_str());
    let namespace: &[u8] = namespace.as_bytes();

    let reviews_on_business: ReadOnlyCashMap<Review, S> = ReadOnlyCashMap::init(namespace, store);
    let reviews_page: Vec<Review> = reviews_on_business.paging(start.unwrap_or(0), page_size)?;

    let displayed_page: Vec<DisplayedReview> = reviews_page
        .iter()
        .map(|review: &Review| DisplayedReview {
            title: review.title.clone(),
            content: review.content.clone(),
            rating: review.rating.clone(),
            last_update_timestamp: review.last_update_timestamp,
        })
        .collect();

    let reviews_count = reviews_on_business.len();

    Ok((displayed_page, reviews_count))
}
