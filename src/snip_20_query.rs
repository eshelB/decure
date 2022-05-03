use cosmwasm_std::{HumanAddr, Querier, StdError, StdResult};
use secret_toolkit::snip20::{transfer_history_query, TransferHistory, Tx};

const QUERY_PAGE_SIZE: u32 = 10;
const SSCRT_HASH: &str = "E47144CD74E2E3E24275962CAA7719F081CCFA81A46532812596CA3D5BA6ECEB";
const SSCRT_ADDRESS: &str = "secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg";

pub fn query_snip20_tx<Q: Querier>(
    querier: &Q,
    tx_id: u64,
    viewing_key: String,
    tx_page: u32,
    requester_address: &HumanAddr,
) -> StdResult<Tx> {
    // address whose balance is being requested:

    let tx_history: TransferHistory = transfer_history_query(
        querier,
        requester_address.clone(),
        viewing_key,
        Some(tx_page),
        QUERY_PAGE_SIZE,
        256,
        SSCRT_HASH.to_string(),
        HumanAddr(SSCRT_ADDRESS.to_string()),
    )?;

    let specific_tx = tx_history.txs.iter().find(|&x| x.id == tx_id);
    match specific_tx {
        Some(tx) => Ok(tx.clone()),
        None => Err(StdError::generic_err(format!(
            "there was no transaction with id {} in the specified page",
            tx_id
        ))),
    }
}
