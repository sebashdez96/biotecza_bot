use sqlx::PgPool;
use crate::whatsapp;
use super::states::UserState;
use regex::Regex;

pub async fn enviar_bienvenida(pool: &PgPool, telefono: &str) {
    crate::database::cambiar_estado(pool, telefono, &UserState::Inicio.to_string()).await;

    // Intentar obtener usuario y personalizar saludo si ya tiene nombre
    if let Some(u) = crate::database::obtener_usuario_por_telefono(pool, telefono).await {
        if !u.first_name.trim().is_empty() {
            let mensaje = format!("¡Hola, {}! Bienvenido a *Biotecza*.\nSelecciona una opción:", u.first_name);
            whatsapp::enviar_botones(telefono, &mensaje, vec!["Laboratorio", "Medicamentos"]).await;
            return;
        }
    }

    whatsapp::enviar_botones(
        telefono,
        "¡Hola! Bienvenido a *Biotecza*.\nSelecciona una opción:",
        vec!["Laboratorio", "Medicamentos"]
    ).await;
}

pub async fn procesar_usuario(
    pool: &PgPool,
    telefono: &str,
    entrada: &str,
    estado: UserState,
    user_id: &uuid::Uuid,
    patient_id: &uuid::Uuid,
) -> bool {
    match estado {
        UserState::ConfirmandoPedido => {
            if entrada == "Confirmar Pedido" {
                crate::database::cambiar_estado(pool, telefono, &UserState::EsperandoPrimerNombre.to_string()).await;
                whatsapp::enviar_texto(telefono, "¡Excelente! ¿Cuál es tu *nombre*?").await;
            } else {
                enviar_bienvenida(pool, telefono).await;
            }
            true
        },

        UserState::EsperandoPrimerNombre => {
            // Guardar el nombre (first_name)
            let _ = sqlx::query!("UPDATE users SET first_name = $1 WHERE user_id = $2", entrada, user_id)
                .execute(pool).await;

            crate::database::cambiar_estado(pool, telefono, &UserState::EsperandoApellidoPaterno.to_string()).await;
            whatsapp::enviar_texto(telefono, "Gracias. ¿Cuál es tu *apellido paterno*?").await;
            true
        },

        UserState::EsperandoApellidoPaterno => {
            // Guardar apellido paterno en paternal_last_name
            let _ = sqlx::query!("UPDATE users SET paternal_last_name = $1 WHERE user_id = $2", entrada, user_id)
                .execute(pool).await;

            crate::database::cambiar_estado(pool, telefono, &UserState::EsperandoApellidoMaterno.to_string()).await;
            whatsapp::enviar_texto(telefono, "Ahora, ¿cuál es tu *apellido materno*? (o responde '-' si no aplica)").await;
            true
        },

        UserState::EsperandoApellidoMaterno => {
            // Si no aplica, se puede enviar '-' para omitir
            if entrada.trim() != "-" {
                // Guardar apellido materno en maternal_last_name
                let _ = sqlx::query!("UPDATE users SET maternal_last_name = $1 WHERE user_id = $2", entrada, user_id)
                    .execute(pool).await;
            }

            crate::database::cambiar_estado(pool, telefono, &UserState::EsperandoEmail.to_string()).await;
            // Obtener first_name para el saludo
            if let Some(u) = crate::database::obtener_usuario_por_telefono(pool, telefono).await {
                whatsapp::enviar_texto(telefono, &format!("Mucho gusto, {}. ¿Cuál es tu *correo*?", u.first_name)).await;
            } else {
                whatsapp::enviar_texto(telefono, "¿Cuál es tu *correo*?").await;
            }
            true
        },

        UserState::EsperandoEmail => {
            let email = entrada.trim();
            // Expresión regular simple para validar formato básico de email
            let re = Regex::new(r"(?i)^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
            if re.is_match(email) {
                crate::database::actualizar_email_usuario(pool, *user_id, email).await;
                crate::database::cambiar_estado(pool, telefono, &UserState::EsperandoCurp.to_string()).await;
                whatsapp::enviar_texto(telefono, "Gracias. Ahora ingresa tu *CURP* (18 caracteres):").await;
            } else {
                whatsapp::enviar_texto(telefono, "❌ Formato de correo inválido. Por favor ingresa un correo válido:").await;
            }
            true
        },

        UserState::EsperandoCurp => {
            if entrada.len() == 18 {
                sqlx::query!("UPDATE patients SET curp = $1 WHERE patient_id = $2", entrada, patient_id).execute(pool).await.ok();
                crate::database::cambiar_estado(pool, telefono, &UserState::EsperandoGenero.to_string()).await;
                whatsapp::enviar_botones(telefono, "¿Cuál es tu género?", vec!["M", "F"]).await;
            } else {
                whatsapp::enviar_texto(telefono, "❌ CURP inválido. Inténtalo de nuevo:").await;
            }
            true
        },

        UserState::EsperandoGenero => {
            sqlx::query!("UPDATE patients SET gender = $1 WHERE patient_id = $2", entrada, patient_id).execute(pool).await.ok();
            crate::database::cambiar_estado(pool, telefono, &UserState::EsperandoDireccion.to_string()).await;
            whatsapp::enviar_texto(telefono, "📍 ¿Cuál es la *dirección completa*?").await;
            true
        },

        UserState::EsperandoDireccion => {
            crate::database::guardar_direccion_paciente(pool, *patient_id, entrada).await;
            crate::database::cambiar_estado(pool, telefono, &UserState::EsperandoReceta.to_string()).await;
            whatsapp::enviar_texto(telefono, "✅ ¡Listo! Ahora envía la *foto de tu receta médica*.").await;
            true
        },

        _ => false,
    }
}
