use sqlx::PgPool;
use crate::{database, whatsapp};
use super::states::UserState;
use uuid::Uuid;

pub async fn enviar_bienvenida(pool: &PgPool, telefono: &str) {
    database::cambiar_estado(pool, telefono, "INICIO").await;

    let mut nombre_saludo = "".to_string();
    if let Some(u) = database::obtener_usuario_por_telefono(pool, telefono).await {
        if !u.first_name.trim().is_empty() {
            nombre_saludo = format!(", {}", u.first_name);
        }
    }

    let mensaje = format!("¡Hola{}! Bienvenido a *Biotecza*.\nSelecciona una opción:", nombre_saludo);
    whatsapp::enviar_botones(telefono, &mensaje, vec!["Laboratorio", "Medicamentos"]).await;
}

pub async fn verificar_y_enviar_bienvenida(pool: &PgPool, telefono: &str) {
    if let Some(paciente) = database::obtener_patient_id_por_telefono(pool, telefono).await {
        let hay_orden_farmacia = database::obtener_orden_farmacia_pendiente(pool, paciente.patient_id).await.is_some();
        let hay_orden_lab = database::obtener_orden_lab_pendiente(pool, paciente.patient_id).await.is_some();

        if hay_orden_farmacia || hay_orden_lab {
            let tipo_pedido = if hay_orden_farmacia { "medicamentos" } else { "análisis de laboratorio" };
            
            let mut nombre_saludo = "".to_string();
            if let Some(u) = database::obtener_usuario_por_telefono(pool, telefono).await {
                if !u.first_name.trim().is_empty() {
                    nombre_saludo = format!(", {}", u.first_name);
                }
            }

            let mensaje = format!("¡Hola{}!\n\nTienes un pedido de {} en proceso. ¿Qué deseas hacer?", nombre_saludo, tipo_pedido);
            database::cambiar_estado(pool, telefono, "INICIO").await;
            whatsapp::enviar_botones(telefono, &mensaje, vec!["Continuar pedido", "Pedido nuevo", "Cancelar"]).await;
            return;
        }
    }
    enviar_bienvenida(pool, telefono).await;
}

pub async fn procesar_usuario(
    pool: &PgPool,
    telefono: &str,
    entrada: &str,
    estado: UserState,
    user_id: &Uuid,
    patient_id: &Uuid,
) -> bool {
    match estado {
        // Este estado se activa si el usuario viene de confirmar un carrito
        UserState::ConfirmandoPedido => {
            if entrada == "Confirmar Pedido" {
                // Aquí podrías pedir el CP antes de lanzar el Flow
                database::cambiar_estado(pool, telefono, "VALIDANDO_CP").await;
                whatsapp::enviar_texto(telefono, "📍 Para verificar la entrega, por favor escribe tu *Código Postal*:").await;
            } else {
                enviar_bienvenida(pool, telefono).await;
            }
            true
        },

        // Aquí recibimos el JSON del Flow parseado
        UserState::EsperandoReceta => { // O el nombre que le hayas dado al estado del Flow
            // Intentamos parsear la entrada (que debería ser un JSON String)
            if let Ok(datos_json) = serde_json::from_str::<serde_json::Value>(entrada) {
                match database::finalizar_pedido_con_datos_flow(pool, patient_id, user_id, &datos_json).await {
                    Ok(_) => {
                        whatsapp::enviar_texto(
                            telefono, 
                            "✅ *¡Pedido registrado!*\n\nHemos guardado tus datos de entrega. Un asesor te contactará pronto. ¡Gracias!"
                        ).await;
                        database::cambiar_estado(pool, telefono, "INICIO").await;
                    },
                    Err(e) => {
                        eprintln!("Error al guardar flow: {:?}", e);
                        whatsapp::enviar_texto(telefono, "❌ Hubo un error al guardar tus datos. Intenta de nuevo.").await;
                    }
                }
            } else {
                whatsapp::enviar_texto(telefono, "⚠️ Error al procesar el formulario. Intenta de nuevo.").await;
            }
            true
        },

        _ => false,
    }
}