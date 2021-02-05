# Module Crowdfunding

Simple Crowdfunding module, supporting multiple campaigns, which are all settled with the platform currency.

This pallet provides a simple on-chain crowdfunding mechanism:
- creator can create a campaign with individual length and amount of funds in PLAY to raise
- investor can invest his funds into one of the running campaigns and become an investor

Upon finalization:
- creator can request allocation of funds
- investors can collectively approve allocation of funds

TODO:
- supervisor can lock, cancel campaigns
...

1. create campaigns with custom funding goal and runtime
2. invest into open campaigns

3. request withdrawal (unreserve) as creator from successful campaign
4. approve withdrawals (unreserve) as investor from successfully funded campaigns
