use sqlx::PgPool;
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::bot_logic::models::Medication;

pub async fn obtener_categorias(pool: &PgPool) -> Vec<String> {
    sqlx::query!("SELECT DISTINCT category::text as cat FROM medications WHERE category IS NOT NULL")
        .fetch_all(pool).await
        .map(|rows| rows.into_iter().filter_map(|r| r.cat).collect())
        .unwrap_or_default()
}

#[allow(dead_code)]
pub async fn buscar_productos_categoria(pool: &PgPool, categoria: &str) -> Vec<(String, String, Option<String>, Decimal)> {
    sqlx::query_as::<sqlx::Postgres, (String, String, Option<String>, Decimal)>(
        "SELECT brand_name, active_compound, presentation, price FROM medications 
         WHERE category::text = $1 AND stock = true LIMIT 10"
    ).bind(categoria).fetch_all(pool).await.unwrap_or_default()
}

pub async fn obtener_productos_nombres_y_ids(pool: &PgPool, categoria: &str) -> Vec<String> {
    sqlx::query!("SELECT brand_name FROM medications WHERE category::text = $1 AND stock = true", categoria)
        .fetch_all(pool).await
        .map(|rows| rows.into_iter().map(|r| r.brand_name).collect())
        .unwrap_or_default()
}

pub async fn obtener_detalle_med_por_nombre(pool: &PgPool, nombre: &str) -> Option<Medication> {
    sqlx::query!(
        "SELECT med_id, brand_name, active_compound, presentation, price, stock FROM medications WHERE brand_name = $1 LIMIT 1",
        nombre
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|row| Medication {
        med_id: row.med_id,
        brand_name: row.brand_name,
        active_compound: row.active_compound,
        presentation: row.presentation,
        price: row.price,
        category: None,
        stock: row.stock.unwrap_or(false),
    })
}

pub async fn agregar_al_carrito(pool: &PgPool, order_id: Uuid, med_id: Uuid, precio: Decimal) {
    let _ = sqlx::query!(
        "INSERT INTO medication_items (order_id, med_id, quantity, unit_price) 
         VALUES ($1, $2, 1, $3)",
        order_id, med_id, precio
    ).execute(pool).await;

    let _ = sqlx::query!(
        "UPDATE orders SET total_amount = total_amount + $1 WHERE order_id = $2",
        precio, order_id
    ).execute(pool).await;
}

pub async fn obtener_o_crear_orden(pool: &PgPool, patient_id: Uuid) -> Uuid {
    let orden = sqlx::query_scalar!(
        "SELECT order_id FROM orders WHERE patient_id = $1 AND p_status = 'pendiente' LIMIT 1",
        patient_id
    ).fetch_optional(pool).await.unwrap_or(None);

    if let Some(id) = orden { 
        id 
    } else {
        let new_id = sqlx::query_scalar!(
            "INSERT INTO orders (patient_id, order_type, total_amount, p_method) 
             VALUES ($1, 'medication', 0.00, 'efectivo') RETURNING order_id",
            patient_id
        ).fetch_one(pool).await.unwrap();
        
        let _ = sqlx::query!("INSERT INTO medication_orders (order_id, delivery_address) VALUES ($1, 'Por definir')", new_id)
            .execute(pool).await;
        new_id
    }
}

pub async fn obtener_resumen_carrito(pool: &PgPool, order_id: Uuid) -> Vec<(String, i32, Decimal)> {
    sqlx::query_as::<sqlx::Postgres, (String, i32, Decimal)>(
        "SELECT m.brand_name, mi.quantity, mi.unit_price 
         FROM medication_items mi
         JOIN medications m ON mi.med_id = m.med_id
         WHERE mi.order_id = $1"
    )
    .bind(order_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Busca medicamentos por nombre de patente o compuesto activo.
/// Devuelve hasta 10 resultados con (brand_name, active_compound, presentation, price).
pub async fn buscar_medicamentos_similares(pool: &PgPool, query: &str) -> Vec<(String, String, Option<String>, Decimal)> {
    // Definimos un umbral de similitud (0.0 a 1.0). 0.3 es el estándar de Postgres.
    // Cuanto más bajo, más "tolerante" a errores, pero menos preciso.
    
    sqlx::query_as::<sqlx::Postgres, (String, String, Option<String>, Decimal)>(
        r#"
        SELECT brand_name, active_compound, presentation, price 
        FROM medications 
        WHERE stock = true 
          AND (
               brand_name % $1 
               OR active_compound % $1 
               OR brand_name ILIKE '%' || $1 || '%'
          )
        ORDER BY 
            similarity(brand_name, $1) DESC, 
            brand_name ASC
        LIMIT 10
        "#
    )
    .bind(query)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}