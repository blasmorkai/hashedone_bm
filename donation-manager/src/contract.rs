#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Order, Reply, Response, StdResult, SubMsg, SubMsgResult, to_binary, Uint128, WasmMsg};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetConfigResponse, InstantiateMsg, MemberPeerAddrResp, QueryMsg};
use crate::state::{Config, CONFIG, MEMBERS, PENDING_INSTANTIATION};

/*
const CONTRACT_NAME: &str = "crates.io:donation-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
 */

// Used to identify the Response-Submessages
pub const PEER_INSTANTIATE_ID: u64 = 1;

// ////////////////////////////////////////INSTANTIATE///////////////////////////////////////////////
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    let config = Config{
        peer_code_id: msg.peer_code_id,
        incremental_donation: msg.incremental_donation,
        collective_ratio: msg.collective_ratio
    };

    CONFIG.save(deps.storage,&config)?;
    Ok(Response::new().add_attribute("action","manager-instantiated"))

}

// ////////////////////////////////////////EXECUTE//////////////////////////////////////////////////
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Join { .. } => {join(deps, info)},
        ExecuteMsg::Leave { .. } => {Ok(Response::new())},
        ExecuteMsg::Donate { .. } => {donate(deps, env, info)},
    }


}

fn join (deps:DepsMut, info:MessageInfo) -> Result<Response, ContractError>{

    let creator = info.sender.to_string();
    let config = CONFIG.load(deps.storage)?;

    // Step 1: Create instantiate message from called contract
    let msg = donation_peer::msg::InstantiateMsg {
        owner: creator.clone(),
        incremental_donation: config.incremental_donation,
        collective_ratio: config.collective_ratio
    };

    // Step 2: Create a WasmMsg of type instantiate
    let msg = WasmMsg::Instantiate {
        admin: None,
        code_id: config.peer_code_id,
        msg: to_binary(&msg)?,
        funds: vec![],                  // Also Vec::new()
        label: format!("{}-peer",creator),
    };

    // Step 3: Record the address of the caller/creator of this process
    PENDING_INSTANTIATION.save(deps.storage,&info.sender)?;

    // Step 4: Create a response with a submessage attaching the message with reply_on_success
    let resp = Response::new()
        .add_submessage(SubMsg::reply_on_success(msg,PEER_INSTANTIATE_ID))
        .add_attribute("action","join")
        .add_attribute("creator",info.sender.to_string());
    Ok(resp)
}

fn donate (deps: DepsMut, env: Env ,info: MessageInfo) -> Result<Response, ContractError> {

    // For each pair of peer/owner on MEMBERS, query the peer contract and get the donators number
    let weights: Vec<_> = MEMBERS
        .keys(deps.storage,None,None,Order::Ascending)
        .map(|peer| -> StdResult<_> {
            let peer = peer?;
            let donators = donation_peer::state::STATE.query(&deps.querier,peer.clone())?.donators;
            Ok((peer,donators as u128))
        })
        .collect::<StdResult<_>>()?;   // The tuple provided in the previous step will join a collection (we can iter() and map() it later)

    //From here weights has got a collection of tuples (peer_address, donator as u128)

    //Add all the donators stored on the weights collection
    let total_donators : u128 = weights.iter().map(|(_,weight)| weight).sum();

    // Query the blockchain to obtain this contract's balance on all coins
    let funds = deps.querier.query_all_balances(env.contract.address)?;

    // Double loop: for each Peer/Owner -> Peer/weight. Get a list of its share of all coins on the contract
    //and create an Option which content is a BankMsg::Send with the peer address and his coins
    let send_msgs = weights.into_iter().filter_map(|(peer, weights)| {
        //How many coins owned by the contract belong to the peer in analysis
        let coins: Vec<_>= funds
            .iter()
            .cloned()
            .map (|mut coin| {
                coin.amount = Uint128::new(coin.amount.u128() * weights / total_donators);
                coin
            })
            .collect();

        //If there are coins for this peer contract, not zero.....
        if coins.iter().all(|c| c.amount==Uint128::zero()) {
            None
        } else {
            Some(BankMsg::Send {
                to_address: peer.to_string(),
                amount:coins,
            })
            // There is a better way of doing this, sending all in one message or through another contract. See video.
        }
    });

    let resp = Response::new()
        .add_messages(send_msgs)
        .add_attribute("action", "donate")
        .add_attribute("sender",info.sender.to_string());

    Ok(resp)
}


// ////////////////////////////////////////QUERY////////////////////////////////////////////////////
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config { .. } => query_config(deps),
        QueryMsg::MemberPeerAddr {addr} => to_binary(&query_member_peer_addr(deps, &addr)?)
    }
}


