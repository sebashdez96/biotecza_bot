use sqlx::PgPool;
use uuid::Uuid;
use crate::bot_logic::models::{User, Patient};

/// Contexto ligero para consultas relacionadas con un teléfono de usuario.
/// Mantiene en cache el `user_id` para evitar consultas repetidas a la DB.
pub struct UserContext<'a> {
    pub pool: &'a PgPool,
    pub phone: String,
    pub user_id: Option<Uuid>,
}

impl<'a> UserContext<'a> {
    pub fn new(pool: &'a PgPool, phone: &str) -> Self {
        UserContext { pool, phone: phone.to_string(), user_id: None }
    }

    /// Devuelve el `user_id` cacheado o lo consulta en la DB si no existe.
    pub async fn user_id(&mut self) -> Option<Uuid> {
        if let Some(id) = self.user_id {
            return Some(id);
        }
        if let Some(user) = obtener_usuario_por_telefono(self.pool, &self.phone).await {
            self.user_id = Some(user.user_id);
            return Some(user.user_id);
        }
        None
    }

    /// Devuelve el `user_id` existente o crea un usuario básico si no existe.
    /// `nombre` se usa solo si es necesario crear el usuario.
    pub async fn get_or_create_user_id(&mut self, nombre: &str) -> Uuid {
        if let Some(id) = self.user_id {
            return id;
        }
        if let Some(user) = obtener_usuario_por_telefono(self.pool, &self.phone).await {
            self.user_id = Some(user.user_id);
            return user.user_id;
        }
        let id = registrar_usuario_basico(self.pool, &self.phone, nombre).await;
        self.user_id = Some(id);
        id
    }
}

pub async fn obtener_estado(pool: &PgPool, telefono: &str) -> String {
    let resultado = sqlx::query_scalar!("SELECT fn_obtener_o_crear_estado($1)", telefono)
        .fetch_one(pool)
        .await;

    match resultado {
        Ok(Some(estado)) => estado,
        _ => "INICIO".to_string(), // Si hay error o es NULL, vuelve al inicio
    }
}

pub async fn cambiar_estado(pool: &PgPool, telefono: &str, nuevo_estado: &str) {
    // Solo llamamos a la función. No nos importa el resultado (VOID)
    let _ = sqlx::query!("SELECT fn_actualizar_estado_sesion($1, $2)", telefono, nuevo_estado)
        .execute(pool)
        .await;
}

