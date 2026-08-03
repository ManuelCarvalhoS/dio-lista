use lista_comum::SessaoLista;
use serde::{Deserialize, Serialize};

const KEY_SESSAO: &str = "lista_sessao";
const KEY_API: &str = "lista_api_url";
const KEY_LOJA: &str = "lista_loja_ida";

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub api_url: String,
}

pub fn ler_sessao() -> Option<SessaoLista> {
    let raw = ls_get(KEY_SESSAO)?;
    serde_json::from_str(&raw).ok()
}

pub fn guardar_sessao(s: &SessaoLista) {
    if let Ok(raw) = serde_json::to_string(s) {
        ls_set(KEY_SESSAO, &raw);
    }
}

pub fn limpar_sessao() {
    if let Some(w) = web_sys::window() {
        if let Ok(Some(s)) = w.local_storage() {
            let _ = s.remove_item(KEY_SESSAO);
        }
    }
}

pub fn ler_config() -> Config {
    ls_get(KEY_API)
        .and_then(|r| serde_json::from_str(&r).ok())
        .unwrap_or_default()
}

pub fn guardar_config(c: &Config) {
    if let Ok(raw) = serde_json::to_string(c) {
        ls_set(KEY_API, &raw);
    }
}

fn ls_get(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(key)
        .ok()?
}

fn ls_set(key: &str, val: &str) {
    if let Some(w) = web_sys::window() {
        if let Ok(Some(s)) = w.local_storage() {
            let _ = s.set_item(key, val);
        }
    }
}

pub fn ler_loja_ida() -> Option<u64> {
    ls_get(KEY_LOJA)?.parse().ok().filter(|n| *n > 0)
}

pub fn guardar_loja_ida(id: Option<u64>) {
    match id {
        Some(n) if n > 0 => ls_set(KEY_LOJA, &n.to_string()),
        _ => {
            if let Some(w) = web_sys::window() {
                if let Ok(Some(s)) = w.local_storage() {
                    let _ = s.remove_item(KEY_LOJA);
                }
            }
        }
    }
}
