use std::collections::HashSet;

use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult, to_json_binary};
use crate::error::ContractError;
use crate::messages::{QueryMsg, InstantiateMsg, AdminsListResp, ExecuteMsg};
use crate::state::{ADMINS, DONATION_DENOM};


pub fn instantiate(deps: DepsMut, env: Env, _info: MessageInfo, msg: InstantiateMsg) -> StdResult<Response> {

    for admin in msg.admins{ 
        let admin = deps.api.addr_validate(&admin)?;
        ADMINS.save(deps.storage, &admin, &env.block.time)?;
    }
    
    DONATION_DENOM.save(deps.storage, &msg.donation_denom)?;
    Ok(Response::new())
}
pub fn leave(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    ADMINS.remove(deps.storage, &info.sender);
    let res = Response::new()
        .add_attribute("action", "leave")
        .add_attribute("sender", info.sender);
    Ok(res)
}
pub fn execute(
    deps: DepsMut,
    env :Env,
    info:MessageInfo,
    msg: ExecuteMsg ) -> Result<Response, ContractError> {
        use ExecuteMsg::*;
        match msg {
            AddMembers {admins} => exec::add_members(deps, env, info, admins),
            Leave {} => exec::leave(deps, info).map_err(Into::into),
            Donate {  } => exec::donate(deps, info)
        }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg{
        AdminsList {} => to_json_binary(&query::admin_list(deps)?),
        JoinTime { admin } => to_json_binary(&query::join_time(deps, admin)?),
    }
}
// Modules
mod exec {
    use std::collections::HashSet;

    use cosmwasm_std::{Event, BankMsg, coins};
    use cw_storage_plus::Map;
    use cw_utils::*;
    use crate::error::ContractError;

    use super::*;
    

    
    pub fn donate(deps: DepsMut, info : MessageInfo) -> Result<Response, ContractError> {
        let denom = DONATION_DENOM.load(deps.storage)?;

        let admins : Result<Vec<_>, _>= ADMINS.keys(deps.storage, None, None, cosmwasm_std::Order::Ascending).collect();
        let admins = admins?;
        let donation = must_pay(&info, &denom)?.u128();
        let donation_per_admin = donation / (admins.len() as u128);
        let res = donation % (admins.len() as u128);

        //FIXME: handle case of 2 admins sharing 51 tokens
        let mut messages : Vec<_> = admins.into_iter().map(|admin| BankMsg::Send 
            {   to_address: admin.into_string(), 
                amount: coins(donation_per_admin, &denom)
            }).collect();
        messages.push(BankMsg::Send { to_address: info.sender.into_string(), amount: coins(res, &denom) });
        
        let resp = Response::new()
            .add_messages(messages.into_iter())
            .add_attribute("action", "donate")
            .add_attribute("amount", donation.to_string())
            .add_attribute("per_admin", donation_per_admin.to_string());
                
        Ok(resp)
    }

    pub fn add_members(deps: DepsMut, env : Env, info: MessageInfo, admins: Vec<String>) -> Result<Response, ContractError>{
        if !ADMINS.has(deps.storage, &info.sender){
            return Err(ContractError::Unauthorized{sender: info.sender})
        }

        let events = admins.iter().map(|admin| Event::new("admin_added").add_attribute("addr", admin));
        
        let resp = Response::new().add_events(events)
            .add_attribute("action", "add_members")
            .add_attribute("added_count", admins.len().to_string());
        
        for admin in admins {
            let admin = deps.api.addr_validate(&admin)?;
            ADMINS.save(deps.storage, &admin, &env.block.time)?;
        }

        Ok(resp)
    }
    pub fn leave(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
        let a = ADMINS.remove(deps.storage, &info.sender);
        Ok(Response::new())
    }
}

mod query{
    use cosmwasm_std::Addr;

    use crate::messages::JoinTimeResp;

    use super::*;
    pub fn admin_list(deps: Deps)  -> StdResult<AdminsListResp>{
        let admins: Result<_, _> =  ADMINS.keys(deps.storage, None, None, cosmwasm_std::Order::Ascending).collect();
        Ok(AdminsListResp { admins:admins?})
    }
    pub fn join_time(deps: Deps, admin : String) -> StdResult<JoinTimeResp> {
        ADMINS
            .load(deps.storage, &Addr::unchecked(admin))
            .map(|joined| JoinTimeResp{joined})
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use crate::messages::JoinTimeResp;

    use super::*;
    use cosmwasm_std::{from_binary, Addr, coins};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cw_multi_test::{App, ContractWrapper, Executor};
    

    #[test]
    fn donations_exploit(){
        let mut app = App::new(|router, _, storage| {
            router.bank.init_balance(storage, &Addr::unchecked("user"), coins(55, "eth")).unwrap()
        });

        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));


        let addr = app.instantiate_contract(
            code_id, 
            Addr::unchecked("owner"), 
            &InstantiateMsg
            { 
                admins : vec!["admin1".to_owned(), "admin2".to_owned()], 
                donation_denom: "eth".to_string()
            },  
            &[], 
            "Contract", 
            None)
            .unwrap();

        app.execute_contract(
            Addr::unchecked("admin1"),
            addr.clone(),
            &ExecuteMsg::AddMembers { admins: vec!["admin1".to_owned()] },
            &[])
            .unwrap();

        assert_eq!(app.wrap().query_balance("user", "eth").unwrap().amount.u128(), 55);

        app.execute_contract(Addr::unchecked("user"), addr.clone(), &ExecuteMsg::Donate {  }, &coins(55, "eth".to_string())).unwrap(); 
        
        assert_eq!(app
            .wrap()
            .query_balance("user", "eth")
            .unwrap()
            .amount
            .u128(), 1);

