//! Ida ao supermercado (sessão de compras sobre uma lista).

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::artigo::agora_unix;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ida {
    pub n_reg: u64,
    pub utilizador: u64,
    pub lista_id: u64,
    pub loja_id: u64,
    pub iniciada_em: u64,
}

impl Ida {
    pub fn nova(utilizador: u64, lista_id: u64, loja_id: u64) -> Self {
        Self {
            n_reg: 0,
            utilizador,
            lista_id,
            loja_id,
            iniciada_em: agora_unix(),
        }
    }
}

/// Layout mcs_bd2 — 32 bytes.
///
/// ```text
/// @  0  utilizador u64
/// @  8  lista_id u64
/// @ 16  loja_id u64
/// @ 24  iniciada_em u64
/// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct IdaReg {
    pub utilizador: u64,
    pub lista_id: u64,
    pub loja_id: u64,
    pub iniciada_em: u64,
}

const _: () = assert!(std::mem::size_of::<IdaReg>() == 32);
const _: () = assert!(std::mem::size_of::<IdaReg>() % 8 == 0);

pub const TAM_REG_IDA_U64: u64 = std::mem::size_of::<IdaReg>() as u64;

impl IdaReg {
    pub fn from_ida(i: &Ida) -> Self {
        Self {
            utilizador: i.utilizador,
            lista_id: i.lista_id,
            loja_id: i.loja_id,
            iniciada_em: i.iniciada_em,
        }
    }

    pub fn to_ida(&self, n_reg: u64) -> Ida {
        Ida {
            n_reg,
            utilizador: self.utilizador,
            lista_id: self.lista_id,
            loja_id: self.loja_id,
            iniciada_em: self.iniciada_em,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PedidoNovaIda {
    #[serde(default)]
    pub loja_id: u64,
}
