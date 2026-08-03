use mcs_bd2::estrutura::EntityFiles;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub type BdMapa = HashMap<String, Arc<Mutex<EntityFiles>>>;

#[derive(Clone)]
pub struct AppState {
    pub bd: Arc<BdMapa>,
    pub sso_secret: String,
    pub jwt_secret: String,
    pub labnetcol_url: String,
    /// API LabNetCol (login / SSO / Google).
    pub labnetcol_api: String,
    /// Login de teste sem LabNetCol (só desenvolvimento).
    pub dev_login: bool,
    /// n_reg LabNetCol com acesso ao catálogo base (além de tipo=1).
    pub admin_ids: HashSet<u64>,
}

impl AppState {
    pub fn e_admin(&self, labnetcol_id: u64, tipo: u8) -> bool {
        tipo == 1 || self.admin_ids.contains(&labnetcol_id)
    }
}
