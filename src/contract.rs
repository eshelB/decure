use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    QueryResult, StdError, StdResult, Storage, Uint128,
};

use crate::msg::{DisplayedBusiness, HandleAnswer, HandleMsg, InitMsg, QueryAnswer, QueryMsg};
use crate::state::{
    apply_review_on_business, create_business, create_review, get_business_by_address,
    get_businesses_page, get_reviews_on_business, may_load_review, Business, Review,
};
use crate::utils::recalculate_weighted_average;

// todo use
// use secret_toolkit::snip20::{transfer_history_query, TransferHistory};

// constants:
const MAX_DESCRIPTION_LENGTH: u8 = 40;
const MAX_NAME_LENGTH: u8 = 20;

// use secret_toolkit::snip20::{transaction_history_query, TransactionHistory};
// use secret_toolkit::snip20::{balance_query, Balance};

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    // initialize_businesses(&mut deps.storage);

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
        } => review_business(deps, env, address, content, rating, title, tx_id, tx_page)?,
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
    tx_page: u64,
) -> StdResult<HandleAnswer> {
    let mut status;

    let existing_business =
        get_business_by_address(&deps.storage, &address)?.ok_or(StdError::generic_err(
            "There is no business registered on that address. You can register it instead.",
        ))?;

    let previous_review = may_load_review(&deps.storage, &address, &env.message.sender)?;

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

        // todo query the snip-20, if fails - no update
        new_weight_from_tx = 20;

        println!("{}", tx_page);
        base_review.weight = Uint128::from(base_review.weight.u128() + new_weight_from_tx);
        base_review.tx_ids.push(1);
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
        // todo verify casting
        rating as u128,
        previous_rating as u128,
        existing_business.total_weight.u128(),
        existing_business.average_rating as u128,
    )?;

    apply_review_on_business(
        &mut deps.storage,
        address,
        // todo verify casting
        new_weight as u32,
        new_average as u32,
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
        average_rating: 0,
        reviews_count: 0,
        total_weight: Uint128(0),
    };

    create_business(&mut deps.storage, new_business)?;

    Ok(HandleAnswer::RegisterBusiness {
        status: "successfully called register business".to_string(),
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::GetBusinesses {
            start,
            end,
            page_size,
        } => query_businesses(&deps.storage, start, end, page_size),
        QueryMsg::GetSingleBusiness { address } => query_business(&deps.storage, address),
        QueryMsg::GetReviewsOnBusiness {
            business_address,
            start,
            end,
            page_size,
        } => query_reviews(&deps.storage, business_address, start, end, page_size),
    }
}

