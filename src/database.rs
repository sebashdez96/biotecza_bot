use sqlx::PgPool;
use rust_decimal::Decimal;
use uuid::Uuid;

// Estructura auxiliar para manejar el detalle del medicamento
pub struct MedDetalle {
    pub id: Uuid,
    pub price: Decimal,
}

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


pub async fn obtener_nombres_estudios(pool: &PgPool) -> Vec<String> {
    sqlx::query!("SELECT test_name FROM lab_tests LIMIT 10")
        .fetch_all(pool).await
        .map(|rows| rows.into_iter().map(|r| r.test_name).collect())
        .unwrap_or_default()
}

// 2. Obtener el detalle completo de uno solo
pub async fn obtener_detalle_estudio(pool: &PgPool, nombre: &str) -> Option<(String, String, rust_decimal::Decimal)> {
    sqlx::query_as::<sqlx::Postgres, (String, String, rust_decimal::Decimal)>(
        "SELECT test_name, instructions, price FROM lab_tests WHERE test_name = $1"
    )
    .bind(nombre)
    .fetch_optional(pool).await.ok().flatten()
}


// registrar usuario

pub async fn obtener_usuario_por_telefono(pool: &PgPool, telefono: &str) -> Option<Uuid> {
    // Buscamos el user_id usando el teléfono de WhatsApp
    sqlx::query_scalar!("SELECT user_id FROM users WHERE phone = $1", telefono)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
}

pub async fn registrar_usuario_basico(pool: &PgPool, telefono: &str, nombre: &str) -> Uuid {
    // Si es nuevo, lo registramos con datos mínimos
    sqlx::query_scalar!(
        "INSERT INTO users (first_name, last_name, email, password_hash, phone, role_id) 
         VALUES ($1, '', $2, 'whatsapp_user', $3, 5) 
         RETURNING user_id",
        nombre, format!("{}@biotecza.com", telefono), telefono
    )
    .fetch_one(pool)
    .await
    .unwrap_or_else(|_| Uuid::new_v4())
}


//ORDERS

// 1. Buscar al usuario por su teléfono de WhatsApp
pub async fn obtener_patient_id_por_telefono(pool: &PgPool, telefono: &str) -> Option<Uuid> {
    // Buscamos directamente en la tabla de pacientes por el número de whatsapp
    sqlx::query_scalar!(
        "SELECT patient_id FROM patients WHERE whatsapp_number = $1",
        telefono
    ).fetch_optional(pool).await.unwrap_or(None)
}
// 3. Añadir el medicamento al carrito (medication_items)
pub async fn agregar_al_carrito(pool: &PgPool, order_id: Uuid, med_id: Uuid, precio: Decimal) {
    // Insertamos el item
    let _ = sqlx::query!(
        "INSERT INTO medication_items (order_id, med_id, quantity, unit_price) 
         VALUES ($1, $2, 1, $3)",
        order_id, med_id, precio
    ).execute(pool).await;

    // Actualizamos el total de la orden principal
    let _ = sqlx::query!(
        "UPDATE orders SET total_amount = total_amount + $1 WHERE order_id = $2",
        precio, order_id
    ).execute(pool).await;
}

// 1. Obtiene solo los nombres para llenar el menú de lista de WhatsApp
pub async fn obtener_productos_nombres_y_ids(pool: &PgPool, categoria: &str) -> Vec<String> {
    sqlx::query!("SELECT brand_name FROM medications WHERE category::text = $1 AND stock = true", categoria)
        .fetch_all(pool).await
        .map(|rows| rows.into_iter().map(|r| r.brand_name).collect())
        .unwrap_or_default()
}

