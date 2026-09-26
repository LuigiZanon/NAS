// crates/common/src/models.rs
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

/// Representa o estado atual de um computador antigo no cluster
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    Online,
    Offline,
    Syncing, // Quando o nó acabou de ligar e está baixando os metadados
}

/// Identifica um computador físico na rede
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub address: SocketAddr, // IP e Porta (ex: 10.0.0.5:8080)
    pub status: NodeStatus,
    pub free_space_bytes: u64,
}

/// Representa um pedaço físico de um arquivo no disco
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: Uuid,
    pub file_id: Uuid,       // A qual arquivo este pedaço pertence
    pub sequence_index: u32, // Ordem do pedaço (0, 1, 2...)
    pub size_bytes: u32,
    pub checksum: String,    // Hash (ex: SHA256) para checar o bit-rot dos HDDs antigos
}

/// Representa os metadados de um arquivo completo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: Uuid,
    pub name: String,
    pub total_size: u64,
    pub chunks: Vec<Uuid>,   // Lista de IDs dos pedaços que compõem o arquivo
}