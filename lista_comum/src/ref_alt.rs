//! Registo de alteração do preço de referência (job diário).

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::artigo::agora_unix;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefAlteracao {
    pub n_reg: u64,
    pub artigo_id: u64,
    pub preco_antes: u32,
    pub preco_depois: u32,
    /// Até 3 lojas onde esse preço unitário apareceu recentemente (0 = vazio).
    pub loja_ids: [u64; 3],
    pub alterado_em: u64,
}

impl RefAlteracao {
    pub fn nova(artigo_id: u64, preco_antes: u32, preco_depois: u32, lojas: &[u64]) -> Self {
        let mut loja_ids = [0u64; 3];
        for (i, &id) in lojas.iter().take(3).enumerate() {
            loja_ids[i] = id;
        }
        Self {
            n_reg: 0,
            artigo_id,
            preco_antes,
            preco_depois,
            loja_ids,
            alterado_em: agora_unix(),
        }
    }
}

/// Layout mcs_bd2 — 48 bytes.
///
/// ```text
/// @  0  artigo_id u64
/// @  8  preco_antes u32
/// @ 12  preco_depois u32
/// @ 16  loja_id_1 u64
/// @ 24  loja_id_2 u64
/// @ 32  loja_id_3 u64
/// @ 40  alterado_em u64
/// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct RefAlteracaoReg {
    pub artigo_id: u64,
    pub preco_antes: u32,
    pub preco_depois: u32,
    pub loja_id_1: u64,
    pub loja_id_2: u64,
    pub loja_id_3: u64,
    pub alterado_em: u64,
}

const _: () = assert!(std::mem::size_of::<RefAlteracaoReg>() == 48);
const _: () = assert!(std::mem::size_of::<RefAlteracaoReg>() % 8 == 0);

pub const TAM_REG_REF_ALT_U64: u64 = std::mem::size_of::<RefAlteracaoReg>() as u64;

impl RefAlteracaoReg {
    pub fn from_alt(a: &RefAlteracao) -> Self {
        Self {
            artigo_id: a.artigo_id,
            preco_antes: a.preco_antes,
            preco_depois: a.preco_depois,
            loja_id_1: a.loja_ids[0],
            loja_id_2: a.loja_ids[1],
            loja_id_3: a.loja_ids[2],
            alterado_em: a.alterado_em,
        }
    }

    pub fn to_alt(&self, n_reg: u64) -> RefAlteracao {
        RefAlteracao {
            n_reg,
            artigo_id: self.artigo_id,
            preco_antes: self.preco_antes,
            preco_depois: self.preco_depois,
            loja_ids: [self.loja_id_1, self.loja_id_2, self.loja_id_3],
            alterado_em: self.alterado_em,
        }
    }
}
