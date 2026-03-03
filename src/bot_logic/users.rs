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

UserState::EsperandoNombreCompleto => {
    println!("🤖 1. Entró al estado EsperandoNombreCompleto");
    let entrada_limpia = entrada.trim();
    let partes: Vec<&str> = entrada_limpia.split_whitespace().collect();
    
    if partes.len() < 2 {
        whatsapp::enviar_texto(telefono, "⚠️ Por favor, escribe tu nombre y al menos un apellido (si solo tienes uno, pon una 'X' al final).").await;
        return true;
    }

    println!("🤖 2. Separando el nombre en partes...");
    let (first_name, paternal, mut maternal) = match partes.len() {
        2 => {
            (partes[0].to_string(), partes[1].to_string(), "".to_string())
        },
        _ => {
            let mat = partes.last().unwrap().to_string();
            let pat = partes[partes.len() - 2].to_string();
            let nombre = partes[0..partes.len() - 2].join(" ");
            (nombre, pat, mat)
        }
    };

    if maternal.to_lowercase() == "x" { maternal = "".to_string(); }
    let paternal_final = if paternal.to_lowercase() == "x" { "".to_string() } else { paternal };

    // --- NUEVA VALIDACIÓN AQUÍ ---
    // Verificamos que al menos exista un apellido paterno válido después de limpiar las "X"
    if paternal_final.trim().is_empty() {
        whatsapp::enviar_texto(
            telefono, 
            "⚠️ Necesitamos al menos un apellido. Por favor, escribe tu nombre y apellido (ej. Juan Pérez X)."
        ).await;
        return true; // Detenemos el flujo para que lo vuelva a intentar
    }
    // -----------------------------

    println!("🤖 3. Datos a guardar -> Nombre: '{}' | Paterno: '{}' | Materno: '{}'", first_name, paternal_final, maternal);

    match crate::database::users::actualizar_nombre_completo(
        pool, user_id, &first_name, &paternal_final, &maternal
    ).await {
        Ok(_) => {
            println!("🤖 4. Base de datos actualizada con éxito (UPDATE users).");
            
            database::cambiar_estado(pool, telefono, "ESPERANDO_CALLE").await;
            println!("🤖 5. Estado cambiado a ESPERANDO_CALLE.");
            
            whatsapp::enviar_texto(
                telefono, 
                "📝 ¡Anotado!\n\nAhora, por favor escribe tu *Calle y Número* (exterior e interior si aplica).\n_Ejemplo: Av. Hidalgo 123, Int 4_"
            ).await;
            println!("🤖 6. Mensaje de WhatsApp enviado.");
        },
        Err(e) => {
            eprintln!("❌ ERROR en base de datos: {:?}", e);
            whatsapp::enviar_texto(telefono, "❌ Hubo un error al guardar tu nombre. Intenta de nuevo.").await;
        }
    }
    true
},

UserState::EsperandoCalle => {
    let calle = entrada.trim();
    
    // Validación básica: que no pongan solo "hola" o "x"
    if calle.len() < 5 {
        whatsapp::enviar_texto(
            telefono, 
            "⚠️ La dirección parece muy corta. Por favor incluye tu calle y número exterior/interior."
        ).await;
        return true;
    }

    // Llamamos a la BD para crear el registro inicial de la dirección
    match crate::database::users::iniciar_direccion_paciente(pool, patient_id, calle).await {
        Ok(_) => {
            database::cambiar_estado(pool, telefono, "ESPERANDO_COLONIA").await;
            
            // Usamos un ejemplo local para que se entienda el formato
            whatsapp::enviar_texto(
                telefono, 
                "📍 ¡Perfecto!\n\nAhora escribe tu *Colonia* y *Código Postal* separados por una coma.\n_Ejemplo: Centro, 98000, Zacatecas_"
            ).await;
        },
        Err(e) => {
            eprintln!("❌ ERROR al guardar la calle: {:?}", e);
            whatsapp::enviar_texto(telefono, "❌ Hubo un error al guardar tu calle. Intenta de nuevo.").await;
        }
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