pub async fn obtener_usuario_por_telefono(pool: &PgPool, telefono: &str) -> Option<User> {
    // Usamos query_as! para que el resultado entre directo al Struct User
    sqlx::query_as!(
        User,
        r#"
        SELECT 
            user_id as "user_id!", 
            COALESCE(first_name, '') as "first_name!", 
            COALESCE(paternal_last_name, '') as "paternal_last_name!",
            COALESCE(maternal_last_name, '') as "maternal_last_name!",
            COALESCE(email, '') as "email!", 
            COALESCE(phone, '') as "phone!"
        FROM view_usuarios_full 
        WHERE phone = $1
        "#,
        telefono
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

#[allow(dead_code)]
pub async fn registrar_usuario_basico(pool: &PgPool, telefono: &str, nombre: &str) -> Uuid {
    // Intentamos insertar y, si el teléfono ya existe, simplemente devolvemos el user_id existente.
    // NOTA: Para que esto funcione, la columna 'phone' debe tener un índice UNIQUE en la DB.
    let res = sqlx::query_scalar!(
        r#"
        INSERT INTO users (first_name, email, password_hash, phone, role_id) 
        VALUES (NULLIF($1, ''), $2, 'whatsapp_user', $3, 5) 
        ON CONFLICT (phone) DO UPDATE SET phone = EXCLUDED.phone
        RETURNING user_id
        "#,
        nombre, format!("{}@biotecza.com", telefono), telefono
    )
    .fetch_one(pool)
    .await;

    match res {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error crítico al registrar/recuperar usuario: {:?}", e);
            panic!("No se pudo asegurar la existencia del usuario para el teléfono {}", telefono);
        }
    }
}
pub async fn obtener_patient_id_por_telefono(pool: &PgPool, telefono: &str) -> Option<Patient> {
    sqlx::query!(
        "SELECT p.patient_id, p.user_id, u.first_name, u.paternal_last_name, u.maternal_last_name, p.curp, p.whatsapp_number, p.gender FROM patients p JOIN users u ON p.user_id = u.user_id WHERE p.whatsapp_number = $1",
        telefono
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|row| Patient {
        patient_id: row.patient_id,
        user_id: row.user_id.expect("El usuario ID debe existir para el paciente"),
        curp: Some(row.curp),
        whatsapp_number: row.whatsapp_number,
        gender: row.gender.and_then(|g| g.chars().next()),
    })
}

pub async fn registrar_paciente_completo(pool: &PgPool, telefono: &str, nombre: &str) -> Patient {
    let mut ctx = UserContext::new(pool, telefono);
    registrar_paciente_completo_con_context(&mut ctx, nombre).await
}

/// Variante que usa `UserContext` para evitar consultas repetidas del `user_id`.
/// Ejemplo de uso:
///
/// let mut ctx = UserContext::new(&pool, telefono);
/// let patient = registrar_paciente_completo_con_context(&mut ctx, "Nombre").await;
pub async fn registrar_paciente_completo_con_context(ctx: &mut UserContext<'_>, nombre: &str) -> Patient {
    // 1. Obtenemos el user_id (esta función ya es segura y maneja el ON CONFLICT en users)
    let real_user_id = ctx.get_or_create_user_id(nombre).await;

    // 2. Usamos ON CONFLICT para la tabla patients.
    // Si el whatsapp_number ya existe, no hace nada nuevo pero nos retorna el patient_id existente.
    let patient_id = sqlx::query_scalar!(
        r#"
        INSERT INTO patients (user_id, curp, whatsapp_number) 
        VALUES ($1, $2, $3) 
        ON CONFLICT (whatsapp_number) 
        DO UPDATE SET user_id = EXCLUDED.user_id -- Truco para forzar el RETURNING
        RETURNING patient_id
        "#,
        real_user_id, 
        format!("TEMP-{}", ctx.phone), 
        ctx.phone
    )
    .fetch_one(ctx.pool)
    .await
    .expect("Error crítico: No se pudo crear ni recuperar el paciente");

    Patient {
        patient_id,
        user_id: real_user_id,
        curp: Some(format!("TEMP-{}", ctx.phone)),
        whatsapp_number: ctx.phone.clone(),
        gender: None,
    }
}

pub async fn actualizar_datos_usuario(pool: &PgPool, user_id: Uuid, first: &str, paternal: &str, maternal: &str) {
    let _ = sqlx::query!(
        "UPDATE users SET first_name = $1, paternal_last_name = $2, maternal_last_name = $3 WHERE user_id = $4",
        first, paternal, maternal, user_id
    ).execute(pool).await;
}

pub async fn actualizar_email_usuario(pool: &PgPool, user_id: Uuid, email: &str) {
    let _ = sqlx::query!(
        "UPDATE users SET email = $1 WHERE user_id = $2",
        email, user_id
    ).execute(pool).await;
}

#[allow(dead_code)]
pub async fn actualizar_datos_clinicos(pool: &PgPool, patient_id: Uuid, curp: &str, genero: &str) {
    let _ = sqlx::query!(
        "UPDATE patients SET curp = $1, gender = $2 WHERE patient_id = $3",
        curp, genero, patient_id
    ).execute(pool).await;
}

pub async fn guardar_direccion_paciente(pool: &PgPool, patient_id: Uuid, direccion: &str) {
    let _ = sqlx::query!(
        "INSERT INTO patient_addresses (patient_id, address_label, full_address, is_default) 
         VALUES ($1, 'WhatsApp Delivery', $2, true)
         ON CONFLICT DO NOTHING",
        patient_id, direccion
    ).execute(pool).await;

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

#[allow(dead_code)]
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
}
