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
