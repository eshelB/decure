use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Querier, StdError, StdResult, Storage,
};
use secret_toolkit::snip20::{transfer_history_query, TransferHistory};

use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, State};

// use secret_toolkit::snip20::{transaction_history_query, TransactionHistory};
// use secret_toolkit::snip20::{balance_query, Balance};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        count: msg.count,
        owner: deps.api.canonical_address(&env.message.sender)?,
    };

    config(&mut deps.storage).save(&state)?;

    debug_print!("Contract was initialized by {}", env.message.sender);

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Increment {} => try_increment(deps, env),
        HandleMsg::Reset { count } => try_reset(deps, env, count),
    }
}

pub fn try_increment<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
) -> StdResult<HandleResponse> {
    config(&mut deps.storage).update(|mut state| {
        state.count += 1;
        debug_print!("count = {}", state.count);
        Ok(state)
    })?;

    debug_print("count incremented successfully");
    Ok(HandleResponse::default())
}

pub fn try_reset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    count: i32,
) -> StdResult<HandleResponse> {
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    config(&mut deps.storage).update(|mut state| {
        if sender_address_raw != state.owner {
            return Err(StdError::Unauthorized { backtrace: None });
        }
        state.count = count;
        Ok(state)
    })?;
    debug_print("count reset successfully");
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
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
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    use super::*;

    #[test]
    fn increment() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg { count: 17 };
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        // anyone can increment
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::Increment {};
        let _res = handle(&mut deps, env, msg).unwrap();

        // should increase counter by 1
        let res = query(&deps, QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        println!("the response is {:?}", value.count);
        assert_eq!("This is an example response", value.count);
    }
}
