// Módulos
pub mod users;
pub mod pharmacy;
pub mod lab;
pub mod states;
pub mod models;

// Re-exportar funciones principales
pub use users::{enviar_bienvenida, procesar_usuario};
pub use pharmacy::procesar_farmacia;
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
        enviar_bienvenida(pool, telefono).await;
        return;
    }

    // 3. Máquina de Estados Principal - Delegar según estado
    match estado {
        UserState::Inicio => {
            if entrada == "Medicamentos" {
                database::cambiar_estado(pool, telefono, &UserState::MenuFarmacia.to_string()).await;
                crate::whatsapp::enviar_botones(telefono, "💊 *Menú Farmacia*\n¿Qué deseas hacer?", vec!["Buscar", "Ver Lista", "Regresar"]).await;
            } else if entrada == "Laboratorio" {
                let estudios = database::obtener_nombres_estudios(pool).await;
                database::cambiar_estado(pool, telefono, &UserState::SeleccionandoExamen.to_string()).await;
                crate::whatsapp::enviar_lista(telefono, "🔬 Estudios", "Selecciona un análisis:", "Ver Estudios", estudios).await;
            } else {
                enviar_bienvenida(pool, telefono).await;
            }
        },

        // Delegar a lab
        UserState::SeleccionandoExamen => {
            let _ = procesar_lab(pool, telefono, entrada, estado).await;
        },

        // Delegar a pharmacy
        UserState::MenuFarmacia | UserState::EsperandoCategoria | UserState::AgregandoProducto | UserState::EsperandoBusqueda => {
            let _ = procesar_farmacia(pool, telefono, entrada, estado, &patient_id).await;
        },

        // Delegar a users
        UserState::ConfirmandoPedido | UserState::EsperandoPrimerNombre | UserState::EsperandoApellidoPaterno | UserState::EsperandoApellidoMaterno | UserState::EsperandoEmail | UserState::EsperandoCurp | UserState::EsperandoGenero | UserState::EsperandoDireccion => {
            let _ = procesar_usuario(pool, telefono, entrada, estado, &user_id, &patient_id).await;
        },

        _ => { enviar_bienvenida(pool, telefono).await; }
    }
}
