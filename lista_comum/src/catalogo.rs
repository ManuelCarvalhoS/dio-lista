//! Ids de imagem.

use crate::artigo::{agora_unix, TAM_IMAG};

pub fn novo_imag_id() -> String {
    let t = agora_unix();
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    std::thread::current().id().hash(&mut h);
    let r = h.finish() as u32;
    let s = format!("{t:08x}{r:08x}");
    s.chars().take(TAM_IMAG).collect()
}

/// Nome de ficheiro de ícone do catálogo: `imag{n_reg}.png` (cabe no `imag[16]`).
pub fn nome_ficheiro_imag(n_reg: u64) -> String {
    let s = format!("imag{n_reg}.png");
    debug_assert!(s.len() <= TAM_IMAG);
    s
}
