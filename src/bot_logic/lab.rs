use sqlx::PgPool;
use crate::{database, whatsapp};
use super::states::UserState;

pub async fn procesar_lab(
    pool: &PgPool,
    telefono: &str,
    entrada: &str,
    estado: UserState,
) -> bool {
    match estado {
        UserState::SeleccionandoExamen => {
            if let Some(estudio) = database::obtener_detalle_estudio(pool, entrada).await {
                let mensaje = format!(
                    "✅ *Información del Estudio*\n━━━━━━━━━━━━━━━\n\n🧪 *{}*\n📝 *Instrucciones:* {}\n💰 *Precio:* ${}\n\n¿Deseas consultar otro estudio?",
                    estudio.test_name.to_uppercase(), estudio.instructions, estudio.price
                );
                whatsapp::enviar_texto(telefono, &mensaje).await;
                
                let estudios = database::obtener_nombres_estudios(pool).await;
                whatsapp::enviar_lista(telefono, "🔬 Otros Estudios", "Selecciona otro:", "Ver Estudios", estudios).await;
                whatsapp::enviar_botones(telefono, "O vuelve al inicio:", vec!["Regresar"]).await;
            } else if entrada == "Regresar" {
                super::users::enviar_bienvenida(pool, telefono).await;
            }
            true
        },

        _ => false,
    }
}
