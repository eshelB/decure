# Decure - Decentralized Customer Reviews

A platform for reliable customer reviews and ratings on any business that accepts payment in
SNIP-20 tokens. Powered by Secret Network, privacy is preserved for reviewers.

This example demonstrates how to query different contracts, and how to use CashMap - the only
KV data structure in secret that can be iterated through (other KV data structures cannot, because
keys are not known in advance).

### Why are ratings and reviews here trustworthy?

1) Customers who rate/review must prove they transacted with the business,
by providing a snip-20 receipt.
2) Ratings and reviews are weighted by price of the transaction.

### Disadvantages
1) Receipt Privacy: although Secret Network makes it possible for us to keep the content of the
   receipt private, the volume of the transaction can still be deduced by a replay attack. The
   attacker can know the business' total weight by querying the change done to its average rating
   by his own transaction, which he knows the weight of. Then he can replay the transaction that
   he wants to discover the weight of, and see how did _it_ change the average.
2) Incentive to rate and review: customers have to pay the network fee and gas fees for rating.
   This price could be mitigated by an option to let the business being reviewed refund the fees.
   This could further improve the business' reputation
---
## Interacting with the contract
### Register a Business
A Business must be registered to be able to review it. Anyone can register an address as a business.
(Functionality to edit business or to claim it by the owner is not implemented). Every review on this
business must provide a receipt for a transfer from the reviewer to that business

```bash
  message='{
    "register_business": {
      "name": "Crypto Bicycles",
      "description": "renting bicycles privately",
      "address": "secret1examplebicycles"
    }
  }'
  secretcli tx compute execute <contract-address> "$message" --from <keyname> --gas 150000

  # then query the result of the tx
  secretcli query compute tx <tx_hash>
  # {
  #   ...
  #   "output_data_as_string": {
  #     "register_business": {
  #       "status": "successfully called register business"
  #     }
  #   }
  # }
```

### Reviewing and Rating a Business

After a Business is registered, anyone who transacted with it can review and rate it. Ratings are integers between 0 and 5.
A "receipt" is a transfer that is fetched from the SNIP-20 contract that was used for payment to the business.
This example uses SSCRT as the paying token. The reviewer must provide the `tx_id` of the payment, the `tx_page` where the
tx occurs in the contract (pages are of size 10), and the `viewing_key`, which is never saved in the contract.
The rating provided by this message will have the same weight as the amount of coins in the transfer.

```bash
  message='{
    "review_business": {
      "address": "secret1examplebicycles",
      "content": "excellent service",
      "rating": 5,
      "title": "Best crypto bicycles I have every ridden",
      "tx_id": 8,
      "tx_page": 0,
      "viewing_key": "vk"
    }
  }'
  secretcli tx compute execute <contract-address> "$message" --from <keyname> --gas 150000
```

The result of this transaction may be:

```bash
#  ...
#  "output_data_as_string": {
#      "review_business": {
#          "status": "Successfully added a new review on business, receipt was accounted for"
#      }
#  }
#  ...
```

To **Edit** a  review, simply provide the same `tx_id` (note that pagination in SSCRT is from newest to oldest so the page
number might change), with the new content/rating. The result of the transaction will then be:
```bash
 "status": "Successfully updated a previous review on business"
```

You can provide more receipts to enlarge the weight of the review, every tx is accounted for. Note that there is only
one review and rating that a single account may have on each business. Previous transactions are considered to have given
the last rating that was given by the account. <br>
For example, these two txs: <br>
`tx1(weight=1, rating=4)`, and then <br>
`tx2(weight=2, rating=0)` <br>
are the same as <br>
`tx3(weight=3, rating=0)`
```bash
 "status": "Successfully updated a previous review on business, receipt was accounted for"
```

### Querying
All Queries that return an array accept a `page_size` and an optional `page` for pagination purposes. <br>
You can query all businesses:
```bash
  message='{
    "get_businesses": {
      "page_size": 8
    }
  }'

  secretcli q compute query <contract-address> "$message"
  #
  #
  #
  #
  #
```
