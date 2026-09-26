// crates/daemon/src/main.rs
use common::{Node, NodeStatus, Result};
use std::net::SocketAddr;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Iniciando o Node Daemon do NAS Distribuído...");

    // Simulando a criação da identidade deste computador
    let my_address: SocketAddr = "10.0.0.10:9000".parse().unwrap();
    let my_node = Node {
        id: Uuid::new_v4(),
        address: my_address,
        status: NodeStatus::Online,
        free_space_bytes: 500 * 1024 * 1024 * 1024, // 500 GB simulados
    };

    println!("Nó inicializado: ID {} escutando em {}", my_node.id, my_node.address);

    // Aqui entrarão os inicializadores reais de Storage, Network e Metadata
    // network::start_grpc_server(my_address).await?;
    // storage::init_disk_manager().await?;

    // Loop principal para manter o processo rodando
    loop {
        // Futuramente, este loop será substituído pela escuta de eventos do sistema
        sleep(Duration::from_secs(5)).await;
        println!("Status: Nó {} operando normalmente.", my_node.id);
    }
}