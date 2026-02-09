use sqlx::PgPool;
use crate::{database, whatsapp};
use rust_decimal::Decimal;
use uuid::Uuid;


// Esta función solo se encarga de mostrar el saludo inicial
pub async fn enviar_bienvenida(pool: &PgPool, telefono: &str) {
    crate::database::cambiar_estado(pool, telefono, "INICIO").await;
    crate::whatsapp::enviar_botones(
        telefono, 
        "¡Hola! Bienvenido a *Biotecza*.\nSelecciona una opción:", 
        vec!["Laboratorio", "Medicamentos"]
    ).await;
}

pub async fn procesar(pool: &PgPool, telefono: &str, entrada: &str) {
    let estado: String = database::obtener_estado(pool, telefono).await;
    println!("🤖 Usuario: {} | Estado: {} | Entrada: {}", telefono, estado, entrada);

    // 1. Identificación de IDs (Scope superior)
    let user_id = match database::obtener_usuario_por_telefono(pool, telefono).await {
        Some(id) => id,
        None => database::registrar_paciente_completo(pool, telefono, "Usuario WhatsApp").await
    };

    let patient_id = database::obtener_patient_id_por_telefono(pool, telefono)
        .await
        .expect("El paciente debería existir");

    // 2. Comandos Globales
    if entrada.to_lowercase() == "hola" || entrada.to_lowercase() == "inicio" {
        enviar_bienvenida(pool, telefono).await;
        return;
    }

    // 3. Máquina de Estados Principal
    match estado.as_str() {
        "INICIO" => {
            if entrada == "Medicamentos" {
                database::cambiar_estado(pool, telefono, "MENU_FARMACIA").await;
                whatsapp::enviar_botones(telefono, "💊 *Menú Farmacia*\n¿Qué deseas hacer?", vec!["Buscar", "Ver Lista", "Regresar"]).await;
            } else if entrada == "Laboratorio" {
                let estudios = database::obtener_nombres_estudios(pool).await;
                database::cambiar_estado(pool, telefono, "SELECCIONANDO_EXAMEN").await;
                whatsapp::enviar_lista(telefono, "🔬 Estudios", "Selecciona un análisis:", "Ver Estudios", estudios).await;
            } else {
                enviar_bienvenida(pool, telefono).await;
            }
        },

        "SELECCIONANDO_EXAMEN" => {
            if let Some(detalle) = database::obtener_detalle_estudio(pool, entrada).await {
                let mensaje = format!(
                    "✅ *Información del Estudio*\n━━━━━━━━━━━━━━━\n\n🧪 *{}*\n📝 *Instrucciones:* {}\n💰 *Precio:* ${}\n\n¿Deseas consultar otro estudio?",
                    detalle.0.to_uppercase(), detalle.1, detalle.2
                );
                whatsapp::enviar_texto(telefono, &mensaje).await;
                
                let estudios = database::obtener_nombres_estudios(pool).await;
                whatsapp::enviar_lista(telefono, "🔬 Otros Estudios", "Selecciona otro:", "Ver Estudios", estudios).await;
                whatsapp::enviar_botones(telefono, "O vuelve al inicio:", vec!["Regresar"]).await;
            } else if entrada == "Regresar" {
                enviar_bienvenida(pool, telefono).await;
            }
        },

        "MENU_FARMACIA" => {
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
                "Regresar" => { enviar_bienvenida(pool, telefono).await; },
                _ => {}
            }
        },

        "ESPERANDO_CATEGORIA" => {
            let productos = database::obtener_productos_nombres_y_ids(pool, entrada).await;
            database::cambiar_estado(pool, telefono, "AGREGANDO_PRODUCTO").await;
            whatsapp::enviar_lista(telefono, &format!("💊 {}", entrada), "Añadir al carrito:", "Añadir", productos).await;
        },

        "AGREGANDO_PRODUCTO" => {
            if entrada == "Finalizar Pedido" {
                let order_id = database::obtener_o_crear_orden(pool, patient_id).await;
                let ticket = generar_ticket_virtual(pool, order_id).await;
                whatsapp::enviar_texto(telefono, &ticket).await;

                database::cambiar_estado(pool, telefono, "CONFIRMANDO_PEDIDO").await;
                whatsapp::enviar_botones(telefono, "¿Todo correcto?", vec!["Confirmar Pedido", "Seguir Comprando"]).await;
            }                 
            else if entrada == "Ver Lista" {
                    let cats = database::obtener_categorias(pool).await;
                    database::cambiar_estado(pool, telefono, "ESPERANDO_CATEGORIA").await;
                    whatsapp::enviar_lista(telefono, "📂 Categorías", "Elige una:", "Ver", cats).await;
                }
            
            else {
                // Lógica para agregar el medicamento si la entrada es el nombre del producto
                if let Some(med) = database::obtener_detalle_med_por_nombre(pool, entrada).await {
                    let order_id = database::obtener_o_crear_orden(pool, patient_id).await;
                    database::agregar_al_carrito(pool, order_id, med.id, med.price).await;
                    
                    let msg = format!("🛒 *{}* añadido.\n¿Deseas algo más?", entrada);
                    whatsapp::enviar_botones(telefono, &msg, vec!["Ver Lista", "Finalizar Pedido", "Inicio"]).await;
                }
            }
        },

        "CONFIRMANDO_PEDIDO" => {
            if entrada == "Confirmar Pedido" {
                database::cambiar_estado(pool, telefono, "ESPERANDO_NOMBRE").await;
                whatsapp::enviar_texto(telefono, "¡Excelente! ¿Podrías decirme tu *Nombre y Apellido*?").await;
            } else {
                enviar_bienvenida(pool, telefono).await;
            }
        },

        "ESPERANDO_NOMBRE" => {
            let partes: Vec<&str> = entrada.split_whitespace().collect();
            let first = partes.get(0).unwrap_or(&"").to_string();
            let last = partes.get(1..).unwrap_or(&[""]).join(" ");
            database::actualizar_datos_usuario(pool, user_id, &first, &last).await;
            
            database::cambiar_estado(pool, telefono, "ESPERANDO_EMAIL").await;
            whatsapp::enviar_texto(telefono, &format!("Mucho gusto, {}. ¿Cuál es tu *correo*?", first)).await;
        },

        "ESPERANDO_EMAIL" => {
            database::actualizar_email_usuario(pool, user_id, entrada).await;
            database::cambiar_estado(pool, telefono, "ESPERANDO_CURP").await;
            whatsapp::enviar_texto(telefono, "Gracias. Ahora ingresa tu *CURP* (18 caracteres):").await;
        },

        "ESPERANDO_CURP" => {
            if entrada.len() == 18 {
                sqlx::query!("UPDATE patients SET curp = $1 WHERE patient_id = $2", entrada, patient_id).execute(pool).await.ok();
                database::cambiar_estado(pool, telefono, "ESPERANDO_GENERO").await;
                whatsapp::enviar_botones(telefono, "¿Cuál es tu género?", vec!["M", "F"]).await;
            } else {
                whatsapp::enviar_texto(telefono, "❌ CURP inválido. Inténtalo de nuevo:").await;
            }
        },

        "ESPERANDO_GENERO" => {
            sqlx::query!("UPDATE patients SET gender = $1 WHERE patient_id = $2", entrada, patient_id).execute(pool).await.ok();
            database::cambiar_estado(pool, telefono, "ESPERANDO_DIRECCION").await;
            whatsapp::enviar_texto(telefono, "📍 ¿Cuál es la *dirección completa*?").await;
        },

        "ESPERANDO_DIRECCION" => {
            database::guardar_direccion_paciente(pool, patient_id, entrada).await;
            database::cambiar_estado(pool, telefono, "ESPERANDO_RECETA").await;
            whatsapp::enviar_texto(telefono, "✅ ¡Listo! Ahora envía la *foto de tu receta médica*.").await;
        },

        "ESPERANDO_BUSQUEDA" => {
            database::cambiar_estado(pool, telefono, "MENU_FARMACIA").await;
            whatsapp::enviar_botones(telefono, "¿Qué deseas hacer?", vec!["Buscar", "Ver Lista", "Regresar"]).await;
        },
        

        _ => { enviar_bienvenida(pool, telefono).await; }
    }
}


pub async fn formatear_lista_medicamentos(pool: &sqlx::PgPool, categoria: &str) -> String {
let items: Vec<(String, String, Option<String>, rust_decimal::Decimal)> = 
    database::buscar_productos_categoria(pool, categoria).await;

    if items.is_empty() {
        return format!("Por el momento no tenemos stock disponible en la categoría *{}*.", categoria);
    }

    let mut res = format!("💊 *Productos en {}:*\n", categoria);
    res.push_str("━━━━━━━━━━━━━━━\n\n");

    for i in items {
        // i.0 = brand_name, i.1 = active_compound, i.2 = presentation, i.3 = price
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
