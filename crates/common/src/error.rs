// crates/common/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NasError {
    // A conversão mágica: O #[from] converte std::io::Error automaticamente para NasError::Io
    #[error("Erro de I/O no sistema operacional: {0}")]
    Io(#[from] std::io::Error),

    // Erros específicos de rede e descobrimento de nós
    #[error("Falha de comunicação na rede: {0}")]
    Network(String),

    // Erros de disco (crucial para hardwares antigos)
    #[error("Integridade de dados comprometida no chunk {chunk_id}")]
    CorruptedData { chunk_id: String },

    #[error("Espaço em disco insuficiente no nó")]
    StorageFull,

    // Erros lógicos do cluster
    #[error("O nó atual não é o líder do consenso Raft")]
    NotLeader,

    #[error("Nó {0} não encontrado ou inativo")]
    NodeOffline(String),
}

// Criamos um alias de Result global para o projeto.
// Isso evita ter que digitar `Result<T, NasError>` o tempo todo nos outros crates.
pub type Result<T> = std::result::Result<T, NasError>;