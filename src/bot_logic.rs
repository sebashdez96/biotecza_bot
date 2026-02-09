use sqlx::PgPool;
use crate::{database, whatsapp};

pub async fn procesar(pool: &PgPool, telefono: &str, entrada: &str) {
    let estado = database::obtener_estado(pool, telefono).await;

    // Comando de reseteo rápido
    if entrada.to_lowercase() == "hola" || entrada.to_lowercase() == "inicio" {
        database::cambiar_estado(pool, telefono, "INICIO").await;
        whatsapp::enviar_botones(telefono, "¡Bienvenido a Biotecza! ¿Qué buscas?", vec!["Laboratorio", "Medicamentos"]).await;
        return;
    }

    match estado.as_str() {
        "INICIO" => {
            if entrada == "Medicamentos" {
                database::cambiar_estado(pool, telefono, "MENU_FARMACIA").await;
                whatsapp::enviar_botones(telefono, "💊 Menú Farmacia", vec!["Buscar", "Ver Lista", "Regresar"]).await;
            }
        },
        "MENU_FARMACIA" => {
            match entrada {
                "Ver Lista" => {
                    let cats = database::obtener_categorias(pool).await;
                    database::cambiar_estado(pool, telefono, "ESPERANDO_CATEGORIA").await;
                    whatsapp::enviar_lista(telefono, "📂 Categorías", "Elige una:", "Ver", cats).await;
                },
                "Regresar" => { /* Volver a inicio */ },
                _ => {}
            }
        },
"ESPERANDO_CATEGORIA" => {
    // 1. Generamos el diseño que te gusta
    let respuesta = formatear_lista_medicamentos(pool, entrada).await;
    
    // 2. Lo enviamos
    crate::whatsapp::enviar_texto(telefono, &respuesta).await;
    
    // 3. Regresamos al menú anterior
    database::cambiar_estado(pool, telefono, "MENU_FARMACIA").await;
    crate::whatsapp::enviar_botones(telefono, "¿Deseas ver otra categoría?", vec!["Ver Lista", "Regresar"]).await;
},
        _ => {}
    }
}

pub async fn formatear_lista_medicamentos(pool: &sqlx::PgPool, categoria: &str) -> String {
    let items = database::buscar_meds_por_categoria(pool, categoria).await;

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