use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {}

#[derive(Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    RegisterBusiness {
        name: String,
        address: HumanAddr,
        description: String,
    },
    ReviewBusiness {
        address: HumanAddr,
        content: String,
        rating: u8,
        title: String,
        tx_id: u64,
        tx_page: u32,
        viewing_key: String,
    },
}

#[derive(Serialize, Clone, Debug, PartialEq, JsonSchema)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    RegisterBusiness { status: String },
    ReviewBusiness { status: String },
}

#[derive(Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBusinesses {
        page: Option<u32>,
        page_size: u32,
    },
    GetSingleBusiness {
        address: HumanAddr,
    },
    GetReviewsOnBusiness {
        business_address: HumanAddr,
        page: Option<u32>,
        page_size: u32,
    },
}

#[derive(Serialize, Clone, Debug, PartialEq, JsonSchema)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(rename_all = "snake_case")]
pub struct DisplayedReview {
    pub title: String,
    pub content: String,
    pub rating: u8, // 0 to 5
    pub last_update_timestamp: u64,
}

#[derive(Serialize, Clone, Debug, PartialEq, JsonSchema)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(rename_all = "snake_case")]
pub struct DisplayedBusiness {
    pub name: String,
    pub description: String,
    pub address: HumanAddr,
    pub average_rating: Uint128, // max - 5000, min - 0
    pub reviews_count: u32,
}

#[derive(Serialize, Clone, Debug, PartialEq, JsonSchema)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    Businesses {
        businesses: Vec<DisplayedBusiness>,
        total: u32,
    },
    SingleBusiness {
        business: Option<DisplayedBusiness>,
        status: String,
    },
    Reviews {
        reviews: Vec<DisplayedReview>,
        total: u32,
    },
}
