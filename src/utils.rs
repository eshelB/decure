use cosmwasm_std::StdResult;

const MAX_VALUE: u128 = 5000;

pub fn recalculate_weighted_average(
    my_added_weight: u128,
    my_previous_weight: u128,
    my_new_rating: u128,
    my_previous_rating: u128,

    previous_total_weight: u128,
    previous_average_rating: u128,
) -> StdResult<(u128, u128)> {
    //todo convert to checked_func
    let my_previous_rating_expanded = my_previous_rating * MAX_VALUE / 5;
    let my_new_rating_expanded = my_new_rating * MAX_VALUE / 5;

    let weight_without_me = previous_total_weight - my_previous_weight;

    let mut rating_rest = 0;
    if weight_without_me != 0 {
        rating_rest = (previous_average_rating * previous_total_weight
            - my_previous_rating_expanded * my_previous_weight)
            / weight_without_me;
    }

    let my_total_weigth = my_previous_weight + my_added_weight;
    let new_total_weight = weight_without_me + my_total_weigth;

    let mut new_average = 0;
    if new_total_weight != 0 {
        new_average = (rating_rest * weight_without_me + my_new_rating_expanded * my_total_weigth)
            / new_total_weight;
    }

    Ok((new_average, new_total_weight))
}
