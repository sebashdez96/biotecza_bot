mod bot_logic;
mod database;
mod whatsapp;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;
    
    // Conexión centralizada
    let pool = PgPool::connect(&database_url).await?;
    println!("✅ Biotecza DB conectada");

    let app = Router::new()
        .route("/webhook", get(whatsapp::handle_verify_webhook))
        .route("/webhook", post(whatsapp::handle_recibir_mensaje))
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 Servidor Biotecza corriendo en http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}