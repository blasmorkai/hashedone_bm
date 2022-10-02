#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{DonatorsResponse, ExecuteMsg, InstantiateMsg, ManagerResp, OwnerResp, QueryMsg};
use crate::state::{OWNER, State, STATE};

/*
const CONTRACT_NAME: &str = "crates.io:donation-peer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
 */

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.save(deps.storage,&owner)?;

    // The manager is the donation-manager contract address
    let state =  State {
        donators: 0,
        incremental_donation: msg.incremental_donation,
        collective_ratio: msg.collective_ratio,
        manager: info.sender,
    };
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute("action","peer_instantiated"))

}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Donators {} => query_donators(deps),
        QueryMsg::Owner {} => query_owner(deps),
        QueryMsg::Manager {} => query_manager(deps),
    }

}

fn query_donators (deps: Deps ) -> StdResult<Binary> {
    let state = STATE.load(deps.storage)?;
    Ok(to_binary(&DonatorsResponse{ donators: state.donators })?)
}

fn query_owner (deps: Deps) -> StdResult<Binary> {
    let owner = OWNER.load(deps.storage)?;
    let resp = to_binary(&OwnerResp{owner})?;
    Ok(resp)
}

fn query_manager (deps: Deps) -> StdResult<Binary> {
    let state = STATE.load(deps.storage)?;
    let resp = to_binary(&ManagerResp{ manager: state.manager })?;
    Ok(resp)
}

#[cfg(test)]
mod tests {}