        assert_eq!(app
            .wrap()
            .query_balance(&addr, "eth")
            .unwrap()
            .amount
            .u128(), 0);

        assert_eq!(app
            .wrap()
            .query_balance("admin2", "eth")
            .unwrap()
            .amount
            .u128(), 27);

        assert_eq!(app
            .wrap()
            .query_balance("admin1", "eth")
            .unwrap()
            .amount
            .u128(), 27);


    }
    #[test]
    fn donations(){
        let mut app = App::new(|router, _, storage| {
            router.bank.init_balance(storage, &Addr::unchecked("user"), coins(51, "eth")).unwrap()
        });

        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));


        let addr = app.instantiate_contract(
            code_id, 
            Addr::unchecked("owner"), 
            &InstantiateMsg
            { 
                admins : vec!["admin1".to_owned(), "admin2".to_owned()], 
                donation_denom: "eth".to_string()
            },  
            &[], 
            "Contract", 
            None)
            .unwrap();

        assert_eq!(app.wrap().query_balance("user", "eth").unwrap().amount.u128(), 51);

        app.execute_contract(Addr::unchecked("user"), addr.clone(), &ExecuteMsg::Donate {  }, &coins(51, "eth".to_string())).unwrap(); 
        
        assert_eq!(app
            .wrap()
            .query_balance("user", "eth")
            .unwrap()
            .amount
            .u128(), 1);

        assert_eq!(app
            .wrap()
            .query_balance(&addr, "eth")
            .unwrap()
            .amount
            .u128(), 0);

        assert_eq!(app
            .wrap()
            .query_balance("admin2", "eth")
            .unwrap()
            .amount
            .u128(), 25);

        assert_eq!(app
            .wrap()
            .query_balance("admin1", "eth")
            .unwrap()
            .amount
            .u128(), 25);


    }
    #[test]
    fn add_members(){
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr: Addr = app.instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg{admins : vec!["owner".to_owned()],  donation_denom: "eth".to_string()},
            &[],
            "Contract",
            None).
            unwrap();
        let response = app.execute_contract(
            Addr::unchecked("owner"),
            addr,
            &ExecuteMsg::AddMembers { admins: vec!["user".to_owned()] },
            &[])
            .unwrap();

        let wasm_event = response.events.iter().find(|ev| ev.ty == "wasm").unwrap();
        
        assert_eq!(
            wasm_event.attributes.iter().find(|e| e.key=="action").unwrap().value, "add_members"
        );
        assert_eq!(
            wasm_event.attributes.iter().find(|e| e.key=="added_count").unwrap().value, "1"
        );
        let admin_events : Vec<_> = response.events.iter().filter(|ev| ev.ty == "wasm-admin_added").collect();
        assert_eq!(admin_events[0].attributes.iter().find(|event| event.key == "addr").unwrap().value, "user")

    }
    #[test]
    fn instantiation(){

        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));


        let addr = app.instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg{admins: vec![],  donation_denom: "eth".to_string()}, 
            &[], "Contract 1", 
            None)
            .unwrap();
        let resp : AdminsListResp = app
                                    .wrap()
                                    .query_wasm_smart(addr, &QueryMsg::AdminsList {  })
                                    .unwrap();

        assert_eq!(resp, AdminsListResp{admins: vec![]});
        
        let block = app.block_info();

        let addr = app.instantiate_contract(
            code_id,
             Addr::unchecked("owner"),
              &InstantiateMsg{admins : vec!["admin1".to_owned(), "admin2".to_owned(), "admin3".to_owned()],  donation_denom: "eth".to_string()}, 
              &[],
               "Contract 1",
                None).unwrap();

        let resp : AdminsListResp = app.wrap()
            .query_wasm_smart(addr.clone(), &QueryMsg::AdminsList {})
            .unwrap();

        assert_eq!(
            resp,
            AdminsListResp {
                admins: vec![Addr::unchecked("admin1"), Addr::unchecked("admin2"), Addr::unchecked("admin3")],
            }
        );

        let resp_joined_time : JoinTimeResp = app.wrap()
            .query_wasm_smart(addr, &QueryMsg::JoinTime { admin: "admin1".to_owned() })
            .unwrap();
        assert_eq!(block.time, resp_joined_time.joined)

    }
    #[test]
    fn unauthorized(){
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr = app.instantiate_contract(code_id, Addr::unchecked("owner"), &InstantiateMsg{admins : vec![],  donation_denom: "eth".to_string()}, &[], "Contract", None).unwrap();

        let err = app.execute_contract(Addr::unchecked("user"), addr, &ExecuteMsg::AddMembers{admins: vec!["user".to_owned()]}, &[]).unwrap_err();

        assert_eq!(ContractError::Unauthorized { sender: Addr::unchecked("user") }, err.downcast().unwrap())
    }

    #[test]
    fn execute_ok(){
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr: Addr = app.instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg{admins: vec!["admin1".to_owned()],  donation_denom: "eth".to_string()},
            &[],
            "Contract",
            None).
            unwrap();
        app.execute_contract(
            Addr::unchecked("admin1"),
            addr.clone(),
            &ExecuteMsg::AddMembers { admins: vec!["admin2".to_owned()] },
            &[])
            .unwrap();
        let query_resp : AdminsListResp = app.wrap().query_wasm_smart(addr, &QueryMsg::AdminsList {  }).unwrap();
        let query_resp : HashSet<Addr> = query_resp.admins.into_iter().collect();

        assert_eq!(query_resp, vec![Addr::unchecked("admin1"), Addr::unchecked("admin2")].into_iter().collect());
    }


}
