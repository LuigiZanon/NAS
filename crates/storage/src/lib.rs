use common::{Chunk, NasError, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

pub struct DiskManager {
    base_dir: PathBuf,
}

impl DiskManager {
    /// Inicializa o gerenciador e garante que o diretório base exista
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let base_dir = path.as_ref().to_path_buf();
        
        if !base_dir.exists() {
            // Se a pasta não existir, cria de forma assíncrona
            fs::create_dir_all(&base_dir).await?;
        }
        
        Ok(Self { base_dir })
    }

    /// Retorna o caminho físico do arquivo baseado no ID do chunk
    fn get_chunk_path(&self, chunk_id: &Uuid) -> PathBuf {
        // Os chunks serão salvos com o UUID como nome do arquivo
        self.base_dir.join(chunk_id.to_string())
    }

    /// Função auxiliar para calcular o SHA-256 e retornar uma String Hexadecimal
    fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        result.iter().map(|b| format!("{:02x}", b)).collect() // Formata para hexadecimal
    }

    /// Grava um novo pedaço de dado no disco
    pub async fn write_chunk(&self, file_id: Uuid, sequence_index: u32, data: &[u8]) -> Result<Chunk> {
        let chunk_id = Uuid::new_v4();
        let chunk_path = self.get_chunk_path(&chunk_id);
        
        // 1. Calcula o hash dos dados em memória
        let checksum = Self::calculate_hash(data);

        // 2. Grava no disco de forma assíncrona
        let mut file = File::create(&chunk_path).await?;
        file.write_all(data).await?;
        
        // CRÍTICO PARA HDDs ANTIGOS: O Linux costuma manter os dados em cache de RAM (Page Cache).
        // O `sync_all` força o disco físico a rodar a agulha e gravar no prato magnético antes de retornar Ok.
        file.sync_all().await?; 

        // 3. Retorna os metadados do chunk (isso será enviado ao módulo `metadata` depois)
        Ok(Chunk {
            id: chunk_id,
            file_id,
            sequence_index,
            size_bytes: data.len() as u32,
            checksum,
        })
    }

    /// Lê um pedaço de arquivo e verifica a integridade contra bit-rot
    pub async fn read_chunk(&self, chunk_metadata: &Chunk) -> Result<Vec<u8>> {
        let chunk_path = self.get_chunk_path(&chunk_metadata.id);
        
        let mut file = File::open(&chunk_path).await?;
        let mut data = Vec::with_capacity(chunk_metadata.size_bytes as usize);
        
        // Lê todo o conteúdo do disco para a memória
        file.read_to_end(&mut data).await?;

        // 1. Recalcula o hash do que acabou de ser lido do disco rígido
        let current_hash = Self::calculate_hash(&data);

        // 2. Compara com o hash original guardado nos metadados
        if current_hash != chunk_metadata.checksum {
            // Se for diferente, detectamos bit-rot ou falha de disco!
            return Err(NasError::CorruptedData {
                chunk_id: chunk_metadata.id.to_string(),
            });
        }

        Ok(data)
    }
}






// ! ========= TESTES ========= !

#[cfg(test)]
mod tests {
    use super::*; // Importa tudo (DiskManager, etc) do módulo pai
    use common::NasError;
    use tempfile::tempdir;
    use tokio::fs::OpenOptions;

    #[tokio::test]
    async fn test_escrita_e_leitura_sucesso() {
        // Cria um diretório temporário no SO (ex: /tmp/.tmpXzY)
        let dir = tempdir().unwrap();
        let dm = DiskManager::new(dir.path()).await.unwrap();

        let data = b"Dados perfeitamente saudaveis do NAS";
        let file_id = Uuid::new_v4();

        // 1. Grava o chunk
        let chunk = dm.write_chunk(file_id, 0, data).await.unwrap();

        // 2. Lê o chunk
        let read_data = dm.read_chunk(&chunk).await.unwrap();

        // 3. Valida se o conteúdo é o mesmo
        assert_eq!(data.as_slice(), read_data.as_slice());
    }

    #[tokio::test]
    async fn test_deteccao_de_bit_rot() {
        let dir = tempdir().unwrap();
        let dm = DiskManager::new(dir.path()).await.unwrap();

        let data = b"Arquivo muito importante";
        let file_id = Uuid::new_v4();

        // 1. Grava o chunk normalmente
        let chunk = dm.write_chunk(file_id, 0, data).await.unwrap();

        // 2. SIMULAÇÃO DE BIT-ROT (Corrupção silenciosa de hardware)
        // Vamos abrir o arquivo fisicamente por fora do DiskManager e alterar 1 byte
        let chunk_path = dm.get_chunk_path(&chunk.id);
        
        let mut file = OpenOptions::new()
            .write(true)
            .open(&chunk_path)
            .await
            .unwrap();
            
        // Sobrescreve o primeiro byte do arquivo com a letra 'X'
        file.write_all(b"X").await.unwrap();
        file.sync_all().await.unwrap();

        // 3. Tentamos ler o arquivo usando nosso gerenciador
        let result = dm.read_chunk(&chunk).await;

        // 4. Validamos se o sistema detectou a corrupção corretamente
        assert!(
            result.is_err(), 
            "O sistema deveria ter bloqueado a leitura por hash incorreto!"
        );

        // Verifica se o erro gerado é exatamente o CorruptedData esperado
        match result {
            Err(NasError::CorruptedData { chunk_id }) => {
                assert_eq!(chunk_id, chunk.id.to_string());
            }
            _ => panic!("O erro retornado não foi CorruptedData"),
        }
    }
}