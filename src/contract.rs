use std::collections::HashSet;

use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult, to_json_binary};
use crate::error::ContractError;
use crate::messages::{QueryMsg, GreetResp, InstantiateMsg, AdminsListResp, ExecuteMsg};
use crate::state::{ADMINS, DONATION_DENOM};

pub fn execute(
    deps: DepsMut,
    _env :Env,
    info:MessageInfo,
    msg: ExecuteMsg ) -> Result<Response, ContractError> {
        use ExecuteMsg::*;
        match msg {
            AddMembers {admins} => exec::add_members(deps, info, admins),
            Leave {} => exec::leave(deps, info).map_err(Into::into),
            Donate {  } => exec::denom(deps, info)
        }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg{
        Greet {} => to_json_binary(&query::greet()?),
        AdminsList {} => to_json_binary(&query::admin_list(deps)?)
    }
}
pub fn instantiate(deps: DepsMut, _env: Env, _info: MessageInfo, msg: InstantiateMsg) -> StdResult<Response> {
    let admins: StdResult<HashSet<_>> = msg.admins.into_iter().map(|addr| deps.api.addr_validate(&addr)).collect();
    ADMINS.save(deps.storage, &admins?)?;
    DONATION_DENOM.save(deps.storage, &msg.donation_denom)?;
    Ok(Response::new())
}

// Modules
mod exec {
    use std::collections::HashSet;

    use cosmwasm_std::{Event, BankMsg, coins};
    use cw_utils::*;
    use crate::error::ContractError;

    use super::*;
    

    
    pub fn denom(deps: DepsMut, info : MessageInfo) -> Result<Response, ContractError> {
        let admins = ADMINS.load(deps.storage)?;
        let denom = DONATION_DENOM.load(deps.storage)?;

        let donation = must_pay(&info, &denom)?.u128();
        let donation_per_admin = donation / (admins.len() as u128);
        let res = donation % (admins.len() as u128);

        //FIXME: handle case of 2 admins sharing 51 tokens
        let mut messages : Vec<_> = admins.into_iter().map(|admin| BankMsg::Send 
            {   to_address: admin.into_string(), 
                amount: coins(donation_per_admin, &denom)
            }).collect();
        messages.push(BankMsg::Send { to_address: info.sender.into_string(), amount: coins(res, &denom) });
        
        

        // let  messages  = admins.into_iter().map(|admin| BankMsg::Send 
        //     {   to_address: admin.into_string(), 
        //         amount: coins(donation_per_admin, &denom)
        //     });
        
        let resp = Response::new()
            .add_messages(messages.into_iter())
            .add_attribute("action", "donate")
            .add_attribute("amount", donation.to_string())
            .add_attribute("per_admin", donation_per_admin.to_string());
                
        Ok(resp)
    }

    pub fn add_members(deps: DepsMut, info: MessageInfo, admins: Vec<String>) -> Result<Response, ContractError>{
        let mut current_admins = ADMINS.load(deps.storage)?;
        if !current_admins.contains(&info.sender){
            return Err(ContractError::Unauthorized{sender: info.sender})
        }
        let events = admins.iter().map(|admin| Event::new("admin_added").add_attribute("addr", admin));
        
        let resp = Response::new().add_events(events).add_attribute("action", "add_members").add_attribute("added_count", admins.len().to_string());
        
        let admins : StdResult<HashSet<_>>= admins.into_iter().map(|admin| deps.api.addr_validate(&admin)).collect();
        let admins = admins.unwrap();
        current_admins.extend(&mut admins.iter().cloned());
        ADMINS.save(deps.storage, &current_admins)?;

        Ok(resp)
    }
    pub fn leave(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
        ADMINS.update(deps.storage, move |admins| -> StdResult<_>{
            let admins = admins.into_iter().filter(|admin| *admin != info.sender).collect();
            Ok(admins)
        })?;
        Ok(Response::new())
    }
}

mod query{
    use super::*;
    pub fn greet() -> StdResult<GreetResp> {
        let resp = GreetResp{
            message : "Hello world!".to_owned(),
        };
        Ok(resp)
    }
    pub fn admin_list(deps: Deps)  -> StdResult<AdminsListResp>{
        let admins = ADMINS.load(deps.storage)?;
        Ok(AdminsListResp { admins: admins.into_iter().collect() })
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

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
    fn greet_query1(){
        let mut deps = mock_dependencies();
        let env = mock_env();
        
        instantiate(
            deps.as_mut(), 
            env.clone(),
            mock_info("sender", &[]),
            InstantiateMsg { admins: vec![],  donation_denom: "eth".to_string() }).unwrap();
        
        let resp = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Greet { }).unwrap();
        let resp: GreetResp = from_binary(&resp).unwrap();

        assert_eq!(resp, GreetResp{message: "Hello world!".to_owned()})
    }

    #[test]
    fn greet_query2(){
        let mut app = App::default();
        let code = ContractWrapper::new(execute,instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr: Addr = app.instantiate_contract(
            code_id, 
            Addr::unchecked("owner"), 
            &InstantiateMsg { admins : vec![], donation_denom: "eth".to_string()}, 
            &[], 
            "Contract", 
            None,)
            .unwrap();

        let resp : GreetResp = app.wrap().query_wasm_smart(addr, &QueryMsg::Greet {  }).unwrap();
        
        assert_eq!(resp, GreetResp{message: "Hello world!".to_owned()})
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
        

        let addr = app.instantiate_contract(
            code_id,
             Addr::unchecked("owner"),
              &InstantiateMsg{admins : vec!["admin1".to_owned(), "admin2".to_owned(), "admin3".to_owned()],  donation_denom: "eth".to_string()}, 
              &[],
               "Contract 1",
                None).unwrap();
        let resp : AdminsListResp = app.wrap().query_wasm_smart(addr, &QueryMsg::AdminsList {  }).unwrap();
        let resp : HashSet<Addr> = resp.admins.into_iter().collect();

        assert_eq!(resp, vec![Addr::unchecked("admin2"), Addr::unchecked("admin1"),Addr::unchecked("admin3")].into_iter().collect())

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
