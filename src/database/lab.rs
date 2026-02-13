use sqlx::PgPool;
use crate::bot_logic::models::LabTest;

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
