//! Listas de compras + itens (separado do catálogo Artigo).

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::artigo::{agora_unix, arr_para_str, str_para_arr, Artigo, TAM_NOME};

/// Estado do item na lista de compras.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EstadoItem {
    #[default]
    PorComprar = 0,
    Comprado = 1,
}

impl EstadoItem {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Comprado,
            _ => Self::PorComprar,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListaCompra {
    pub n_reg: u64,
    pub utilizador: u64,
    pub nome: String,
    pub activa: bool,
    pub criada_em: u64,
}

impl ListaCompra {
    pub fn nova(utilizador: u64, nome: &str) -> Self {
        Self {
            n_reg: 0,
            utilizador,
            nome: nome.trim().to_string(),
            activa: true,
            criada_em: agora_unix(),
        }
    }
}

/// Layout mcs_bd2 — 48 bytes.
///
/// ```text
/// @  0  utilizador u64
/// @  8  nome [24]
/// @ 32  activa u8
/// @ 33  _pad [7]
/// @ 40  criada_em u64
/// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ListaCompraReg {
    pub utilizador: u64,
    pub nome: [u8; TAM_NOME],
    pub activa: u8,
    pub _pad: [u8; 7],
    pub criada_em: u64,
}

const _: () = assert!(std::mem::size_of::<ListaCompraReg>() == 48);
const _: () = assert!(std::mem::size_of::<ListaCompraReg>() % 8 == 0);

pub const TAM_REG_LISTA_U64: u64 = std::mem::size_of::<ListaCompraReg>() as u64;

impl ListaCompraReg {
    pub fn from_lista(l: &ListaCompra) -> Self {
        Self {
            utilizador: l.utilizador,
            nome: str_para_arr(&l.nome),
            activa: u8::from(l.activa),
            _pad: [0; 7],
            criada_em: l.criada_em,
        }
    }

    pub fn to_lista(&self, n_reg: u64) -> ListaCompra {
        ListaCompra {
            n_reg,
            utilizador: self.utilizador,
            nome: arr_para_str(&self.nome),
            activa: self.activa != 0,
            criada_em: self.criada_em,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemLista {
    pub n_reg: u64,
    pub lista_id: u64,
    pub artigo_id: u64,
    pub qtd: u16,
    pub unidade: u8,
    pub estado: u8,
    pub criado_em: u64,
}

impl ItemLista {
    pub fn novo(lista_id: u64, artigo: &Artigo, qtd: u16) -> Self {
        Self {
            n_reg: 0,
            lista_id,
            artigo_id: artigo.n_reg,
            qtd: qtd.max(1),
            unidade: artigo.unidade,
            estado: EstadoItem::PorComprar as u8,
            criado_em: agora_unix(),
        }
    }
}

/// Layout mcs_bd2 — 32 bytes.
///
/// ```text
/// @  0  lista_id u64
/// @  8  artigo_id u64
/// @ 16  qtd u16
/// @ 18  unidade u8
/// @ 19  estado u8
/// @ 20  _pad [4]
/// @ 24  criado_em u64
/// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ItemListaReg {
    pub lista_id: u64,
    pub artigo_id: u64,
    pub qtd: u16,
    pub unidade: u8,
    pub estado: u8,
    pub _pad: [u8; 4],
    pub criado_em: u64,
}

const _: () = assert!(std::mem::size_of::<ItemListaReg>() == 32);
const _: () = assert!(std::mem::size_of::<ItemListaReg>() % 8 == 0);

pub const TAM_REG_ITEM_U64: u64 = std::mem::size_of::<ItemListaReg>() as u64;

impl ItemListaReg {
    pub fn from_item(i: &ItemLista) -> Self {
        Self {
            lista_id: i.lista_id,
            artigo_id: i.artigo_id,
            qtd: i.qtd,
            unidade: i.unidade,
            estado: i.estado,
            _pad: [0; 4],
            criado_em: i.criado_em,
        }
    }

    pub fn to_item(&self, n_reg: u64) -> ItemLista {
        ItemLista {
            n_reg,
            lista_id: self.lista_id,
            artigo_id: self.artigo_id,
            qtd: self.qtd,
            unidade: self.unidade,
            estado: self.estado,
            criado_em: self.criado_em,
        }
    }
}

/// Item enriquecido com dados do artigo (resposta API).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemListaDto {
    pub n_reg: u64,
    pub lista_id: u64,
    pub artigo_id: u64,
    pub nome: String,
    pub imag: String,
    pub secao: u8,
    pub qtd: u16,
    pub unidade: u8,
    pub estado: u8,
    /// Preço unitário de referência (cêntimos); 0 = desconhecido.
    #[serde(default)]
    pub preco_ref_cent: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PedidoLista {
    pub nome: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PedidoItem {
    pub artigo_id: u64,
    #[serde(default)]
    pub qtd: u16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PedidoItemPatch {
    #[serde(default)]
    pub qtd: Option<u16>,
    #[serde(default)]
    pub estado: Option<u8>,
    /// Loja desta ida (gravada no histórico ao marcar comprado).
    #[serde(default)]
    pub loja_id: Option<u64>,
    /// Preço da linha em cêntimos. Ao marcar ✓: omitido/0 → usa estimativa (ref × qtd).
    #[serde(default)]
    pub preco_cent: Option<u32>,
}