pub fn query_config(deps: Deps) -> StdResult<Binary> {
// when using may_load the response of the function seems to need-be an Option
    let config = CONFIG.may_load(deps.storage)?;
    let resp = to_binary(&GetConfigResponse{ config })?;
    Ok(resp)
}

pub fn query_member_peer_addr(deps: Deps, addr: &str) -> StdResult<MemberPeerAddrResp> {
    // Find all the peers whose owner is the addr parameter.
    // We search in the MEMBERS storage unit. We do not ask outside this contract.
    let peer = MEMBERS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|addr| addr.ok())           //The returned iterator yields only the values for which the supplied closure returns Some(value).
        .find(|(_, owner)| owner.as_str() == addr);

    // let (peer, _) = peer.ok_or_else(|| StdError::generic_err("No such member"))?;

    // NEXT LINE DOES NOT WORK with ? because ContractErrror::CustomError does not implement follow something.
    // ok_or_else Transforms the Option<T> into a Result<T, E>, mapping Some(v) to Ok(v) and None to Err(err()).
    let (peer, _) = peer.ok_or_else(|| ContractError::CustomError { val: "No such member".to_string() }).unwrap();

    Ok(MemberPeerAddrResp{ addr: peer })
}

// ////////////////////////////////////////REPLY////////////////////////////////////////////////////
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response,ContractError> {
    match msg.id {
        PEER_INSTANTIATE_ID => {peer_instantiate_reply(deps, msg.result)},
        _ => Err(ContractError::CustomError {val:"unknown reply id".to_string()})
    }
}

fn peer_instantiate_reply (deps: DepsMut, msg: SubMsgResult) -> Result<Response,ContractError> {
    //Objetive: Access the address of the newly created contract to save it @ Members
    // Three steps: check submsg response, get its data, parse its data.
    //Then we can access the resp.contract_address that is the address of the newly generated contract. We can update MEMBERS then

    // 1.- Make sure you have got the response Ok and not an Err. Next line does not work because msg.into_result does not produce the right error.
    //let resp = msg.into_result()?;

    // Alternative to ? when getting the result of a function. Here for some reason on the workshop we can not use ?
    let resp = match msg.into_result() {
        Ok(resp ) => resp,
        Err(err) => {return Err(ContractError::CustomError {val: err.to_string()})}
    };

     // 2.- Get the data, make sure it is there. In instatiation the data is a Option<Binary>. events can be analized as well.
    // ok_or_else does nothing if it is ok, but if else, it executes the attached code
    let data = resp.data
        .ok_or_else(|| ContractError::CustomError {val:"No instantiate response data".to_string()})?;

    // 3.- Parse the data from the Option<Binary>
    let resp = cw_utils::parse_instantiate_response_data(&data)
        .map_err(|error| ContractError::CustomError {val:error.to_string()})?;

    let creator = PENDING_INSTANTIATION.load(deps.storage)?;
    //Newly created peer address
    let peer = Addr::unchecked(resp.contract_address);

    MEMBERS.save(deps.storage, peer.clone(), &creator.clone())?;

    let resp = Response::new()
        .add_attribute("action","joined")
        .add_attribute("owner", creator)
        .add_attribute("peer",peer);

    Ok(resp)
}

