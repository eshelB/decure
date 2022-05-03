use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    QueryResult, StdError, StdResult, Storage, Uint128,
};

use crate::msg::{DisplayedBusiness, HandleAnswer, HandleMsg, InitMsg, QueryAnswer, QueryMsg};
use crate::snip_20_query::query_snip20_tx;
use crate::state::{
    apply_review_on_business, create_business, create_review, get_business_by_address,
    get_businesses_page, get_reviews_on_business, may_load_review, Business, Review,
};
use crate::utils::{recalculate_weighted_average, result_add};

// constants:
const MAX_DESCRIPTION_LENGTH: u8 = 40;
const MAX_NAME_LENGTH: u8 = 20;
const MIN_RATING: u8 = 0;
const MAX_RATING: u8 = 5;

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    let answer = match msg {
        HandleMsg::RegisterBusiness {
            name,
            address,
            description,
        } => register_business(deps, env, name, HumanAddr(address.to_string()), description)?,

        HandleMsg::ReviewBusiness {
            address,
            content,
            rating,
            title,
            tx_id,
            tx_page,
            viewing_key,
        } => review_business(
            deps,
            env,
            address,
            content,
            rating,
            title,
            tx_id,
            tx_page,
            viewing_key,
        )?,
    };

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&answer)?),
    })
}

fn review_business<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    address: HumanAddr,
    content: String,
    rating: u8,
    title: String,
    tx_id: u64,
    tx_page: u32,
    viewing_key: String,
) -> StdResult<HandleAnswer> {
    if rating < MIN_RATING || rating > MAX_RATING {
        return Err(StdError::generic_err(
            "ratings must be between 0 and 5 stars",
        ));
    }

    let mut status;

    let existing_business =
        get_business_by_address(&deps.storage, &address)?.ok_or(StdError::generic_err(
            "There is no business registered on that address. You can register it instead.",
        ))?;

    let previous_review = may_load_review(&deps.storage, &address, &env.message.sender);

    let mut increment_count: u8 = 0;
    if previous_review.is_none() {
        status = "Successfully added a new review on business".to_string();
        increment_count = 1;
    } else {
        status = "Successfully updated a previous review on business".to_string()
    }

    // this review will get overriden but it is useful as
    // a starting point for tx and weight accumulation
    let mut base_review = previous_review.unwrap_or(Review {
        title: "".to_string(),
        content: "".to_string(),
        rating: 0,
        weight: Uint128(0),
        tx_ids: vec![],
        last_update_timestamp: Default::default(),
    });

    let previous_weight = base_review.weight.u128();
    let previous_rating = base_review.rating;

    let mut new_weight_from_tx = 0;
    if !base_review.tx_ids.contains(&tx_id) {
        status.push_str(", receipt was accounted for");

        let tx = query_snip20_tx(
            &deps.querier,
            tx_id,
            viewing_key,
            tx_page,
            &env.message.sender,
        )?;

        if tx.sender != env.message.sender {
            return Err(StdError::generic_err(
                "The specified transfer was not spent by the authenticated account",
            ));
        }

        if tx.receiver != address {
            return Err(StdError::generic_err(
                "The specified transfer's recipient is not the specified business",
            ));
        }
        new_weight_from_tx = tx.coins.amount.u128();

        println!("tx_page {}", tx_page);
        base_review.weight =
            Uint128::from(result_add(base_review.weight.u128(), new_weight_from_tx)?);
        base_review.tx_ids.push(tx_id);
    } else {
        status.push_str(", specified receipt was already used");
    }

    base_review.title = title;
    base_review.content = content;
    base_review.rating = rating;

    create_review(
        &mut deps.storage,
        &address,
        &env.message.sender,
        base_review,
    )?;

    let (new_average, new_weight) = recalculate_weighted_average(
        new_weight_from_tx,
        previous_weight,
        rating as u128,
        previous_rating as u128,
        existing_business.total_weight.u128(),
        existing_business.average_rating.u128(),
    )?;

    apply_review_on_business(
        &mut deps.storage,
        address,
        new_weight,
        new_average,
        increment_count,
    )?;

    Ok(HandleAnswer::ReviewBusiness { status })
}

