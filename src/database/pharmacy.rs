use sqlx::PgPool;
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::bot_logic::models::Medication;


pub struct InfoPostal {
    pub municipio: String,
    pub estado: String,
    pub colonias: Vec<serde_json::Value>, // Para el data-source del Flow
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
    // Buscamos una orden donde la parte de farmacia esté EXACTAMENTE en 'solicitado'
    // Si está en 'preparando', 'en_ruta' o 'cancelado', esta consulta devolverá None.
    let orden = sqlx::query_scalar!(
        r#"
        SELECT o.order_id 
        FROM orders o
        JOIN medication_orders mo ON o.order_id = mo.order_id
        WHERE o.patient_id = $1 
          AND o.p_status = 'pendiente' 
          AND mo.current_status = 'solicitado'::med_status
        LIMIT 1
        "#,
        patient_id
    ).fetch_optional(pool).await.unwrap_or(None);

    if let Some(id) = orden { 
        // Si encontramos una en 'solicitado', la reutilizamos
        id 
    } else {
        // Si no hay ninguna en 'solicitado', creamos una NUEVA orden
        // Esto garantiza que si el usuario ya tiene una 'en_ruta', 
        // lo que pida ahora sea un pedido separado.
        let new_id = sqlx::query_scalar!(
            "INSERT INTO orders (patient_id, order_type, total_amount, p_method, p_status) 
             VALUES ($1, 'medication', 0.00, 'efectivo', 'pendiente') RETURNING order_id",
            patient_id
        ).fetch_one(pool).await.unwrap();
        
        let _ = sqlx::query!(
            "INSERT INTO medication_orders (order_id, delivery_address, current_status) 
             VALUES ($1, 'Por definir', 'solicitado')", 
            new_id
        ).execute(pool).await;

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

pub async fn cancelar_pedido_farmacia(pool: &PgPool, patient_id: Uuid) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 1. Obtenemos el ID de la orden ANTES de cambiar nada para asegurar el tiro
    let order_id = sqlx::query_scalar!(
        r#"
        SELECT o.order_id 
        FROM orders o
        JOIN medication_orders mo ON o.order_id = mo.order_id
        WHERE o.patient_id = $1 
          AND o.p_status = 'pendiente' 
          AND mo.current_status = 'solicitado'::med_status
        LIMIT 1
        "#,
        patient_id
    ).fetch_optional(&mut *tx).await?;

    if let Some(oid) = order_id {
        // 2. Borramos los items primero (para evitar conflictos de FK si existieran)
        sqlx::query!(
            "DELETE FROM medication_items WHERE order_id = $1",
            oid
        )
        .execute(&mut *tx)
        .await?;

        // 3. Actualizamos la orden de farmacia a cancelado
        sqlx::query!(
            "UPDATE medication_orders SET current_status = 'cancelado' WHERE order_id = $1",
            oid
        )
        .execute(&mut *tx)
        .await?;
        
        // 4. Resetear el total de la orden principal a 0
        sqlx::query!(
            "UPDATE orders SET total_amount = 0 WHERE order_id = $1",
            oid
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}


/// Verifica si hay una orden de medicamentos pendiente para un paciente.
/// Retorna Some(order_id) si existe una orden de medicamentos que no esté cancelada, None en caso contrario.
pub async fn obtener_orden_farmacia_pendiente(pool: &PgPool, patient_id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar!(
        r#"
        SELECT mo.order_id 
        FROM medication_orders mo
        JOIN orders o ON mo.order_id = o.order_id
        WHERE o.patient_id = $1 AND mo.current_status::text != 'cancelado'
        LIMIT 1
        "#,
        patient_id
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}


/// Guarda el término que el usuario escribió para buscar
pub async fn guardar_ultimo_termino_busqueda(pool: &PgPool, telefono: &str, termino: &str) {
    let _ = sqlx::query!(
        "UPDATE user_sessions SET last_search_term = $1, ultima_actualizacion = CURRENT_TIMESTAMP WHERE telefono_whatsapp = $2",
        termino, telefono
    ).execute(pool).await;
}

/// Recupera el término para volver a ejecutar la búsqueda internamente
pub async fn obtener_ultimo_termino_busqueda(pool: &PgPool, telefono: &str) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT last_search_term FROM user_sessions WHERE telefono_whatsapp = $1", 
        telefono
    )
    .fetch_optional(pool).await.ok().flatten().flatten()
}

/// Guarda el nombre del medicamento que el usuario eligió de la lista numérica
pub async fn guardar_producto_seleccionado(pool: &PgPool, telefono: &str, nombre_med: &str) {
    let _ = sqlx::query!(
        "UPDATE user_sessions SET selected_product_name = $1, ultima_actualizacion = CURRENT_TIMESTAMP WHERE telefono_whatsapp = $2",
        nombre_med, telefono
    ).execute(pool).await;
}

/// Recupera el nombre del producto para agregarlo finalmente al carrito
pub async fn obtener_producto_seleccionado(pool: &PgPool, telefono: &str) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT selected_product_name FROM user_sessions WHERE telefono_whatsapp = $1", 
        telefono
    )
    .fetch_optional(pool).await.ok().flatten().flatten()
}

pub async fn aplicar_costo_envio_db(pool: &PgPool, order_id: Uuid) {
    let _ = sqlx::query!(
        "UPDATE orders SET total_amount = total_amount + 20.00 WHERE order_id = $1",
        order_id
    ).execute(pool).await;
}


pub async fn obtener_info_por_cp(pool: &PgPool, cp_usuario: &str) -> Option<InfoPostal> {
    let rows = sqlx::query!(
        r#"
        SELECT colonia, municipio, estado 
        FROM cobertura_postal 
        WHERE cp = $1 AND tiene_cobertura = TRUE
        "#,
        cp_usuario
    )
    .fetch_all(pool)
    .await
    .ok()?;

    if rows.is_empty() {
        return None;
    }

    // Tomamos municipio y estado del primer resultado (son iguales para el mismo CP)
    let municipio = rows[0].municipio.clone();
    let estado = rows[0].estado.clone();

    // Mapeamos las colonias al formato que espera el Flow { "id": "...", "title": "..." }
    let colonias = rows.iter().map(|r| {
        serde_json::json!({
            "id": r.colonia,
            "title": r.colonia
        })
    }).collect();

    Some(InfoPostal {
        municipio,
        estado,
        colonias,
    })
}