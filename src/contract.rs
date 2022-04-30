use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    StdError, StdResult, Storage, Uint128,
};
use secret_toolkit::snip20::{transfer_history_query, TransferHistory};

use crate::msg::{CountResponse, HandleAnswer, HandleMsg, InitMsg, QueryMsg};
use crate::state::{create_business, Business};

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
    };

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&answer)?),
    })
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
        average_rating: 0,
        reviews_count: 0,
        total_weight: Uint128(0),
    };

    create_business(&mut deps.storage, new_business, address)?;

    Ok(HandleAnswer::RegisterBusiness {
        status: "successfully called register business".to_string(),
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::WithPermit { permit, query } => permit_queries(deps, permit, query),
        // todo remove
        // _ => Ok(Binary(vec![2u8])),
    }
}

fn query_count<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<CountResponse> {
    // address whose balance is being requested:
    let address = HumanAddr("secret1ap26qrlp8mcq2pg6r47w43l0y8zkqm8a450s03".to_string());
    let key = "vk".to_string();
    let block_size = 256;
    let callback_code_hash =
        "E47144CD74E2E3E24275962CAA7719F081CCFA81A46532812596CA3D5BA6ECEB".to_string();
    let contract_addr = HumanAddr("secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg".to_string());

    // let balance: Balance =
    //     balance_query(&deps.querier, address, key, block_size, callback_code_hash, contract_addr)?;

    // let balance_s = format!("the balance returned from the query is {:?}", balance.amount.u128());

    let page = 0u32;
    let page_size = 2u32;
    let tx_history: TransferHistory = transfer_history_query(
        &deps.querier,
        address,
        key,
        Some(page),
        page_size,
        block_size,
        callback_code_hash,
        contract_addr,
    )?;

    let id_to_find = 2;
    let specific_tx = tx_history.txs.iter().find(|&x| x.id == id_to_find);
    let tx_history_s = match specific_tx {
        Some(tx) => format!(
            "the tx with id {} from the query is {:?}, and its amount is {:?}",
            id_to_find,
            tx,
            tx.coins.amount.u128()
        ),
        None => "there was no such transaction in the given page".to_string(),
    };

    Ok(CountResponse {
        count: tx_history_s,
    })
}

#[cfg(test)]
mod tests {
    use crate::msg::QueryAnswer::Businesses;
    use crate::state::{get_business_by_address, get_businesses_bucket};
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, Order, KV};
    use cosmwasm_storage::bucket;

    use super::*;

    #[test]
    fn register_business() {
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
        }

        // check that the business was indeed saved
        let saved = get_business_by_address(&deps.storage, &HumanAddr("mock-address".to_string()));

        assert_eq!(
            saved.unwrap().unwrap(),
            Business {
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
                name: "second".to_string(),
                description: "second".to_string(),
                average_rating: 0,
                reviews_count: 0,
                total_weight: Default::default(),
            },
        );

        all_businesses.save(
            b"third",
            &Business {
                name: "third".to_string(),
                description: "third".to_string(),
                average_rating: 0,
                reviews_count: 0,
                total_weight: Default::default(),
            },
        );

        all_businesses.save(
            b"arthur",
            &Business {
                name: "arthur".to_string(),
                description: "arthur the third".to_string(),
                average_rating: 0,
                reviews_count: 0,
                total_weight: Default::default(),
            },
        );

        let vecbus: StdResult<Vec<KV<Business>>> =
            // all_businesses.range(None, None, Order::Ascending).collect();
            // all_businesses.range(Some(b"secone"), None, Order::Ascending).collect();
            all_businesses.range(None, None, Order::Ascending).collect();

        println!("{:?}", vecbus);
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
