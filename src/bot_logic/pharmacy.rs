use sqlx::PgPool;
use crate::{database, whatsapp};
use uuid::Uuid;
use super::states::UserState;


pub async fn procesar_farmacia(
    pool: &PgPool,
    telefono: &str,
    entrada: &str,
    estado: UserState,
    patient_id: &Uuid,
) -> bool {
    match estado {

UserState::EsperandoBusqueda => {
    let term = entrada.trim();
    if term.is_empty() { return true; }

    let resultados = database::buscar_medicamentos_similares(pool, term).await;

    if resultados.is_empty() {
        whatsapp::enviar_texto(telefono, &format!("❌ No encontré resultados para '{}'. Intenta con otro nombre.", term)).await;
        return true;
    }

    let mut msg = format!("🔎 Estos son los resultados para: *{}*\n\n", term);
    for (i, res) in resultados.iter().enumerate() {
        let pres = res.2.clone().unwrap_or_else(|| "N/A".to_string());
        msg.push_str(&format!("{}. *{}* - {} ({}) - *${}*\n\n", i + 1, res.0, res.1, pres, res.3));
    }
    msg.push_str("🔢 Escribe el *número* de la opción que quieres.\nSi ninguna es correcta, escribe: *otra*");

    // Guardamos el término para que el bot "recuerde" la lista cuando el usuario mande un número
    database::guardar_ultimo_termino_busqueda(pool, telefono, term).await;
    
    database::cambiar_estado(pool, telefono, "CONFIRMANDO_SELECCION").await;
    whatsapp::enviar_texto(telefono, &msg).await;
    true
},

UserState::ConfirmandoSeleccion => {
    if entrada.to_lowercase() == "otra" {
        database::cambiar_estado(pool, telefono, "ESPERANDO_BUSQUEDA").await;
        whatsapp::enviar_texto(telefono, "🔍 Escribe de nuevo el nombre del producto:").await;
        return true;
    }

    if let Ok(num) = entrada.parse::<usize>() {
        // Log para ver qué número recibió
        println!("🤖 Usuario envió número: {}", num);

        if let Some(termino) = database::obtener_ultimo_termino_busqueda(pool, telefono).await {
            println!("🤖 Término recuperado de DB: {}", termino);
            
            let resultados = database::buscar_medicamentos_similares(pool, &termino).await;
            
            if num > 0 && num <= resultados.len() {
                let seleccion = &resultados[num - 1];
                let nombre_prod = &seleccion.0;
                let precio_prod = seleccion.3;

                println!("🤖 Selección válida encontrada: {:?}", seleccion);

                database::guardar_producto_seleccionado(pool, telefono, nombre_prod).await;
                
                let msg_conf = format!("✨ ¡Perfecto! Anoté: *{}*\n💰 Precio: *${}*\n\n¿Es correcto?", nombre_prod, precio_prod);
                
                // ASEGÚRATE de que este string sea exactamente igual al del Enum en states.rs
                database::cambiar_estado(pool, telefono, "AGREGANDO_PRODUCTO_FINAL").await;
                whatsapp::enviar_botones(telefono, &msg_conf, vec!["Sí, agregar", "Elegir otro"]).await;
            } else {
                println!("🤖 Número fuera de rango: {}", num);
                whatsapp::enviar_texto(telefono, &format!("⚠️ El número {} no está en la lista. Intenta con un número del 1 al {}.", num, resultados.len())).await;
            }
        } else {
            println!("❌ ERROR: No se encontró el último término de búsqueda en la sesión.");
            // Si esto falla, el bot no sabe qué lista está viendo el usuario
            whatsapp::enviar_texto(telefono, "⚠️ Tu sesión de búsqueda expiró. Por favor, escribe el nombre del medicamento de nuevo:").await;
            database::cambiar_estado(pool, telefono, "ESPERANDO_BUSQUEDA").await;
        }
    } else {
        whatsapp::enviar_texto(telefono, "⚠️ Por favor, escribe solo el número de la opción (ej. 1) o escribe 'otra'.").await;
    }
    true
},

UserState::AgregandoProductoFinal => {
    if entrada == "Sí, agregar" {
        if let Some(nombre_med) = database::obtener_producto_seleccionado(pool, telefono).await {
            if let Some(med) = database::obtener_detalle_med_por_nombre(pool, &nombre_med).await {
                let order_id = database::obtener_o_crear_orden(pool, *patient_id).await;
                database::agregar_al_carrito(pool, order_id, med.med_id, med.price).await;

                whatsapp::enviar_botones(
                    telefono, 
                    &format!("🛒 *{}* añadido al carrito. ¿Deseas algo más?", med.brand_name),
                    vec!["Agregar más", "Finalizar Pedido", "Cancelar Pedido"]
                ).await;
                database::cambiar_estado(pool, telefono, "AGREGANDO_PRODUCTO").await;
            }
        }
    } else {
        // "Elegir otro" o cualquier otra cosa
        database::cambiar_estado(pool, telefono, "ESPERANDO_BUSQUEDA").await;
        whatsapp::enviar_texto(telefono, "🔍 Escribe de nuevo el producto que buscas:").await;
    }
    true
},

UserState::AgregandoProducto => {
    match entrada {
        "Agregar más" => {
            database::cambiar_estado(pool, telefono, "ESPERANDO_BUSQUEDA").await;
            whatsapp::enviar_texto(telefono, "🔍 Escribe el nombre del medicamento que buscas:").await;
        },
        "Finalizar Pedido" => {
            let order_id = database::obtener_o_crear_orden(pool, *patient_id).await;
            let ticket = generar_ticket_con_envio(pool, order_id).await;
            
            database::cambiar_estado(pool, telefono, "CONFIRMANDO_TICKET").await;
            whatsapp::enviar_botones(
                telefono, 
                &ticket, 
                vec!["Sí, todo correcto", "Agregar más", "Cancelar Pedido"]
            ).await;
        },
        "Cancelar Pedido" => {
            let _ = database::cancelar_pedido_farmacia(pool, *patient_id).await;
            database::cambiar_estado(pool, telefono, "INICIO").await;
            whatsapp::enviar_texto(telefono, "❌ Pedido cancelado. ¿En qué más puedo ayudarte?").await;
        },
        _ => {
            whatsapp::enviar_botones(
                telefono, 
                "⚠️ Selecciona una opción para continuar:", 
                vec!["Agregar más", "Finalizar Pedido", "Cancelar Pedido"]
            ).await;
        }
    }
    true
},

UserState::ConfirmandoTicket => {
    if entrada == "Sí, todo correcto" {
        database::cambiar_estado(pool, telefono, "VALIDANDO_CP").await;
        whatsapp::enviar_texto(telefono, "📍 Para verificar la cobertura, por favor escribe tu *Código Postal*:").await;
        return true; // Forzamos el retorno exitoso
    } else if entrada == "Agregar más" {
        database::cambiar_estado(pool, telefono, "ESPERANDO_BUSQUEDA").await;
        whatsapp::enviar_texto(telefono, "🔍 Escribe el nombre del producto:").await;
        return true;
    }
    // Si no es ninguna de las opciones de botones, podrías reenviar los botones o ignorar
    true
},

UserState::ValidandoCp => {
    let cp = entrada.trim();
    
    // Verificamos si hay cobertura
    if let Some(_info) = database::obtener_info_por_cp(pool, cp).await {
        
        // OPCIONAL: Aquí deberías guardar el CP temporalmente en algún lado 
        // para usarlo al final cuando armes la dirección completa.
        // database::guardar_cp_temporal(pool, telefono, cp).await;
        
        let _ = crate::database::users::iniciar_direccion_con_cp(pool, patient_id, cp).await;
        
        database::cambiar_estado(pool, telefono, "ESPERANDO_NOMBRE_COMPLETO").await;
        
        let msg = "✅ ¡Excelente! Sí tenemos cobertura en tu zona.\n\nPara agendar tu envío, por favor escribe tu *Nombre Completo* (Nombre y apellidos).\n_Nota: Si solo tienes un apellido, pon una 'X' al final._";
        whatsapp::enviar_texto(telefono, msg).await;
        
    } else {
        whatsapp::enviar_texto(telefono, "❌ Lo sentimos, aún no tenemos cobertura en ese CP. Intenta con otro o contacta a soporte.").await;
    }
    true
},
        _ => false,
    }
}

// Función auxiliar para el ticket con los $20
pub async fn generar_ticket_con_envio(pool: &PgPool, order_id: Uuid) -> String {
    let items = database::obtener_resumen_carrito(pool, order_id).await;
    let mut subtotal = rust_decimal::Decimal::from(0);
    let mut ticket = "📝 *RESUMEN DE TU COMPRA*\n━━━━━━━━━━━━━━\n".to_string();

    for i in items {
        let precio_item = i.2 * rust_decimal::Decimal::from(i.1);
        subtotal += precio_item;
        ticket.push_str(&format!("• {} (x{}) - ${}\n", i.0, i.1, precio_item));
    }

    let envio = rust_decimal::Decimal::from(20);
    let total = subtotal + envio;

    ticket.push_str("━━━━━━━━━━━━━━\n");
    ticket.push_str(&format!("💵 Subtotal: ${}\n", subtotal));
    ticket.push_str(&format!("🚚 Envío: $20.00\n"));
    ticket.push_str(&format!("💰 *TOTAL: ${}*\n\n", total));
    ticket.push_str("_¿Confirmas que tu pedido es correcto?_");
    
    ticket
}