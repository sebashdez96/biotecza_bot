use sqlx::PgPool;
use crate::{database, whatsapp};
use rust_decimal::Decimal;
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
        UserState::MenuFarmacia => {
            match entrada {
                "Ver Lista" => {
                    let cats = database::obtener_categorias(pool).await;
                    database::cambiar_estado(pool, telefono, "ESPERANDO_CATEGORIA").await;
                    whatsapp::enviar_lista(telefono, "📂 Categorías", "Elige una:", "Ver", cats).await;
                },
                "Buscar" => {
                    database::cambiar_estado(pool, telefono, "ESPERANDO_BUSQUEDA").await;
                    whatsapp::enviar_texto(telefono, "🔍 Escribe el nombre del medicamento:").await;
                },
                "Regresar" => { super::users::enviar_bienvenida(pool, telefono).await; },
                _ => {}
            }
            true
        },

        UserState::EsperandoCategoria => {
            // Enviar primero la lista detallada (nombre de patente, compuesto activo, precio)
            let detalle = formatear_lista_medicamentos(pool, entrada).await;
            whatsapp::enviar_texto(telefono, &detalle).await;

            // Luego enviar la lista interactiva para poder añadir al carrito por nombre
            let productos = database::obtener_productos_nombres_y_ids(pool, entrada).await;
            database::cambiar_estado(pool, telefono, &UserState::AgregandoProducto.to_string()).await;
            whatsapp::enviar_lista(telefono, &format!("💊 {}", entrada), "Añadir al carrito:", "Añadir", productos).await;
            true
        },

UserState::AgregandoProducto => {
    if entrada == "Finalizar Pedido" {
        // ... (tu lógica actual de finalizar pedido)
    } else if entrada == "Ver Lista" {
        // ... (tu lógica actual de ver lista)
    } else if entrada == "Agregar más" {
        // Mostramos las opciones de búsqueda/navegación nuevamente
        let msg = "🛒 ¿Cómo deseas buscar el siguiente producto?";
        whatsapp::enviar_botones(telefono, msg, vec!["Buscar", "Ver Lista", "Finalizar Pedido"]).await;
    } else if entrada == "Cancelar Pedido" {
        // Lógica opcional para limpiar el carrito o simplemente volver al inicio
        super::users::enviar_bienvenida(pool, telefono).await;
    } else {
        // Lógica para añadir el producto seleccionado
        if let Some(med) = database::obtener_detalle_med_por_nombre(pool, entrada).await {
            let order_id = database::obtener_o_crear_orden(pool, *patient_id).await;
            database::agregar_al_carrito(pool, order_id, med.med_id, med.price).await;
            
            // CAMBIO AQUÍ: Enviamos botones que inviten a seguir o terminar
            let msg = format!("✅ *{}* añadido al carrito.", med.brand_name);
            whatsapp::enviar_botones(
                telefono, 
                &msg, 
                vec!["Agregar más", "Finalizar Pedido", "Cancelar Pedido"]
            ).await;
            
            // IMPORTANTE: Mantenemos el estado en AgregandoProducto para procesar 
            // los botones que acabamos de enviar.
        }
    }
    true
},
        UserState::EsperandoBusqueda => {
            // Interpretar la entrada como término de búsqueda (nombre o compuesto)
            // Buscamos coincidencias en la DB y enviamos una lista de resultados similares
            let term = entrada.trim();
            if term.is_empty() {
                database::cambiar_estado(pool, telefono, &UserState::MenuFarmacia.to_string()).await;
                whatsapp::enviar_botones(telefono, "¿Qué deseas hacer?", vec!["Buscar", "Ver Lista", "Regresar"]).await;
                return true;
            }

            let resultados = database::buscar_medicamentos_similares(pool, term).await;

            if resultados.is_empty() {
                let msg = format!("No encontré medicamentos relacionados con '{}'. Intenta con otra palabra clave.", term);
                whatsapp::enviar_texto(telefono, &msg).await;
                database::cambiar_estado(pool, telefono, &UserState::MenuFarmacia.to_string()).await;
                whatsapp::enviar_botones(telefono, "¿Qué deseas hacer?", vec!["Buscar", "Ver Lista", "Regresar"]).await;
                return true;
            }

            // Formatear resultados en texto legible
            let mut texto = format!("🔎 Resultados para '{}':\n", term);
            texto.push_str("━━━━━━━━━━━━━━━\n\n");
            let mut nombres: Vec<String> = Vec::new();

            for (brand, compound, presentation, price) in &resultados {
                let pres = presentation.clone().unwrap_or_else(|| "N/A".to_string());
                texto.push_str(&format!("• *{}*\n  Compuesto: {}\n  Presentación: {}\n  💰 ${}\n\n", brand, compound, pres, price));
                nombres.push(brand.clone());
            }

            texto.push_str("⚠️ Si quieres agregar un producto, tocá su nombre en la lista siguiente.");

            // Enviar texto con detalles y luego la lista interactiva (por brand_name)
            whatsapp::enviar_texto(telefono, &texto).await;
            database::cambiar_estado(pool, telefono, &UserState::AgregandoProducto.to_string()).await;
            whatsapp::enviar_lista(telefono, &format!("🔎 Selecciona uno:"), "Añadir al carrito:", "Añadir", nombres).await;
            true
        },

        _ => false,
    }
}

#[allow(dead_code)]
pub async fn formatear_lista_medicamentos(pool: &sqlx::PgPool, categoria: &str) -> String {
    let items: Vec<(String, String, Option<String>, Decimal)> = 
        database::buscar_productos_categoria(pool, categoria).await;

    if items.is_empty() {
        return format!("Por el momento no tenemos stock disponible en la categoría *{}*.", categoria);
    }

    let mut res = format!("💊 *Productos en {}:*\n", categoria);
    res.push_str("━━━━━━━━━━━━━━━\n\n");

    for i in items {
        let dosis = i.2.unwrap_or_else(|| "N/A".to_string());
        
        res.push_str(&format!(
            "📌 *{}*\n🧪 Compuesto: {}\n⚖️ Dosis/Pres: {}\n💰 Precio: ${}\n\n",
            i.0.to_uppercase(), 
            i.1, 
            dosis, 
            i.3
        ));
    }
    
    res.push_str("⚠️ *Recuerde:* Algunos medicamentos requieren receta médica.");
    res
}

pub async fn generar_ticket_virtual(pool: &PgPool, order_id: Uuid) -> String {
    let items = database::obtener_resumen_carrito(pool, order_id).await;
    
    if items.is_empty() {
        return "Tu carrito está vacío. 🛒".to_string();
    }

    let mut ticket = "📝 *RESUMEN DE TU PEDIDO*\n".to_string();
    ticket.push_str("━━━━━━━━━━━━━━━\n\n");

    let mut total: Decimal = Decimal::from(0);

    for (nombre, cant, precio) in items {
        let subtotal = precio * Decimal::from(cant);
        total += subtotal;
        ticket.push_str(&format!("• {} (x{})\n  Subtotal: ${}\n\n", nombre, cant, subtotal));
    }

    ticket.push_str("━━━━━━━━━━━━━━━\n");
    ticket.push_str(&format!("💰 *TOTAL A PAGAR: ${}*\n\n", total));
    ticket.push_str("¿Deseas confirmar este pedido?");
    
    ticket
}
