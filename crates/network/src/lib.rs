// crates/network/src/lib.rs
use common::{Chunk, NasError};
use std::sync::Arc;
use storage::DiskManager;
use tonic::{Request, Response, Status};
use uuid::Uuid;

// Esta macro mágica importa o código Rust que o tonic-build gerou a partir do 'package nas_rpc'
pub mod proto {
    tonic::include_proto!("nas_rpc");
}

use proto::node_service_server::{NodeService, NodeServiceServer};
use proto::*;

/// Estrutura do nosso servidor gRPC.
/// Usamos Arc (Atomic Reference Counting) porque o Tonic lida com várias 
/// requisições simultâneas em múltiplas threads e todas precisam acessar o mesmo DiskManager.
pub struct RpcServer {
    pub node_id: Uuid,
    pub disk_manager: Arc<DiskManager>,
}

impl RpcServer {
    pub fn new(node_id: Uuid, disk_manager: Arc<DiskManager>) -> Self {
        Self {
            node_id,
            disk_manager,
        }
    }

    /// Função auxiliar para converter a struct em um serviço do Tonic pronto para rodar
    pub fn into_service(self) -> NodeServiceServer<Self> {
        NodeServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl NodeService for RpcServer {
    async fn upload_chunk(
        &self,
        request: Request<UploadChunkRequest>,
    ) -> std::result::Result<Response<UploadChunkResponse>, Status> {
        let req = request.into_inner();

        // Converte a String recebida via rede de volta para Uuid
        let file_id = Uuid::parse_str(&req.file_id)
            .map_err(|_| Status::invalid_argument("file_id não é um UUID válido"))?;

        // Chama o seu DiskManager para gravar no HD e gerar o hash SHA-256
        let chunk = self
            .disk_manager
            .write_chunk(file_id, req.sequence_index, &req.data)
            .await
            .map_err(|e| Status::internal(format!("Falha ao gravar no disco: {}", e)))?;

        Ok(Response::new(UploadChunkResponse {
            chunk_id: chunk.id.to_string(),
            checksum: chunk.checksum,
            size_bytes: chunk.size_bytes,
        }))
    }

    async fn download_chunk(
        &self,
        request: Request<DownloadChunkRequest>,
    ) -> std::result::Result<Response<DownloadChunkResponse>, Status> {
        let req = request.into_inner();

        let chunk_id = Uuid::parse_str(&req.chunk_id)
            .map_err(|_| Status::invalid_argument("chunk_id inválido"))?;
        let file_id = Uuid::parse_str(&req.file_id)
            .map_err(|_| Status::invalid_argument("file_id inválido"))?;

        // Reconstruímos a struct Chunk para passar ao DiskManager validar contra bit-rot
        let chunk_meta = Chunk {
            id: chunk_id,
            file_id,
            sequence_index: req.sequence_index,
            size_bytes: req.size_bytes,
            checksum: req.expected_checksum,
        };

        match self.disk_manager.read_chunk(&chunk_meta).await {
            Ok(data) => Ok(Response::new(DownloadChunkResponse { data })),
            Err(NasError::CorruptedData { chunk_id }) => {
                // Se o HD antigo corrompeu o arquivo, avisamos a rede com o código DATA_LOSS do gRPC!
                // Assim o cliente sabe que deve buscar a réplica em outro computador.
                Err(Status::data_loss(format!(
                    "Bit-rot detectado no chunk {}",
                    chunk_id
                )))
            }
            Err(e) => Err(Status::internal(format!("Erro de leitura: {}", e))),
        }
    }

    async fn ping(
        &self,
        _request: Request<PingRequest>,
    ) -> std::result::Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            node_id: self.node_id.to_string(),
            free_space_bytes: 500 * 1024 * 1024 * 1024, // Futuramente leremos o espaço real via statvfs
        }))
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use proto::node_service_client::NodeServiceClient;
    use tokio::net::TcpListener;
    use tonic::transport::Server;

    #[tokio::test]
    async fn test_grpc_upload_and_download_chunk() {
        // 1. Prepara um diretório temporário para o servidor salvar os chunks
        let temp_dir = tempfile::tempdir().unwrap();
        let disk_manager = Arc::new(DiskManager::new(temp_dir.path()).await.unwrap());
        let node_id = Uuid::new_v4();
        let rpc_server = RpcServer::new(node_id, disk_manager);

        // 2. Faz bind na porta 0 (o Linux escolhe uma porta livre automaticamente para não dar conflito)
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // 3. Inicia o servidor gRPC em background usando tokio::spawn
        tokio::spawn(async move {
            let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
            Server::builder()
                .add_service(rpc_server.into_service())
                .serve_with_incoming(incoming)
                .await
                .unwrap();
        });

        // 4. Conecta o Cliente gRPC ao endereço do servidor
        let endpoint = format!("http://{}", addr);
        let mut client = NodeServiceClient::connect(endpoint).await.unwrap();

        // 5. Teste de Upload: Envia um chunk pela rede
        let file_id = Uuid::new_v4().to_string();
        let payload = b"Conteudo do chunk enviado via gRPC pela rede local!".to_vec();

        let upload_req = Request::new(UploadChunkRequest {
            file_id: file_id.clone(),
            sequence_index: 0,
            data: payload.clone(),
        });

        let upload_res = client.upload_chunk(upload_req).await.unwrap().into_inner();
        println!("Chunk salvo remotamente! ID: {}, Hash: {}", upload_res.chunk_id, upload_res.checksum);

        // 6. Teste de Download: Pede o chunk de volta exigindo validação de bit-rot
        let download_req = Request::new(DownloadChunkRequest {
            chunk_id: upload_res.chunk_id,
            file_id,
            sequence_index: 0,
            size_bytes: upload_res.size_bytes,
            expected_checksum: upload_res.checksum,
        });

        let download_res = client.download_chunk(download_req).await.unwrap().into_inner();

        // Garante que os bytes que viajaram ida e volta pela rede e pelo disco são idênticos
        assert_eq!(payload, download_res.data);
    }
}