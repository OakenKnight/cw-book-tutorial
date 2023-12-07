use cosmwasm_std::Addr;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GreetResp{
    pub message : String,
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct AdminListResp{
    pub admins : Vec<Addr>
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct InstantiateMsg{
    pub admins : Vec<String>,
    pub donation_denom: String,

}




#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum QueryMsg {
    Greet {},
    AdminList {},
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ExecuteMsg{
    AddMembers { admins: Vec<String> },
    Leave {},
    Donate {},
}