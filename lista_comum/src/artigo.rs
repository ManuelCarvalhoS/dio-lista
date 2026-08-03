//! Catálogo de produtos Lista.
//!
//! `utilizador == 1` → catálogo geral (gestor; = n_reg típico do Admin LabNetCol).
//! `utilizador == n_reg` (≥ 2, ou outro) → produto pessoal desse utilizador.
//! `utilizador == 0` → reservado (“sem dono” / ausência), não se usa em produtos.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

pub const TAM_NOME: usize = 24;
pub const TAM_IMAG: usize = 16;
pub const TAM_LABEL: usize = 24;

/// Catálogo geral (gestor). Coincide com o `n_reg` habitual do Admin LabNetCol.
/// `0` fica reservado para “ausência”, não para produtos.
pub const UTILIZADOR_BASE: u64 = 1;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Unidade {
    #[default]
    Un = 0,
    Kg = 1,
    G = 2,
    L = 3,
    Ml = 4,
    Pack = 5,
}

impl Unidade {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Kg,
            2 => Self::G,
            3 => Self::L,
            4 => Self::Ml,
            5 => Self::Pack,
            _ => Self::Un,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Un => "un",
            Self::Kg => "kg",
            Self::G => "g",
            Self::L => "L",
            Self::Ml => "ml",
            Self::Pack => "pack",
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Secao {
    #[default]
    Sem = 0,
    Frescos = 1,
    Talho = 2,
    Peixaria = 3,
    Laticinios = 4,
    Mercearia = 5,
    Bebidas = 6,
    Limpeza = 7,
    Higiene = 8,
    Congelados = 9,
    Outros = 10,
    Fruta = 11,
}

impl Secao {
    pub const ALL: [Secao; 12] = [
        Self::Sem,
        Self::Frescos,
        Self::Fruta,
        Self::Talho,
        Self::Peixaria,
        Self::Laticinios,
        Self::Mercearia,
        Self::Bebidas,
        Self::Limpeza,
        Self::Higiene,
        Self::Congelados,
        Self::Outros,
    ];

    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Frescos,
            2 => Self::Talho,
            3 => Self::Peixaria,
            4 => Self::Laticinios,
            5 => Self::Mercearia,
            6 => Self::Bebidas,
            7 => Self::Limpeza,
            8 => Self::Higiene,
            9 => Self::Congelados,
            10 => Self::Outros,
            11 => Self::Fruta,
            _ => Self::Sem,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sem => "—",
            Self::Frescos => "Frescos",
            Self::Fruta => "Fruta",
            Self::Talho => "Talho",
            Self::Peixaria => "Peixaria",
            Self::Laticinios => "Laticínios",
            Self::Mercearia => "Mercearia",
            Self::Bebidas => "Bebidas",
            Self::Limpeza => "Limpeza",
            Self::Higiene => "Higiene",
            Self::Congelados => "Congelados",
            Self::Outros => "Outros",
        }
    }
}

/// DTO — produto do catálogo (base ou pessoal).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Artigo {
    pub n_reg: u64,
    /// 1 = catálogo geral (Admin); senão n_reg LabNetCol (pessoal).
    pub utilizador: u64,
    pub nome: String,
    pub imag: String,
    pub unidade: u8,
    pub secao: u8,
    /// Cêntimos (moda dos últimos preços registados); 0 = ainda sem dados.
    #[serde(default)]
    pub preco_referencia: u32,
    pub criado_em: u64,
}

impl Artigo {
    pub fn novo_base(nome: &str) -> Self {
        Self {
            n_reg: 0,
            utilizador: UTILIZADOR_BASE,
            nome: nome.trim().to_string(),
            imag: String::new(),
            unidade: Unidade::Un as u8,
            secao: Secao::Sem as u8,
            preco_referencia: 0,
            criado_em: agora_unix(),
        }
    }

    pub fn e_base(&self) -> bool {
        self.utilizador == UTILIZADOR_BASE
    }

    pub fn unidade_label(&self) -> &'static str {
        Unidade::from_u8(self.unidade).label()
    }

    pub fn secao_label(&self) -> &'static str {
        Secao::from_u8(self.secao).label()
    }
}

pub fn agora_unix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn str_para_arr<const N: usize>(s: &str) -> [u8; N] {
    let mut arr = [0u8; N];
    let b = s.as_bytes();
    let n = b.len().min(N);
    arr[..n].copy_from_slice(&b[..n]);
    arr
}

pub fn arr_para_str(arr: &[u8]) -> String {
    let n = arr.iter().rposition(|&b| b != 0).map(|i| i + 1).unwrap_or(0);
    String::from_utf8_lossy(&arr[..n]).into_owned()
}

/// Layout mcs_bd2 — 64 bytes.
///
/// ```text
/// @  0  utilizador u64   (1 = catálogo geral / Admin; 0 = não usar)
/// @  8  nome [24]
/// @ 32  imag [16]
/// @ 48  unidade u8
/// @ 49  secao u8
/// @ 50  _pad [2]
/// @ 52  preco_referencia u32  (cêntimos; 0 = desconhecido)
/// @ 56  criado_em u64
/// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ArtigoReg {
    pub utilizador: u64,
    pub nome: [u8; TAM_NOME],
    pub imag: [u8; TAM_IMAG],
    pub unidade: u8,
    pub secao: u8,
    pub _pad: [u8; 2],
    pub preco_referencia: u32,
    pub criado_em: u64,
}

const _: () = assert!(std::mem::size_of::<ArtigoReg>() == 64);
const _: () = assert!(std::mem::size_of::<ArtigoReg>() % 8 == 0);

pub const TAM_REG_ARTIGO_U64: u64 = std::mem::size_of::<ArtigoReg>() as u64;

impl ArtigoReg {
    pub fn from_artigo(a: &Artigo) -> Self {
        Self {
            utilizador: a.utilizador,
            nome: str_para_arr(&a.nome),
            imag: str_para_arr(&a.imag),
            unidade: a.unidade,
            secao: a.secao,
            _pad: [0; 2],
            preco_referencia: a.preco_referencia,
            criado_em: a.criado_em,
        }
    }

    pub fn to_artigo(&self, n_reg: u64) -> Artigo {
        Artigo {
            n_reg,
            utilizador: self.utilizador,
            nome: arr_para_str(&self.nome),
            imag: arr_para_str(&self.imag),
            unidade: self.unidade,
            secao: self.secao,
            preco_referencia: self.preco_referencia,
            criado_em: self.criado_em,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PedidoArtigo {
    pub nome: String,
    #[serde(default)]
    pub imag: String,
    #[serde(default)]
    pub unidade: u8,
    #[serde(default)]
    pub secao: u8,
}

impl PedidoArtigo {
    pub fn para_base(&self) -> Artigo {
        let mut a = Artigo::novo_base(&self.nome);
        a.imag = self.imag.trim().to_string();
        a.unidade = self.unidade;
        a.secao = self.secao;
        a
    }

    /// Produto só deste utilizador LabNetCol (`utilizador == n_reg`, ≠ base).
    pub fn para_pessoal(&self, utilizador: u64) -> Artigo {
        let mut a = self.para_base();
        a.utilizador = utilizador;
        a
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessaoLista {
    pub token: String,
    pub labnetcol_id: u64,
    pub nome: String,
    #[serde(default)]
    pub admin: bool,
}

// Labels (marca/loja/nota) — passam a ser usados nas compras, mais tarde.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LabelItem {
    pub n_reg: u64,
    pub nome: String,
}
