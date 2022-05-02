use cosmwasm_std::{StdError, StdResult};

const MAX_EXPANDED_VALUE: u128 = 5000;
const MAX_CHOOSABLE_VALUE: u128 = 5;

pub fn recalculate_weighted_average(
    my_added_weight: u128,
    my_previous_weight: u128,
    my_new_rating: u128,
    my_previous_rating: u128,

    previous_total_weight: u128,
    previous_average_rating: u128,
) -> StdResult<(u128, u128)> {
    let my_previous_rating_expanded = result_div(
        result_mul(my_previous_rating, MAX_EXPANDED_VALUE)?,
        MAX_CHOOSABLE_VALUE,
    )?;

    let my_new_rating_expanded = result_div(
        result_mul(my_new_rating, MAX_EXPANDED_VALUE)?,
        MAX_CHOOSABLE_VALUE,
    )?;

    let weight_without_me = result_sub(previous_total_weight, my_previous_weight)?;

    let mut rating_rest = 0;
    if weight_without_me != 0 {
        rating_rest = result_div(
            result_sub(
                result_mul(previous_average_rating, previous_total_weight)?,
                result_mul(my_previous_rating_expanded, my_previous_weight)?,
            )?,
            weight_without_me,
        )?;
    }

    let my_total_weigth = result_add(my_previous_weight, my_added_weight)?;
    let new_total_weight = result_add(weight_without_me, my_total_weigth)?;

    let mut new_average = 0;
    if new_total_weight != 0 {
        new_average = result_div(
            result_add(
                result_mul(rating_rest, weight_without_me)?,
                result_mul(my_new_rating_expanded, my_total_weigth)?,
            )?,
            new_total_weight,
        )?;
    }

    Ok((new_average, new_total_weight))
}

fn result_add(lhs: u128, rhs: u128) -> StdResult<u128> {
    lhs.checked_add(rhs)
        .ok_or(StdError::generic_err("overflow in addition"))
}

fn result_mul(lhs: u128, rhs: u128) -> StdResult<u128> {
    lhs.checked_mul(rhs)
        .ok_or(StdError::generic_err("overflow in multiplication"))
}

fn result_sub(lhs: u128, rhs: u128) -> StdResult<u128> {
    lhs.checked_sub(rhs)
        .ok_or(StdError::generic_err("underflow in subtraction"))
}

fn result_div(lhs: u128, rhs: u128) -> StdResult<u128> {
    lhs.checked_div(rhs)
        .ok_or(StdError::generic_err("underflow in division"))
}
