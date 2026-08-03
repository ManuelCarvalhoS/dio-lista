//! Lojas (sítios onde se compra) — base partilhada + pessoais.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::artigo::{arr_para_str, str_para_arr, TAM_NOME, UTILIZADOR_BASE};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Loja {
    pub n_reg: u64,
    pub utilizador: u64,
    pub nome: String,
}

impl Loja {
    pub fn e_base(&self) -> bool {
        self.utilizador == UTILIZADOR_BASE
    }
}

/// Layout mcs_bd2 — 32 bytes.
///
/// ```text
/// @  0  utilizador u64   (1 = base partilhada; ≥2 = pessoal)
/// @  8  nome [24]
/// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct LojaReg {
    pub utilizador: u64,
    pub nome: [u8; TAM_NOME],
}

const _: () = assert!(std::mem::size_of::<LojaReg>() == 32);
const _: () = assert!(std::mem::size_of::<LojaReg>() % 8 == 0);

pub const TAM_REG_LOJA_U64: u64 = std::mem::size_of::<LojaReg>() as u64;

impl LojaReg {
    pub fn from_nome(utilizador: u64, nome: &str) -> Self {
        Self {
            utilizador,
            nome: str_para_arr(nome.trim()),
        }
    }

    pub fn to_loja(&self, n_reg: u64) -> Loja {
        Loja {
            n_reg,
            utilizador: self.utilizador,
            nome: arr_para_str(&self.nome),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PedidoLoja {
    pub nome: String,
}

/// Lojas iniciais (PT) — catálogo base.
pub const LOJAS_SEED: &[&str] = &[
    "Continente",
    "Pingo Doce",
    "Auchan",
    "Aldi",
    "Lidl",
    "Mercadona",
    "Intermarché",
    "Minipreço",
    "Outra",
];
