use sqlx::PgPool;
use crate::bot_logic::models::LabTest;
use uuid::Uuid;

pub async fn obtener_nombres_estudios(pool: &PgPool) -> Vec<String> {
    sqlx::query!("SELECT test_name FROM lab_tests LIMIT 10")
        .fetch_all(pool).await
        .map(|rows| rows.into_iter().map(|r| r.test_name).collect())
        .unwrap_or_default()
}

pub async fn obtener_detalle_estudio(pool: &PgPool, nombre: &str) -> Option<LabTest> {
    sqlx::query_as::<sqlx::Postgres, (String, String, rust_decimal::Decimal)>(
        "SELECT test_name, instructions, price FROM lab_tests WHERE test_name = $1"
    )
    .bind(nombre)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|(test_name, instructions, price)| LabTest {
        test_id: None,
        test_name,
        instructions,
        price,
        category: None,
    })
}

/// Verifica si hay una orden de laboratorio pendiente para un paciente.
/// Retorna Some(order_id) si existe una orden de laboratorio que no esté cancelada, None en caso contrario.
pub async fn obtener_orden_lab_pendiente(pool: &PgPool, patient_id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar!(
        r#"
        SELECT lo.order_id 
        FROM laboratory_orders lo
        JOIN orders o ON lo.order_id = o.order_id
        WHERE o.patient_id = $1 AND lo.current_status::text != 'cancelado'
        LIMIT 1
        "#,
        patient_id
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// Cancela una orden de laboratorio pendiente para un paciente.
pub async fn cancelar_pedido_lab(pool: &PgPool, patient_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE laboratory_orders 
        SET current_status = 'cancelado' 
        WHERE order_id IN (
            SELECT order_id FROM orders 
            WHERE patient_id = $1
        ) AND current_status::text != 'cancelado'
        "#,
        patient_id
    )
    .execute(pool)
    .await?;

    Ok(())}