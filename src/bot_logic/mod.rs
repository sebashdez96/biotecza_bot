// Módulos
pub mod users;
pub mod pharmacy;
pub mod lab;
pub mod states;
pub mod models;
use crate::whatsapp;

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
    let estado = UserState::from_str(&estado_str).unwrap_or(UserState::Nuevo);
    
    // 1. Identificación de IDs
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

        UserState::Nuevo => {
            let saludo = "Hola, no te había visto por aquí. 👀 Mucho gusto, soy el Asistente virtual de *Biotecza*.\n\n\
                          ¿Cuál es tu nombre? 👇🏼\n\
                          _Escribe solo tu nombre_";
            
            crate::database::cambiar_estado(pool, telefono, &UserState::EsperandoNombre.to_string()).await;
            whatsapp::enviar_texto(telefono, saludo).await;
        }

UserState::EsperandoNombre => {
    // Usamos 'entrada' que es el texto que envió el usuario
    let nombre_recibido = entrada.trim();
    
    // Guardamos en la tabla 'users' directamente o en un campo temporal
    database::users::actualizar_nombre_temporal(pool, user_id, nombre_recibido).await;
    
    let pregunta = format!("¡Hola, *{}*! ¿Es correcto tu nombre? 👇🏼", nombre_recibido);
    let botones = vec!["✅ Sí, es correcto", "❌ No, corregir"];
    
    database::cambiar_estado(pool, telefono, &UserState::ConfirmandoNombre.to_string()).await;
    whatsapp::enviar_botones(telefono, &pregunta, botones).await;
}

UserState::ConfirmandoNombre => {
    if entrada.contains("Sí") {
        // Recuperamos el nombre que guardamos en el paso anterior
        let nombre = database::users::obtener_nombre_temporal(pool, user_id).await.unwrap_or("Amigo".to_string());
        
        // Mensaje de Privacidad independiente
        let aviso = format!("¡Mucho gusto, *{}*! Conoce aquí nuestro Aviso de Privacidad 👇\n\
                             https://biotecza.com/privacidad", nombre);
        whatsapp::enviar_texto(telefono, &aviso).await;

        // Menú Principal
        let menu = "¿Qué necesitas hoy? Elige la opción que mejor se adapte a tu solicitud 👇😊";
        let opciones = vec!["🔬 Laboratorio", "💊 Medicamentos"];
        
        database::cambiar_estado(pool, telefono, &UserState::Inicio.to_string()).await; // Volvemos a Inicio para que el menú funcione
        whatsapp::enviar_botones(telefono, menu, opciones).await;
    } else {
        let reintento = "No te preocupes, ¿cómo te llamas entonces? 👇🏼";
        database::cambiar_estado(pool, telefono, &UserState::EsperandoNombre.to_string()).await;
        whatsapp::enviar_texto(telefono, reintento).await;
    }
} // Delegar a lab
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
