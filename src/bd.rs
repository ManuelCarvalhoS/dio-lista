//! Persistência mcs_bd2 — catálogo `Artigo` (= lista_comum, 64 B) + imgs/.

use lista_comum::{
    agora_unix, nome_ficheiro_imag, Artigo, ArtigoReg, TAM_REG_ARTIGO_U64,
};
use bytemuck::{bytes_of, Pod};
use mcs_bd2::estrutura::{Cabecalho, EntitiesMap, EntityFiles, Registo, TAMANHO_CABECALHO};
use mcs_bd2::ler_registo_aberto;
use mcs_bd2::op_ger_tab::gravar_com_indices;
use mcs_bd2::{abrir_ficheiros, CampoIndice};
use std::io::{Read, Seek, SeekFrom};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

pub static INDICES_VAZIOS: &[CampoIndice] = &[];

type BdHandle = Arc<Mutex<EntityFiles>>;

static MAPA: OnceLock<EntitiesMap> = OnceLock::new();

const MAX_IMG_BYTES: usize = 512_000;
/// Marca o layout catálogo 64 B (substitui o Artigo 96 B antigo).
const LAYOUT_STAMP: &str = ".layout_artigo_v2";

fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("LISTA_DATA_DIR") {
        let path = PathBuf::from(p);
        let _ = std::fs::create_dir_all(path.join("imgs"));
        return path;
    }
    #[cfg(target_os = "android")]
    {
        return android_data_dir();
    }
    #[cfg(not(target_os = "android"))]
    {
        let p = PathBuf::from("data");
        let _ = std::fs::create_dir_all(p.join("imgs"));
        p
    }
}

#[cfg(target_os = "android")]
fn android_data_dir() -> PathBuf {
    if let Ok(raw) = std::fs::read("/proc/self/cmdline") {
        let pkg = String::from_utf8_lossy(&raw)
            .split('\0')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if pkg.contains('.') && !pkg.contains('/') {
            let p = PathBuf::from(format!("/data/data/{pkg}/files/lista_bd"));
            if std::fs::create_dir_all(p.join("imgs")).is_ok() {
                return p;
            }
        }
    }
    for pkg in ["pt.mcs.lista", "com.example.DioLista"] {
        let p = PathBuf::from(format!("/data/data/{pkg}/files/lista_bd"));
        if std::fs::create_dir_all(p.join("imgs")).is_ok() {
            return p;
        }
    }
    let p = PathBuf::from("lista_bd");
    let _ = std::fs::create_dir_all(p.join("imgs"));
    p
}

/// Uma vez: apaga artigo 96 B e tabelas marca/loja/nota (já não no modelo).
fn migrar_layout_se_preciso(dir: &Path) {
    let stamp = dir.join(LAYOUT_STAMP);
    if stamp.exists() {
        return;
    }
    for nome in ["artigo", "marca", "loja", "nota"] {
        for ext in ["dad", "h1", "h2"] {
            let _ = std::fs::remove_file(dir.join(format!("{nome}.{ext}")));
        }
    }
    let _ = std::fs::write(&stamp, b"artigo_64_catalogo\n");
}

fn entities() -> Vec<(String, u32, Vec<String>, u64, &'static [CampoIndice])> {
    vec![(
        "artigo".into(),
        1,
        vec!["artigo.dad".into(), "artigo.h1".into(), "artigo.h2".into()],
        TAM_REG_ARTIGO_U64,
        INDICES_VAZIOS,
    )]
}

fn mapa() -> anyhow::Result<&'static EntitiesMap> {
    if let Some(m) = MAPA.get() {
        return Ok(m);
    }
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let _ = std::fs::create_dir_all(dir.join("imgs"));
    migrar_layout_se_preciso(&dir);
    let dir_s = dir.to_string_lossy().to_string();
    let m = abrir_ficheiros(&dir, &dir_s, entities())
        .map_err(|e| anyhow::anyhow!("abrir_ficheiros ({}): {e}", dir.display()))?;
    let _ = MAPA.set(m);
    MAPA.get()
        .ok_or_else(|| anyhow::anyhow!("mapa BD não inicializado"))
}

fn ent(nome: &str) -> anyhow::Result<BdHandle> {
    mapa()?
        .get(nome)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("entidade '{nome}' em falta — apaga data/ se layout mudou"))
}

fn gravar_pod<T: Pod + Send>(bd: &BdHandle, dados: T) -> anyhow::Result<u64> {
    let (tamanho_reg, indices) = {
        let g = bd.lock().map_err(|_| anyhow::anyhow!("mutex poisoned"))?;
        (g.tamanho_reg, g.indices)
    };
    let reg = Registo {
        cab: Cabecalho { n_reg: 0, cc: 100 },
        dados,
    };
    let mut bytes = bytes_of(&reg).to_vec();
    gravar_com_indices(bd, 0, tamanho_reg, &mut bytes, indices)
        .map_err(|e| anyhow::anyhow!("gravar: {e}"))
}

fn alterar_pod<T: Pod + Send>(bd: &BdHandle, dados: T, pos: u64) -> anyhow::Result<u64> {
    let (tamanho_reg, indices) = {
        let g = bd.lock().map_err(|_| anyhow::anyhow!("mutex poisoned"))?;
        (g.tamanho_reg, g.indices)
    };
    let reg = Registo {
        cab: Cabecalho {
            n_reg: pos,
            cc: 120,
        },
        dados,
    };
    let mut bytes = bytes_of(&reg).to_vec();
    gravar_com_indices(bd, pos, tamanho_reg, &mut bytes, indices)
        .map_err(|e| anyhow::anyhow!("alterar: {e}"))
}

