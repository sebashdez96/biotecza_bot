mod database;
mod whatsapp;
mod bot_logic;

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::net::SocketAddr;

#[derive(Deserialize)]
struct VerifyQuery {
    #[serde(rename = "hub.mode")]
    mode: String,
    #[serde(rename = "hub.verify_token")]
    verify_token: String,
    #[serde(rename = "hub.challenge")]
    challenge: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;
    
    // Conexión centralizada
    let pool = PgPool::connect(&database_url).await?;
    println!("✅ Biotecza DB conectada");

    let app = Router::new()
        .route("/webhook", get(verificar_webhook))
        .route("/webhook", post(recibir_mensaje))
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 Servidor Biotecza corriendo en http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// --- MANEJADORES ---

async fn verificar_webhook(Query(params): Query<VerifyQuery>) -> String {
    let token_esperado = std::env::var("VERIFY_TOKEN").unwrap_or_default();
    if params.mode == "subscribe" && params.verify_token == token_esperado {
        return params.challenge;
    }
    "Token inválido".to_string()
}

async fn recibir_mensaje(
    State(pool): State<PgPool>,
    Json(payload): Json<serde_json::Value>,
) -> String {
    // 1. Extraer el mensaje del JSON gigante de Meta
    if let Some(msg) = payload["entry"][0]["changes"][0]["value"]["messages"][0].as_object() {
        let telefono = msg["from"].as_str().unwrap_or("");
        
        // 2. Limpieza de número (El famoso "1" de México)
        let mut tel_limpio = telefono.to_string();
        if tel_limpio.starts_with("521") {
            tel_limpio = format!("52{}", &tel_limpio[3..]);
        }

        // 3. Obtener el texto (ya sea que escribió, picó un botón o eligió de una lista)
        let texto_usuario = extraer_texto(msg);

        if !tel_limpio.is_empty() {
            // 4. Delegar todo al cerebro del bot
            bot_logic::procesar(&pool, &tel_limpio, &texto_usuario).await;
        }
    }

    "EVENT_RECEIVED".to_string()
}

// --- HELPER DE EXTRACCIÓN ---

fn extraer_texto(msg: &serde_json::Map<String, serde_json::Value>) -> String {
    // ¿Es texto simple?
    if let Some(t) = msg.get("text") {
        return t["body"].as_str().unwrap_or("").to_string();
    }
    
    // ¿Es una interacción (botón o lista)?
    if let Some(i) = msg.get("interactive") {
        // Caso: Botón normal (Max 3)
        if let Some(b) = i.get("button_reply") {
            return b["title"].as_str().unwrap_or("").to_string();
        }
        // Caso: List Message (Categorías)
        if let Some(l) = i.get("list_reply") {
            return l["title"].as_str().unwrap_or("").to_string();
        }
    }
    
    String::new()
}