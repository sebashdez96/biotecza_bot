// Módulos
pub mod users;
pub mod pharmacy;
pub mod lab;
pub mod states;
pub mod models;

// Re-exportar funciones principales
pub use users::{enviar_bienvenida, procesar_usuario, verificar_y_enviar_bienvenida};
pub use pharmacy::{procesar_farmacia,generar_ticket_con_envio};
pub use lab::procesar_lab;
pub use states::UserState;

use sqlx::PgPool;
use crate::database;
use std::str::FromStr;

pub async fn procesar(pool: &PgPool, telefono: &str, entrada: &str) {
    let estado_str: String = database::obtener_estado(pool, telefono).await;
    let estado = UserState::from_str(&estado_str).unwrap_or(UserState::Inicio);
    println!("🤖 Usuario: {} | Estado: {:?} | Entrada: {}", telefono, estado, entrada);

    // 1. Identificación de IDs (Scope superior)
    let user_id = match database::obtener_usuario_por_telefono(pool, telefono).await {
        Some(user) => user.user_id,
        None => {
            let patient = database::registrar_paciente_completo(pool, telefono, "").await;
            patient.user_id
        }
    };

    let paciente = match database::obtener_patient_id_por_telefono(pool, telefono).await {
        Some(p) => p,
        None => database::registrar_paciente_completo(pool, telefono, "").await,
    };
    let patient_id = paciente.patient_id;

    // 2. Comandos Globales
    if entrada.to_lowercase() == "hola" || entrada.to_lowercase() == "inicio" {
        verificar_y_enviar_bienvenida(pool, telefono).await;
        return;
    }

    // 3. Máquina de Estados Principal - Delegar según estado
    match estado {
        UserState::Inicio => {
            match entrada {
                "Medicamentos" => {
                    database::cambiar_estado(pool, telefono, &UserState::EsperandoBusqueda.to_string()).await;
                    crate::whatsapp::enviar_texto(
                    telefono, 
                     "🔍 *Buscador de Medicamentos*\n\nEscribe el nombre, marca o compuesto activo para buscar.\n_Ejemplo: Sertralina 50mg_"
            ).await;                },
                "Laboratorio" => {
                    let estudios = database::obtener_nombres_estudios(pool).await;
                    database::cambiar_estado(pool, telefono, &UserState::SeleccionandoExamen.to_string()).await;
                    crate::whatsapp::enviar_lista(telefono, "🔬 Estudios", "Selecciona un análisis:", "Ver Estudios", estudios).await;
                    },
"Continuar pedido" => {
    // Si la función devuelve Option<Uuid>, 'id_orden' ya es el Uuid que necesitas
    if let Some(id_orden) = database::obtener_orden_farmacia_pendiente(pool, patient_id).await {
        
        database::cambiar_estado(pool, telefono, "CONFIRMANDO_TICKET").await;
        
        // Pasamos id_orden directamente porque ya es el Uuid
        let ticket = pharmacy::generar_ticket_con_envio(pool, id_orden).await;
        
        let msg = format!("👋 ¡Hola de nuevo! Detectamos que tenías un pedido pendiente:\n\n{}", ticket);
        
        crate::whatsapp::enviar_botones(
            telefono, 
            &msg, 
            vec!["Sí, todo correcto", "Agregar más", "Cancelar Pedido"]
        ).await;

    } else if let Some(id_orden_lab) = database::obtener_orden_lab_pendiente(pool, patient_id).await {
        // ... lógica de laboratorio ...
    }
},
                "Pedido nuevo" => {
                    // Cancelar el pedido anterior y empezar uno nuevo
                    let _ = database::cancelar_pedido_farmacia(pool, patient_id).await;
                    let _ = database::cancelar_pedido_lab(pool, patient_id).await;
                    enviar_bienvenida(pool, telefono).await;
                },
                "Cancelar" => {
                    // Cancelar el pedido anterior
                    let _ = database::cancelar_pedido_farmacia(pool, patient_id).await;
                    let _ = database::cancelar_pedido_lab(pool, patient_id).await;
                    enviar_bienvenida(pool, telefono).await;
                },
                _ => {
                    enviar_bienvenida(pool, telefono).await;
                }
            }
        },

        // Delegar a lab
        UserState::SeleccionandoExamen => {
            let _ = procesar_lab(pool, telefono, entrada, estado).await;
        },

        // Delegar a pharmacy
        UserState::MenuFarmacia | 
        UserState::EsperandoCategoria | 
        UserState::AgregandoProducto | 
        UserState::EsperandoBusqueda | 
        UserState::ConfirmandoSeleccion |     // <--- AGREGAR ESTE
        UserState::AgregandoProductoFinal => {
            let _ = procesar_farmacia(pool, telefono, entrada, estado, &patient_id).await;
        },

        // Delegar a users
        UserState::ConfirmandoPedido | 
        UserState::ConfirmandoTicket |
        UserState::ValidandoCp => { // <-- ASEGÚRATE DE QUE ESTE ESTÉ AQUÍ
            pharmacy::procesar_farmacia(pool, &telefono, &entrada, estado, &patient_id).await;
        }, 

        UserState::EsperandoNombreCompleto |
        UserState::EsperandoCalle |
        UserState::EsperandoDatosFlow => {
            let _ = procesar_usuario(pool, telefono, entrada, estado, &user_id, &patient_id).await;
        },

        _ => { enviar_bienvenida(pool, telefono).await; }
    }
}