fn listar_pod<T: Pod + Copy>(bd: &BdHandle) -> anyhow::Result<Vec<(u64, T)>> {
    let mut guard = bd.lock().map_err(|_| anyhow::anyhow!("mutex poisoned"))?;
    let tamanho_reg = guard.tamanho_reg;
    let dad = guard
        .dad
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!(".dad em falta"))?;
    let mut buf8 = [0u8; 8];
    dad.seek(SeekFrom::Start(0))?;
    dad.read_exact(&mut buf8)?;
    let n = u64::from_le_bytes(buf8);
    let mut out = Vec::new();
    for n_reg in 1..n {
        if let Ok(bytes) = ler_registo_aberto(dad, n_reg, tamanho_reg) {
            let cc = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
            if cc != 200 {
                let dados = &bytes[TAMANHO_CABECALHO as usize..];
                if dados.len() == std::mem::size_of::<T>() {
                    out.push((n_reg, *bytemuck::from_bytes::<T>(dados)));
                }
            }
        }
    }
    Ok(out)
}

pub fn backend_label() -> String {
    #[cfg(target_os = "android")]
    {
        format!("mcs_bd2 · {}", data_dir().display())
    }
    #[cfg(not(target_os = "android"))]
    {
        "mcs_bd2".into()
    }
}

pub fn listar() -> Result<Vec<Artigo>, String> {
    wrap(|| {
        let regs = listar_pod::<ArtigoReg>(&ent("artigo")?)?;
        Ok(regs.into_iter().map(|(n, r)| r.to_artigo(n)).collect())
    })
}

pub fn adicionar(artigo: Artigo) -> Result<Artigo, String> {
    let mut a = artigo;
    let nome = a.nome.trim().to_string();
    if nome.is_empty() {
        return Err("Indica o nome do artigo.".into());
    }
    if nome.as_bytes().len() > lista_comum::TAM_NOME {
        return Err(format!("Nome: máximo {} bytes.", lista_comum::TAM_NOME));
    }
    a.nome = nome;
    wrap(move || {
        let mut reg = ArtigoReg::from_artigo(&a);
        let n_reg = gravar_pod(&ent("artigo")?, reg)?;
        if n_reg == 0 {
            return Err(anyhow::anyhow!("Falha ao gravar artigo."));
        }
        if a.imag.trim().is_empty() {
            let imag = nome_ficheiro_imag(n_reg);
            reg.imag = lista_comum::str_para_arr(&imag);
            alterar_pod(&ent("artigo")?, reg, n_reg)?;
            a.imag = imag;
        }
        a.n_reg = n_reg;
        Ok(a)
    })
}

/// Grava ficheiro em `imgs/` e devolve o nome a guardar em `Artigo.imag` (≤16).
pub fn guardar_imagem(bytes: &[u8]) -> Result<String, String> {
    if bytes.is_empty() {
        return Err("Imagem vazia.".into());
    }
    if bytes.len() > MAX_IMG_BYTES {
        return Err("Imagem demasiado grande (máx. ~500 KB). Usa um thumbnail.".into());
    }
    let ext = sniff_ext(bytes);
    // i + 8 hex + . + ext  → cabe em imag[16]
    let ficheiro = format!("i{:08x}.{ext}", agora_unix() as u32);
    if ficheiro.len() > lista_comum::TAM_IMAG {
        return Err("Nome de imagem inválido.".into());
    }
    let path = data_dir().join("imgs").join(&ficheiro);
    std::fs::write(&path, bytes).map_err(|e| format!("gravar imagem: {e}"))?;
    Ok(ficheiro)
}

pub fn url_imagem(imag: &str) -> Option<String> {
    if imag.is_empty() {
        return None;
    }
    let dir = data_dir().join("imgs");
    let directo = dir.join(imag);
    if directo.exists() {
        return Some(format!("file://{}", directo.display()));
    }
    // legado: só id sem extensão
    for ext in ["webp", "jpg", "jpeg", "png", "gif"] {
        let p = dir.join(format!("{imag}.{ext}"));
        if p.exists() {
            return Some(format!("file://{}", p.display()));
        }
    }
    None
}

fn sniff_ext(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpg"
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "webp"
    } else if bytes.starts_with(b"GIF8") {
        "gif"
    } else {
        "jpg"
    }
}

fn wrap<T>(f: impl FnOnce() -> anyhow::Result<T> + std::panic::UnwindSafe) -> Result<T, String> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("panic interno (mcs_bd2)".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lista_comum::UTILIZADOR_BASE;

    #[test]
    fn gravar_artigo_catalogo() {
        let dir = std::env::temp_dir().join(format!("lista_cat_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe {
            std::env::set_var("LISTA_DATA_DIR", &dir);
        }

        let a = Artigo::novo_base("leite");
        assert_eq!(a.utilizador, UTILIZADOR_BASE);
        let a = adicionar(a).unwrap();
        assert!(a.n_reg >= 1);
        assert_eq!(a.imag, nome_ficheiro_imag(a.n_reg));
        let lista = listar().unwrap();
        assert_eq!(lista.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
