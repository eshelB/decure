use cosmwasm_std::{HumanAddr, Order, StdError, StdResult, Storage, Uint128, KV};
use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::DisplayedReview;

pub static KEY_BUSINESSES: &[u8] = b"businesses";
pub static PREFIX_REVIEWS: &[u8] = b"reviews";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Business {
    pub name: String,
    pub description: String,
    pub address: HumanAddr,
    pub average_rating: u32, // max - maxint, min - 0
    pub reviews_count: u32,

    // todo - kept private (implement DisplayedBusiness)
    pub total_weight: Uint128,
}

pub fn create_business<S: Storage>(store: &mut S, business: Business) -> StdResult<()> {
    let mut all_businesses = bucket(KEY_BUSINESSES, store);
    let existing_business = all_businesses
        .may_load(business.address.as_str().as_bytes())
        .map_err(|_| StdError::generic_err("couldn't load businesses"))?;

    match existing_business {
        Some(..) => Err(StdError::generic_err(format!(
            "A business is already registered on that address",
        ))),
        None => {
            // todo remove print
            println!(
                "saving business {:?} on address {:?}",
                business, business.address
            );
            all_businesses.save(business.address.as_str().as_bytes(), &business)?;
            Ok(())
        }
    }
}

pub fn apply_review_on_business<S: Storage>(
    store: &mut S,
    business_address: HumanAddr,
    new_total_weight: u32,
    new_average_rating: u32,
    is_new: u8,
) -> StdResult<Business> {
    let mut all_businesses = bucket(KEY_BUSINESSES, store);
    all_businesses.update(
        business_address.as_str().as_bytes(),
        |business: Option<Business>| match business {
            Some(mut b) => {
                b.average_rating = new_average_rating;
                //todo unite casting types
                b.total_weight = Uint128::from(new_total_weight as u128);
                b.reviews_count += is_new as u32;
                Ok(b)
            }
            None => Err(StdError::generic_err(
                "Critical failure updating existing business",
            )),
        },
    )
}

// todo for testing purposes, remove in final version.
pub fn get_businesses_bucket<S: Storage>(store: &S) -> ReadonlyBucket<S, Business> {
    bucket_read(KEY_BUSINESSES, store)
}

pub fn get_businesses_page<S: Storage>(
    store: &S,
    start: Option<&[u8]>,
    end: Option<&[u8]>,
    page_size: u8,
) -> StdResult<(Vec<Business>, Uint128)> {
    let all_businesses = bucket_read(KEY_BUSINESSES, store);

    let businesses_page: Vec<Business> = all_businesses
        .range(start, end, Order::Ascending)
        .take(page_size as usize)
        .map(|b: StdResult<KV<Business>>| b.unwrap().1)
        .collect();

    let all_businesses_count = all_businesses.range(None, None, Order::Ascending).count();

    Ok((businesses_page, Uint128::from(all_businesses_count as u128)))
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
) -> StdResult<Option<Review>> {
    let reviews_on_business = ReadonlyBucket::multilevel(
        &[PREFIX_REVIEWS, business_address.as_str().as_bytes()],
        store,
    );

    reviews_on_business
        .may_load(&reviewer_address.as_str().as_bytes())
        .map_err(|_| StdError::generic_err("couldn't load review for business"))
}

pub fn create_review<S: Storage>(
    store: &mut S,
    business_address: &HumanAddr,
    reviewer_address: &HumanAddr,
    review: Review,
) -> StdResult<()> {
    let mut reviews_on_business: Bucket<S, Review> = Bucket::multilevel(
        &[PREFIX_REVIEWS, business_address.as_str().as_bytes()],
        store,
    );

    reviews_on_business
        .save(reviewer_address.as_str().as_bytes(), &review)
        .map_err(|_| StdError::generic_err("couldn't save review for business"))
}

pub fn get_reviews_on_business<S: Storage>(
    store: &S,
    business_address: &HumanAddr,
    start: Option<&[u8]>,
    end: Option<&[u8]>,
    page_size: u8,
) -> StdResult<(Vec<DisplayedReview>, Uint128)> {
    let reviews_on_business: ReadonlyBucket<S, Review> = ReadonlyBucket::multilevel(
        &[PREFIX_REVIEWS, business_address.as_str().as_bytes()],
        store,
    );

    let reviews_page: Vec<DisplayedReview> = reviews_on_business
        .range(start, end, Order::Ascending)
        .take(page_size as usize)
        .map(|b: StdResult<KV<Review>>| {
            let review = b.unwrap().1;
            DisplayedReview {
                title: review.title,
                content: review.content,
                rating: review.rating,
                last_update_timestamp: review.last_update_timestamp,
            }
        })
        .collect();

    let reviews_count = reviews_on_business
        .range(None, None, Order::Ascending)
        .count();

    Ok((reviews_page, Uint128::from(reviews_count as u128)))
}