pub fn query_businesses<S: Storage>(
    store: &S,
    start: Option<String>,
    end: Option<String>,
    page_size: u8,
) -> StdResult<Binary> {
    let start = start.as_ref().map(|x| x.as_bytes());
    let end = end.as_ref().map(|x| x.as_bytes());

    let (businesses_in_range, total) = get_businesses_page(store, start, end, page_size)?;

    to_binary(&QueryAnswer::Businesses {
        businesses: businesses_in_range,
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
    start: Option<String>,
    end: Option<String>,
    page_size: u8,
) -> StdResult<Binary> {
    let start = start.as_ref().map(|x| x.as_bytes());
    let end = end.as_ref().map(|x| x.as_bytes());

    let (reviews_page, total) =
        get_reviews_on_business(store, &business_address, start, end, page_size)?;

    to_binary(&QueryAnswer::Reviews {
        reviews: reviews_page,
        total,
    })
}

// todo use POC of looking up receipt.
// fn query_count<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<CountResponse> {
//     // address whose balance is being requested:
//     let address = HumanAddr("secret1ap26qrlp8mcq2pg6r47w43l0y8zkqm8a450s03".to_string());
//     let key = "vk".to_string();
//     let block_size = 256;
//     let callback_code_hash =
//         "E47144CD74E2E3E24275962CAA7719F081CCFA81A46532812596CA3D5BA6ECEB".to_string();
//     let contract_addr = HumanAddr("secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg".to_string());
//
//     // let balance: Balance =
//     //     balance_query(&deps.querier, address, key, block_size, callback_code_hash, contract_addr)?;
//
//     // let balance_s = format!("the balance returned from the query is {:?}", balance.amount.u128());
//
//     let page = 0u32;
//     let page_size = 2u32;
//     let tx_history: TransferHistory = transfer_history_query(
//         &deps.querier,
//         address,
//         key,
//         Some(page),
//         page_size,
//         block_size,
//         callback_code_hash,
//         contract_addr,
//     )?;
//
//     let id_to_find = 2;
//     let specific_tx = tx_history.txs.iter().find(|&x| x.id == id_to_find);
//     let tx_history_s = match specific_tx {
//         Some(tx) => format!(
//             "the tx with id {} from the query is {:?}, and its amount is {:?}",
//             id_to_find,
//             tx,
//             tx.coins.amount.u128()
//         ),
//         None => "there was no such transaction in the given page".to_string(),
//     };
//
//     Ok(CountResponse {
//         count: tx_history_s,
//     })
// }

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, Order, KV};
    use cosmwasm_storage::bucket;

    use crate::msg::DisplayedReview;
    use crate::state::{get_business_by_address, get_businesses_bucket};

    use super::*;

    #[test]
    fn register_business() -> StdResult<()> {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg { count: 17 };
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RegisterBusiness {
            name: "Starbucks".to_string(),
            description: "a place to eat".to_string(),
            address: HumanAddr("mock-address".to_string()),
        };
        let res = handle(&mut deps, env, msg);
        // println!("res: {:?}", res);
        let res2 = res.unwrap();
        // println!("res2: {:?}", res2);
        let res3 = res2.data;
        // println!("res3: {:?}", res3);
        let res4 = res3.unwrap();
        // println!("res4: {:?}", res4);
        let res5: StdResult<HandleAnswer> = from_binary(&res4);
        // println!("res5: {:?}", res5);
        let res6: HandleAnswer = res5.unwrap();
        // println!("res6: {:?}", res6);
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
                average_rating: 0,
                reviews_count: 0,
                total_weight: Uint128(0)
            }
        );

        // try range of businesses
        // todo remove
        let all_businesses = get_businesses_bucket(&deps.storage);
        let vecbus: StdResult<Vec<KV<Business>>> =
            all_businesses.range(None, None, Order::Ascending).collect();
        assert_eq!(
            vecbus.unwrap(),
            vec!((
                b"mock-address".to_vec(),
                Business {
                    name: "Starbucks".to_string(),
                    address: HumanAddr("mock-address".to_string()),
                    description: "a place to eat".to_string(),
                    average_rating: 0,
                    reviews_count: 0,
                    total_weight: Uint128(0)
                }
            ))
        );

        let mut all_businesses = bucket(b"businesses", &mut deps.storage);
        all_businesses.save(
            b"second",
            &Business {
                address: HumanAddr("second".to_string()),
                name: "second".to_string(),
                description: "second".to_string(),
                average_rating: 0,
                reviews_count: 0,
                total_weight: Default::default(),
            },
        )?;

        all_businesses.save(
            b"third",
            &Business {
                address: HumanAddr("third".to_string()),
                name: "third".to_string(),
                description: "third".to_string(),
                average_rating: 0,
                reviews_count: 0,
                total_weight: Default::default(),
            },
        )?;

        all_businesses.save(
            b"arthur",
            &Business {
                address: HumanAddr("arthur".to_string()),
                name: "arthur".to_string(),
                description: "arthur the third".to_string(),
                average_rating: 0,
                reviews_count: 0,
                total_weight: Default::default(),
            },
        )?;

        // let vecbus: StdResult<Vec<KV<Business>>> =
        //     // all_businesses.range(None, None, Order::Ascending).collect();
        //     // all_businesses.range(Some(b"secone"), None, Order::Ascending).collect();
        //     all_businesses.range(None, None, Order::Ascending).collect();

        // QUERY
        let msg = QueryMsg::GetBusinesses {
            start: Some("third".to_string()), //only last element
            end: None,
            page_size: 2,
        };

        let res = query(&deps, msg);
        let res_unpacked: QueryAnswer = from_binary(&res.unwrap()).unwrap();
        match res_unpacked {
            QueryAnswer::Businesses { businesses, total } => {
                assert_eq!(total.u128(), 4);
                // println!("{:?}", businesses);
                assert_eq!(businesses.len(), 1);
                println!("success")
            }
            _ => panic!("wrong query variant"),
        }

        let msg = QueryMsg::GetBusinesses {
            start: None,
            end: Some("third".to_string()), //up to "second" element, (which is 3rd)
            page_size: 4,
        };

        let res = query(&deps, msg);
        let res_unpacked: QueryAnswer = from_binary(&res.unwrap()).unwrap();
        match res_unpacked {
            QueryAnswer::Businesses { businesses, total } => {
                assert_eq!(total.u128(), 4);
                println!("end query {:?}", businesses);
                assert_eq!(businesses.len(), 3);
                println!("success")
            }
            _ => panic!("wrong query variant"),
        }

        Ok(())
    }

    #[test]
    fn register_existing_business() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg { count: 17 };
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

        let msg = InitMsg { count: 17 };
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
    fn review() -> StdResult<()> {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg { count: 17 };
        let env = mock_env("creator", &coins(2, "token"));
        init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RegisterBusiness {
            name: "Starbucks".to_string(),
            description: "a place to eat".to_string(),
            address: HumanAddr("mock-address".to_string()),
        };
        handle(&mut deps, env, msg)?;

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RegisterBusiness {
            name: "Starbucks".to_string(),
            description: "a place to eat".to_string(),
            address: HumanAddr("another-address".to_string()),
        };
        handle(&mut deps, env, msg)?;

        // 1st review
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::ReviewBusiness {
            address: HumanAddr("mock-address".to_string()),
            content: "very enjoyable time at this place".to_string(),
            rating: 5,
            title: "Fantastic!".to_string(),
            tx_id: 0,
            tx_page: 0,
        };

        let res = handle(&mut deps, env, msg);
        let res_unpacked = from_binary::<HandleAnswer>(&res.unwrap().data.unwrap()).unwrap();
        match res_unpacked {
            HandleAnswer::ReviewBusiness { status } => {
                assert_eq!(
                    "Successfully added a new review on business, receipt was accounted for",
                    status
                );
                println!("success")
            }
            _ => panic!("got wrong answer variant"),
        }

        // 2nd review, another business, should not appear
        let env = mock_env("anyone-2", &coins(2, "token"));
        let msg = HandleMsg::ReviewBusiness {
            address: HumanAddr("another-address".to_string()),
            content: "this one's in another business, should not come back".to_string(),
            rating: 5,
            title: "Fantastic2!".to_string(),
            tx_id: 0,
            tx_page: 0,
        };
        handle(&mut deps, env, msg)?;

        // 3rd review, another reviewer
        let env = mock_env("anyone-2", &coins(2, "token"));
        let msg = HandleMsg::ReviewBusiness {
            address: HumanAddr("mock-address".to_string()),
            content: "very enjoyable time at this place".to_string(),
            rating: 3,
            title: "Fantastic-3!".to_string(),
            tx_id: 0,
            tx_page: 0,
        };
        handle(&mut deps, env, msg)?;

        // 4th review - This one should only update the first review, since it's the same sender
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::ReviewBusiness {
            address: HumanAddr("mock-address".to_string()),
            content: "changed my mind, 4 instead of 5".to_string(),
            rating: 4,
            title: "Fantastic-4!".to_string(),
            tx_id: 0,
            tx_page: 0,
        };
        handle(&mut deps, env, msg)?;

        let msg = QueryMsg::GetReviewsOnBusiness {
            business_address: HumanAddr("mock-address".to_string()),
            start: None,
            end: None,
            page_size: 4,
        };

        let res = query(&deps, msg);
        let res_unpacked: QueryAnswer = from_binary(&res.unwrap()).unwrap();
        match res_unpacked {
            QueryAnswer::Reviews { reviews, total } => {
                assert_eq!(total.u128(), 2);
                println!("reviews query {:?}", reviews);
                assert_eq!(reviews.len(), 2);
                assert_eq!(
                    reviews,
                    vec![
                        DisplayedReview {
                            content: "changed my mind, 4 instead of 5".to_string(),
                            rating: 4,
                            title: "Fantastic-4!".to_string(),

                            // todo update timestamp
                            last_update_timestamp: Default::default(),
                        },
                        DisplayedReview {
                            content: "very enjoyable time at this place".to_string(),
                            rating: 3,
                            title: "Fantastic-3!".to_string(),

                            // todo update timestamp
                            last_update_timestamp: Default::default(),
                        }
                    ]
                );
                println!("success")
            }
            _ => panic!("wrong query variant"),
        }

        let msg = QueryMsg::GetSingleBusiness {
            address: HumanAddr("mock-address".to_string()),
        };
        let res = query(&deps, msg);
        let res_unpacked: QueryAnswer = from_binary(&res.unwrap()).unwrap();
        match res_unpacked {
            QueryAnswer::SingleBusiness { business, status } => {
                assert_eq!(
                    business.unwrap(),
                    DisplayedBusiness {
                        name: "Starbucks".to_string(),
                        description: "a place to eat".to_string(),
                        address: HumanAddr("mock-address".to_string()),
                        average_rating: 3666, // the weight of 4-star reviews is twice the weight of 3-star reviews
                        reviews_count: 2,
                    }
                );
                assert_eq!(status, "Successfully retrieved business by address");
            }
            _ => panic!("wrong query variant returned"),
        }

        Ok(())
    }

    #[test]
    fn register_business_long_name() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg { count: 17 };
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

        let msg = InitMsg { count: 17 };
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
