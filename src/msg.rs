use crate::state::Business;
use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    pub count: i32,
}

#[derive(Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    RegisterBusiness {
        name: String,
        address: HumanAddr,
        description: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    RegisterBusiness { status: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBusinesses {
        page: Option<Uint128>,
        page_size: Uint128,
    },
}

#[derive(Serialize, Clone, Debug, PartialEq, JsonSchema)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    Businesses {
        businesses: Vec<Business>,
        total: Option<Uint128>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CountResponse {
    pub count: String,
}