fn register_business<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    name: String,
    address: HumanAddr,
    description: String,
) -> StdResult<HandleAnswer> {
    if description.chars().count() as u8 > MAX_DESCRIPTION_LENGTH {
        return Err(StdError::generic_err(format!(
            "Description length can't be bigger than {}",
            MAX_DESCRIPTION_LENGTH
        )));
    }

    if name.chars().count() as u8 > MAX_NAME_LENGTH {
        return Err(StdError::generic_err(format!(
            "Name length can't be bigger than {}",
            MAX_NAME_LENGTH
        )));
    }

    // check that a correctly formatted address was given
    deps.api.canonical_address(&address)?;

    let new_business = Business {
        name,
        description,
        address: HumanAddr(address.to_string()),
        average_rating: Uint128::from(0u128),
        reviews_count: 0,
        total_weight: Uint128::from(0u128),
    };

    create_business(&mut deps.storage, new_business)?;

    Ok(HandleAnswer::RegisterBusiness {
        status: "successfully called register business".to_string(),
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::GetBusinesses { start, page_size } => {
            query_businesses(&deps.storage, start, page_size)
        }
        QueryMsg::GetSingleBusiness { address } => query_business(&deps.storage, address),
        QueryMsg::GetReviewsOnBusiness {
            business_address,
            start,
            page_size,
        } => query_reviews(&deps.storage, business_address, start, page_size),
    }
}

pub fn query_businesses<S: Storage>(
    store: &S,
    start: Option<u32>,
    page_size: u32,
) -> StdResult<Binary> {
    let (businesses_in_range, total) = get_businesses_page(store, start, page_size)?;
    let displayed_businesses = businesses_in_range
        .iter()
        .map(|b| DisplayedBusiness {
            name: b.name.clone(),
            description: b.description.clone(),
            address: b.address.clone(),
            average_rating: b.average_rating.clone(),
            reviews_count: b.reviews_count.clone(),
        })
        .collect();

    to_binary(&QueryAnswer::Businesses {
        businesses: displayed_businesses,
        total,
    })
}

pub fn query_business<S: Storage>(store: &S, address: HumanAddr) -> StdResult<Binary> {
    let business = get_business_by_address(store, &address)?;

    let status = match business {
        None => "No business is registered on that address".to_string(),
        Some(..) => "Successfully retrieved business by address".to_string(),
    };

    to_binary(&QueryAnswer::SingleBusiness {
        business: business.map(|b| DisplayedBusiness {
            name: b.name,
            description: b.description,
            address: b.address,
            average_rating: b.average_rating,
            reviews_count: b.reviews_count,
        }),
        status,
    })
}

pub fn query_reviews<S: Storage>(
    store: &S,
    business_address: HumanAddr,
    start: Option<u32>,
    page_size: u32,
) -> StdResult<Binary> {
    let (reviews_page, total) =
        get_reviews_on_business(store, &business_address, start, page_size)?;

    to_binary(&QueryAnswer::Reviews {
        reviews: reviews_page,
        total,
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary};
    use secret_toolkit::incubator::CashMap;

    use crate::msg::DisplayedReview;
    use crate::state::get_business_by_address;

    use super::*;

    #[test]
    fn register_business() -> StdResult<()> {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RegisterBusiness {
            name: "Starbucks".to_string(),
            description: "a place to eat".to_string(),
            address: HumanAddr("mock-address".to_string()),
        };
        let res = handle(&mut deps, env, msg);
        println!("res: {:?}", res);
        let res2 = res.unwrap();
        println!("res2: {:?}", res2);
        let res3 = res2.data;
        println!("res3: {:?}", res3);
        let res4 = res3.unwrap();
        println!("res4: {:?}", res4);
        let res5: StdResult<HandleAnswer> = from_binary(&res4);
        println!("res5: {:?}", res5);
        let res6: HandleAnswer = res5.unwrap();
        println!("res6: {:?}", res6);
        match res6 {
            HandleAnswer::RegisterBusiness { status } => {
                assert_eq!("successfully called register business", status);
                println!("success")
            }
            _ => panic!("got wrong answer variant"),
        }

        // check that the business was indeed saved
        let saved = get_business_by_address(&deps.storage, &HumanAddr("mock-address".to_string()));

        assert_eq!(
            saved.unwrap().unwrap(),
            Business {
                address: HumanAddr("mock-address".to_string()),
                name: "Starbucks".to_string(),
                description: "a place to eat".to_string(),
                average_rating: Uint128::from(0u128),
                reviews_count: 0,
                total_weight: Uint128(0)
            }
        );

        let mut all_businesses = CashMap::init(b"businesses", &mut deps.storage);
        all_businesses.insert(
            b"second",
            Business {
                address: HumanAddr("second".to_string()),
                name: "second".to_string(),
                description: "second".to_string(),
                average_rating: Uint128::from(0u128),
                reviews_count: 0,
                total_weight: Default::default(),
            },
        )?;

        all_businesses.insert(
            b"third",
            Business {
                address: HumanAddr("third".to_string()),
                name: "third".to_string(),
                description: "third".to_string(),
                average_rating: Uint128::from(0u128),
                reviews_count: 0,
                total_weight: Default::default(),
            },
        )?;

        all_businesses.insert(
            b"arthur",
            Business {
                address: HumanAddr("arthur".to_string()),
                name: "arthur".to_string(),
                description: "arthur the third".to_string(),
                average_rating: Uint128::from(0u128),
                reviews_count: 0,
                total_weight: Default::default(),
            },
        )?;

        // QUERY
        let msg = QueryMsg::GetBusinesses {
            start: Some(1),
            page_size: 2,
        };

        let res = query(&deps, msg);
        let res_unpacked: QueryAnswer = from_binary(&res.unwrap()).unwrap();
        match res_unpacked {
            QueryAnswer::Businesses { businesses, total } => {
                assert_eq!(total, 4);
                // println!("{:?}", businesses);
                assert_eq!(businesses[0].address.as_str(), "third");
                println!("success")
            }
            _ => panic!("wrong query variant"),
        }

        Ok(())
    }

    #[test]
    fn register_existing_business() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RegisterBusiness {
            name: "Starbucks".to_string(),
            description: "a place to eat".to_string(),
            address: HumanAddr("mock-address".to_string()),
        };
        let res = handle(&mut deps, env, msg);
        let res_unpacked = from_binary::<HandleAnswer>(&res.unwrap().data.unwrap()).unwrap();
        match res_unpacked {
            HandleAnswer::RegisterBusiness { status } => {
                assert_eq!("successfully called register business", status);
                println!("success")
            }
            _ => panic!("got wrong answer variant"),
        }

        // another business, should succeed
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RegisterBusiness {
            name: "Starbucks".to_string(),
            description: "a place to eat".to_string(),
            address: HumanAddr("another-address".to_string()),
        };
        let res = handle(&mut deps, env, msg);
        let res_unpacked = from_binary::<HandleAnswer>(&res.unwrap().data.unwrap()).unwrap();
        match res_unpacked {
            HandleAnswer::RegisterBusiness { status } => {
                assert_eq!("successfully called register business", status);
                println!("success")
            }
            _ => panic!("got wrong answer variant"),
        }

        // again, should fail
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RegisterBusiness {
            name: "Starbucks".to_string(),
            description: "a place to eat".to_string(),
            address: HumanAddr("mock-address".to_string()),
        };
        let res = handle(&mut deps, env, msg).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err("A business is already registered on that address")
        );
    }

    #[test]
    fn review_unregistered_business() -> StdResult<()> {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::ReviewBusiness {
            address: HumanAddr("mock-address".to_string()),
            content: "very enjoyable time at this place".to_string(),
            rating: 5,
            title: "Fantastic!".to_string(),
            tx_id: 0,
            tx_page: 0,
            viewing_key: "vk".to_string(),
        };

        let res = handle(&mut deps, env, msg);
        let error = res.unwrap_err();

        if let StdError::GenericErr { msg, .. } = error {
            assert_eq!(
                "There is no business registered on that address. You can register it instead.",
                msg
            )
        } else {
            panic!("there should be a generic error here")
        }

        Ok(())
    }

    #[test]
    fn review_rating_out_of_bounds() -> StdResult<()> {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RegisterBusiness {
            name: "Starbucks".to_string(),
            description: "a place to eat".to_string(),
            address: HumanAddr("mock-address".to_string()),
        };
        handle(&mut deps, env, msg)?;

        // 1st review
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::ReviewBusiness {
            address: HumanAddr("mock-address".to_string()),
            content: "very enjoyable time at this place".to_string(),
            rating: 6,
            title: "Fantastic!".to_string(),
            tx_id: 0,
            tx_page: 0,
            viewing_key: "vk".to_string(),
        };

        let res = handle(&mut deps, env, msg);
        let error = res.unwrap_err();

        if let StdError::GenericErr { msg, .. } = error {
            assert_eq!("ratings must be between 0 and 5 stars", msg);
        } else {
            panic!("there should be a generic error here");
        }

        Ok(())
    }

    #[test]
    fn register_business_long_name() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RegisterBusiness {
            name: "NameIs21Characters...".to_string(),
            description: "a place to eat".to_string(),
            address: HumanAddr("mock-address".to_string()),
        };

        let res = handle(&mut deps, env, msg);
        let error = res.unwrap_err();

        if let StdError::GenericErr { msg, .. } = error {
            assert_eq!("Name length can't be bigger than 20", msg)
        } else {
            panic!("there should be a generic error here")
        }
    }

    #[test]
    fn register_business_long_description() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RegisterBusiness {
            name: "Scrt Labs".to_string(),
            description: "DescriptionIs43CharactersLongWhichIsTooMuch".to_string(),
            address: HumanAddr("mock-address".to_string()),
        };

        let res = handle(&mut deps, env, msg);
        let error = res.unwrap_err();

        if let StdError::GenericErr { msg, .. } = error {
            assert_eq!("Description length can't be bigger than 40", msg)
        } else {
            panic!("there should be a generic error here")
        }
    }
}
