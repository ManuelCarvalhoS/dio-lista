//! Persistência unificada: mcs_bd2 (nativo) / localStorage (web).
//! Modelo = `lista_comum::Artigo` (igual ao server).

use lista_comum::Artigo;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub use crate::bd::{adicionar, backend_label, guardar_imagem, listar, url_imagem};
}

#[cfg(target_arch = "wasm32")]
mod web {
    use lista_comum::{agora_unix, Artigo, TAM_IMAG, TAM_NOME};
    use std::collections::HashMap;
    use std::sync::Mutex;

    static ARTIGOS: Mutex<Vec<Artigo>> = Mutex::new(Vec::new());
    const KEY_ART: &str = "dio-lista-artigos-v3";
    const KEY_IMG: &str = "dio-lista-imgs-v3";
    const MAX_IMG: usize = 512_000;

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

    fn ler_artigos() -> Vec<Artigo> {
        if let Some(raw) = ls_get(KEY_ART) {
            if let Ok(v) = serde_json::from_str(&raw) {
                return v;
            }
        }
        ARTIGOS.lock().map(|g| g.clone()).unwrap_or_default()
    }

    fn guardar_artigos(list: &[Artigo]) {
        if let Ok(mut g) = ARTIGOS.lock() {
            *g = list.to_vec();
        }
        if let Ok(raw) = serde_json::to_string(list) {
            ls_set(KEY_ART, &raw);
        }
    }

    fn ler_imgs() -> HashMap<String, String> {
        ls_get(KEY_IMG)
            .and_then(|r| serde_json::from_str(&r).ok())
            .unwrap_or_default()
    }

    fn guardar_imgs(map: &HashMap<String, String>) {
        if let Ok(raw) = serde_json::to_string(map) {
            ls_set(KEY_IMG, &raw);
        }
    }

    pub fn backend_label() -> String {
        "localStorage (web)".into()
    }

    pub fn listar() -> Result<Vec<Artigo>, String> {
        Ok(ler_artigos())
    }

    pub fn adicionar(mut artigo: Artigo) -> Result<Artigo, String> {
        let nome = artigo.nome.trim().to_string();
        if nome.is_empty() {
            return Err("Indica o nome do artigo.".into());
        }
        if nome.as_bytes().len() > TAM_NOME {
            return Err(format!("Nome: máximo {TAM_NOME} bytes."));
        }
        artigo.nome = nome;
        let mut list = ler_artigos();
        let n_reg = list.iter().map(|p| p.n_reg).max().unwrap_or(0) + 1;
        artigo.n_reg = n_reg;
        if artigo.imag.trim().is_empty() {
            artigo.imag = lista_comum::nome_ficheiro_imag(n_reg);
        }
        list.push(artigo.clone());
        guardar_artigos(&list);
        Ok(artigo)
    }

    pub fn guardar_imagem(bytes: &[u8]) -> Result<String, String> {
        if bytes.is_empty() {
            return Err("Imagem vazia.".into());
        }
        if bytes.len() > MAX_IMG {
            return Err("Imagem demasiado grande (máx. ~500 KB). Usa um thumbnail.".into());
        }
        let ext = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
            "png"
        } else if bytes.starts_with(b"GIF8") {
            "gif"
        } else if bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
            "webp"
        } else {
            "jpg"
        };
        let ficheiro = format!("i{:08x}.{ext}", agora_unix() as u32);
        if ficheiro.len() > TAM_IMAG {
            return Err("Nome de imagem inválido.".into());
        }
        let mime = match ext {
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "image/jpeg",
        };
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        let data_url = format!("data:{mime};base64,{b64}");
        let mut map = ler_imgs();
        map.insert(ficheiro.clone(), data_url);
        guardar_imgs(&map);
        Ok(ficheiro)
    }

    pub fn url_imagem(imag: &str) -> Option<String> {
        if imag.is_empty() {
            return None;
        }
        ler_imgs().get(imag).cloned()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
pub use web::*;

pub fn listar_ou_vazio() -> Vec<Artigo> {
    listar().unwrap_or_default()
}