// 2. Obtiene el ID y el precio real para poder hacer el insert en 'medication_items'
pub async fn obtener_detalle_med_por_nombre(pool: &PgPool, nombre: &str) -> Option<MedDetalle> {
    sqlx::query_as!(
        MedDetalle,
        "SELECT med_id as id, price FROM medications WHERE brand_name = $1 LIMIT 1",
        nombre
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
}


// src/database.rs

pub async fn registrar_paciente_completo(pool: &PgPool, telefono: &str, nombre: &str) -> Uuid {
    let mut tx = pool.begin().await.unwrap();

    let user_id = sqlx::query_scalar!(
        "INSERT INTO users (first_name, last_name, email, password_hash, phone, role_id) 
         VALUES ($1, '', $2, 'pass', $3, 5) RETURNING user_id",
        nombre, format!("{}@biotecza.com", telefono), telefono
    ).fetch_one(&mut *tx).await.unwrap();

    let _ = sqlx::query!(
        "INSERT INTO patients (user_id, curp, whatsapp_number, email) 
         VALUES ($1, $2, $3, $4)",
        user_id, format!("TEMP-{}", telefono), telefono, format!("{}@biotecza.com", telefono)
    ).execute(&mut *tx).await.unwrap();

    tx.commit().await.unwrap();
    user_id // Retornamos el ID para usarlo en el scope de procesar
}

// Actualiza obtener_o_crear_orden para que use el ID correcto
pub async fn obtener_o_crear_orden(pool: &PgPool, patient_id: Uuid) -> Uuid {
    let orden = sqlx::query_scalar!(
        "SELECT order_id FROM orders WHERE patient_id = $1 AND p_status = 'pendiente' LIMIT 1",
        patient_id
    ).fetch_optional(pool).await.unwrap_or(None);

    if let Some(id) = orden { id } 
    else {
        // Creamos la orden vinculada al paciente
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
    // Unimos medication_items con medications para sacar el nombre (brand_name)
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

pub async fn guardar_direccion_paciente(pool: &PgPool, patient_id: Uuid, direccion: &str) {
    // 1. Insertamos en la tabla de direcciones
    // Usamos 'WhatsApp' como etiqueta predeterminada
    let _ = sqlx::query!(
        "INSERT INTO patient_addresses (patient_id, address_label, full_address, is_default) 
         VALUES ($1, 'WhatsApp Delivery', $2, true)
         ON CONFLICT DO NOTHING",
        patient_id, direccion
    ).execute(pool).await;

    // 2. También actualizamos la columna delivery_address en medication_orders 
    // para que la orden actual tenga la dirección de entrega específica.
    let _ = sqlx::query!(
        "UPDATE medication_orders 
         SET delivery_address = $1 
         FROM orders 
         WHERE medication_orders.order_id = orders.order_id 
         AND orders.patient_id = $2 
         AND orders.p_status = 'pendiente'",
        direccion, patient_id
    ).execute(pool).await;
}


pub async fn actualizar_datos_usuario(pool: &PgPool, user_id: Uuid, first: &str, last: &str) {
    let _ = sqlx::query!(
        "UPDATE users SET first_name = $1, last_name = $2 WHERE user_id = $3",
        first, last, user_id
    ).execute(pool).await;
}

pub async fn actualizar_email_usuario(pool: &PgPool, user_id: Uuid, email: &str) {
    let _ = sqlx::query!(
        "UPDATE users SET email = $1 WHERE user_id = $2",
        email, user_id
    ).execute(pool).await;
}

pub async fn actualizar_datos_clinicos(pool: &PgPool, patient_id: Uuid, curp: &str, genero: &str) {
    // genero debe ser 'M' o 'F' según tu tabla gender character(1)
    let _ = sqlx::query!(
        "UPDATE patients SET curp = $1, gender = $2 WHERE patient_id = $3",
        curp, genero, patient_id
    ).execute(pool).await;
}

pub async fn guardar_receta_orden(pool: &PgPool, patient_id: Uuid, media_id: &str) {
    let _ = sqlx::query!(
        "UPDATE medication_orders 
         SET prescription_url = $1 
         FROM orders 
         WHERE medication_orders.order_id = orders.order_id 
         AND orders.patient_id = $2 
         AND orders.p_status = 'pendiente'",
        media_id, patient_id
    ).execute(pool).await;

    // Aprovechamos para marcar la orden como 'en revisión' o algo similar si quieres
}