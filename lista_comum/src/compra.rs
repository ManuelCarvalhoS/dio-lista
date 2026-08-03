//! Registo de compra (histórico ao marcar item como comprado).

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::artigo::agora_unix;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Compra {
    pub n_reg: u64,
    pub utilizador: u64,
    pub artigo_id: u64,
    pub lista_id: u64,
    pub qtd: u16,
    pub unidade: u8,
    /// Cêntimos; 0 = não indicado.
    pub preco_cent: u32,
    pub loja_id: u64,
    pub comprado_em: u64,
}

impl Compra {
    pub fn nova(
        utilizador: u64,
        artigo_id: u64,
        lista_id: u64,
        qtd: u16,
        unidade: u8,
        preco_cent: u32,
        loja_id: u64,
    ) -> Self {
        Self {
            n_reg: 0,
            utilizador,
            artigo_id,
            lista_id,
            qtd: qtd.max(1),
            unidade,
            preco_cent,
            loja_id,
            comprado_em: agora_unix(),
        }
    }
}

/// Layout mcs_bd2 — 48 bytes.
///
/// ```text
/// @  0  utilizador u64
/// @  8  artigo_id u64
/// @ 16  lista_id u64
/// @ 24  qtd u16
/// @ 26  unidade u8
/// @ 27  _pad u8
/// @ 28  preco_cent u32
/// @ 32  loja_id u64
/// @ 40  comprado_em u64
/// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CompraReg {
    pub utilizador: u64,
    pub artigo_id: u64,
    pub lista_id: u64,
    pub qtd: u16,
    pub unidade: u8,
    pub _pad: u8,
    pub preco_cent: u32,
    pub loja_id: u64,
    pub comprado_em: u64,
}

const _: () = assert!(std::mem::size_of::<CompraReg>() == 48);
const _: () = assert!(std::mem::size_of::<CompraReg>() % 8 == 0);

pub const TAM_REG_COMPRA_U64: u64 = std::mem::size_of::<CompraReg>() as u64;

impl CompraReg {
    pub fn from_compra(c: &Compra) -> Self {
        Self {
            utilizador: c.utilizador,
            artigo_id: c.artigo_id,
            lista_id: c.lista_id,
            qtd: c.qtd,
            unidade: c.unidade,
            _pad: 0,
            preco_cent: c.preco_cent,
            loja_id: c.loja_id,
            comprado_em: c.comprado_em,
        }
    }

    pub fn to_compra(&self, n_reg: u64) -> Compra {
        Compra {
            n_reg,
            utilizador: self.utilizador,
            artigo_id: self.artigo_id,
            lista_id: self.lista_id,
            qtd: self.qtd,
            unidade: self.unidade,
            preco_cent: self.preco_cent,
            loja_id: self.loja_id,
            comprado_em: self.comprado_em,
        }
    }
}

/// Quantos preços unitários recentes (por utilizador) entram no histórico da UI.
pub const N_PRECOS_MODA: usize = 7;

/// Moda do preço de referência no catálogo: últimos N preços unitários **de todos** os utilizadores.
/// 500 é um bom compromisso (robusto a outliers, leve no job diário).
pub const N_PRECOS_REF_GLOBAL: usize = 500;

/// Moda dos valores; em empate, fica o mais recente (último no slice).
pub fn moda_centimos(valores: &[u32]) -> Option<u32> {
    if valores.is_empty() {
        return None;
    }
    let mut freq: std::collections::HashMap<u32, (usize, usize)> =
        std::collections::HashMap::new();
    for (i, &v) in valores.iter().enumerate() {
        let e = freq.entry(v).or_insert((0, i));
        e.0 += 1;
        e.1 = i;
    }
    freq.into_iter()
        .max_by_key(|(_, (n, i))| (*n, *i))
        .map(|(v, _)| v)
}

/// Preço unitário (cêntimos) a partir do valor de linha e da quantidade.
pub fn preco_unitario(preco_linha: u32, qtd: u16) -> u32 {
    let q = u32::from(qtd.max(1));
    preco_linha / q
}

/// Entrada de histórico de preços (API / UI).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompraHistorico {
    pub n_reg: u64,
    pub loja: String,
    pub loja_id: u64,
    /// Preço da linha (cêntimos).
    pub preco_cent: u32,
    /// Preço unitário (cêntimos).
    pub preco_unit_cent: u32,
    pub qtd: u16,
    pub comprado_em: u64,
}

/// Quantas compras recentes mostrar no histórico do produto.
pub const N_HISTORICO_PRECOS: usize = 8;
