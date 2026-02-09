use sqlx::PgPool;
use rust_decimal::Decimal;

pub async fn obtener_estado(pool: &PgPool, telefono: &str) -> String {
    sqlx::query_scalar!(
        "INSERT INTO user_sessions (telefono_whatsapp) VALUES ($1) 
         ON CONFLICT (telefono_whatsapp) DO UPDATE SET ultima_actualizacion = NOW() 
         RETURNING estado",
        telefono
    ).fetch_one(pool).await.unwrap_or_else(|_| "INICIO".to_string())
}

pub async fn cambiar_estado(pool: &PgPool, telefono: &str, nuevo_estado: &str) {
    let _ = sqlx::query!("UPDATE user_sessions SET estado = $1 WHERE telefono_whatsapp = $2", nuevo_estado, telefono)
        .execute(pool).await;
}

pub async fn obtener_categorias(pool: &PgPool) -> Vec<String> {
    sqlx::query!("SELECT DISTINCT category::text as cat FROM medications WHERE category IS NOT NULL")
        .fetch_all(pool).await
        .map(|rows| rows.into_iter().filter_map(|r| r.cat).collect())
        .unwrap_or_default()
}

pub async fn buscar_productos_categoria(pool: &PgPool, categoria: &str) -> Vec<(String, String, Option<String>, Decimal)> {
    sqlx::query_as::<sqlx::Postgres, (String, String, Option<String>, Decimal)>(
        "SELECT brand_name, active_compound, presentation, price FROM medications 
         WHERE category::text = $1 AND stock = true LIMIT 10"
    ).bind(categoria).fetch_all(pool).await.unwrap_or_default()
}