// ////////////////////////////////////////TESTS/////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, coin, Decimal, Empty};
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};
    use donation_peer::msg::{DonatorsResponse, ManagerResp, OwnerResp};
    use crate::contract::{execute, instantiate, query, reply};
    use crate::msg::{ExecuteMsg, GetConfigResponse, InstantiateMsg, MemberPeerAddrResp, QueryMsg};
    use crate::state::Config;

    fn peer() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(donation_peer::contract::execute, donation_peer::contract::instantiate, donation_peer::contract::query);
        Box::new(contract)
    }

    fn manager() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);
        Box::new(contract)
    }

    #[test]
    fn instantiate_check() {
        let mut app = App::default();

        let peer_code_id = app.store_code(peer());
        let manager_code_id = app.store_code(manager());

        let manager_addr = app
            .instantiate_contract(
                manager_code_id,
                Addr::unchecked("manager_creator_address"),
                &InstantiateMsg{
                    peer_code_id,
                    incremental_donation: coin(100, "utdg"),
                    collective_ratio: Decimal::percent(60),
                },
            &[],
            "manager",
            None)
            .unwrap();

        let config : GetConfigResponse= app
            .wrap()
            .query_wasm_smart(manager_addr.clone(), &QueryMsg::Config {})
            .unwrap();

        assert_eq!(config, GetConfigResponse{ config : Some(Config{
            peer_code_id,
            incremental_donation: coin(100,"utdg"),
            collective_ratio: Decimal::percent(60),
        }) } )
    }

    #[test]
    fn join_check() {
        let mut app = App::default();
        let peer_code_id = app.store_code(peer());
        let manager_code_id = app.store_code(manager());

        let manager_address = app
            .instantiate_contract(
                manager_code_id,
                Addr::unchecked("manager_creator_address"),
                &InstantiateMsg{
                    peer_code_id,
                    incremental_donation: coin(100, "utdg"),
                    collective_ratio: Decimal::percent(60),
                },
                &[],
                "manager",
                None)
            .unwrap();

        // Manager - ExecuteMsg::Join {} from "creator_address"
        app.execute_contract(Addr::unchecked("creator_address"), manager_address.clone(), &ExecuteMsg::Join {}, &[]).unwrap();

        // Manager - QueryMsg::MemberPeerAddr { addr: "creator_address".to_string() }
        let peer : MemberPeerAddrResp = app
            .wrap()
            .query_wasm_smart(manager_address.clone(), &QueryMsg::MemberPeerAddr { addr: "creator_address".to_string() })
            .unwrap();

        // Peer - QueryMsg::Owner . The peer address is obtained in the previous step - MemberPeerAddrResp.addr
        let owner_resp : OwnerResp = app
            .wrap()
            .query_wasm_smart(peer.addr.clone(), &donation_peer::msg::QueryMsg::Owner {})
            .unwrap();
        assert_eq!(Addr::unchecked("creator_address"),owner_resp.owner);

        //Peer - QueryMsg::Manager . It should be the contract that created the peer i.e. the manager
        let manager_resp : ManagerResp = app
            .wrap()
            .query_wasm_smart(peer.addr.clone(),&donation_peer::msg::QueryMsg::Manager {})
            .unwrap();
        assert_eq!(manager_address, manager_resp.manager);

        //Peer - QueryMsg::Donators. It should be zero
        let donators_resp : DonatorsResponse = app
            .wrap()
            .query_wasm_smart(peer.addr,&donation_peer::msg::QueryMsg::Donators {})
            .unwrap();
        assert_eq!(donators_resp.donators,0);
    }


    #[test]
    fn single_peer_single_donate() {
        // // After a single donation, the only peer should got the whole amount
        //
        // let mut app = App::new(|router, _api, storage| {
        //     router
        //         .bank
        //         .init_balance(storage, &Addr::unchecked("donator"), coins(100, "utgd"))
        //         .unwrap();
        // });
        // let peer_code_id = app.store_code(peer());
        // let manager_code_id = app.store_code(manager());
        //
        // let manager = app
        //     .instantiate_contract(
        //         manager_code_id,
        //         Addr::unchecked("admin"),
        //         &InstantiateMsg {
        //             peer_code_id,
        //             incremental_donation: coin(100, "utgd"),
        //             collective_ratio: Decimal::percent(60),
        //         },
        //         &[],
        //         "manager",
        //         None,
        //     )
        //     .unwrap();
        //
        // app.execute_contract(
        //     Addr::unchecked("member"),
        //     manager.clone(),
        //     &ExecMsg::Join {},
        //     &[],
        // )
        // .unwrap();
        //
        // let peer: MemberPeerAddrResp = app
        //     .wrap()
        //     .query_wasm_smart(
        //         manager,
        //         &QueryMsg::MemberPeerAddr {
        //             addr: "member".to_owned(),
        //         },
        //     )
        //     .unwrap();
        //
        // app.execute_contract(
        //     Addr::unchecked("donator"),
        //     peer.addr.clone(),
        //     &PeerExec::Donate {},
        //     &coins(100, "utgd"),
        // )
        // .unwrap();
        //
        // app.execute_contract(
        //     Addr::unchecked("member"),
        //     peer.addr.clone(),
        //     &PeerExec::Withdraw {},
        //     &[],
        // )
        // .unwrap();
        //
        // assert_eq!(
        //     coin(0, "utgd"),
        //     app.wrap().query_balance("donator", "utgd").unwrap()
        // );
        // assert_eq!(
        //     coin(0, "utgd"),
        //     app.wrap()
        //         .query_balance(peer.addr.as_str(), "utgd")
        //         .unwrap()
        // );
        // assert_eq!(
        //     coin(100, "utgd"),
        //     app.wrap().query_balance("member", "utgd").unwrap()
        // );
    }
    
}
