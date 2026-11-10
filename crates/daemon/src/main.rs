use common::{NasError, Node, NodeStatus, Result};
use network::RpcServer;
use std::net::SocketAddr;
use std::sync::Arc;
use storage::DiskManager;
use tonic::transport::Server;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Iniciando o Node Daemon do NAS Distribuído...");

    // Usamos 127.0.0.1 para testes locais (ou 0.0.0.0:9000 para ouvir todas as placas de rede)
    let my_address: SocketAddr = "127.0.0.1:9000".parse().unwrap();
    let my_node = Node {
        id: Uuid::new_v4(),
        address: my_address,
        status: NodeStatus::Online,
        free_space_bytes: 500 * 1024 * 1024 * 1024,
    };

    // 1. Inicializa o gerenciador de disco e envelopa em um Arc (para compartilhamento seguro entre threads)
    let disk_manager = Arc::new(DiskManager::new("/tmp/nas_storage").await?);
    println!("Storage manager pronto em /tmp/nas_storage");

    // 2. Instancia o nosso servidor gRPC passando o ID do nó e o DiskManager
    let rpc_server = RpcServer::new(my_node.id, disk_manager);

    println!("Nó {} escutando gRPC em {}", my_node.id, my_node.address);

    // 3. Inicia o servidor Tonic (ele mantém o programa vivo aguardando conexões)
    Server::builder()
        .add_service(rpc_server.into_service())
        .serve(my_address)
        .await
        .map_err(|e| NasError::Network(e.to_string()))?;

    Ok(())
}