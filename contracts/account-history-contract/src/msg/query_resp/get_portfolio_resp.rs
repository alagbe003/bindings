use crate::types::Portfolio;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::SignedDecimal256;

#[cw_serde]
pub struct GetPortfolioResp {
    pub portfolio: Portfolio,
    pub actual_portfolio_balance: SignedDecimal256,
    pub old_portfolio_balance: SignedDecimal256,
    pub balance_24h_change: SignedDecimal256,
}
