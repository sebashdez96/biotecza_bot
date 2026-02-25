use serde_json::json;
use reqwest::Client;

pub async fn enviar_texto(telefono: &str, texto: &str) {
    llamar_meta(json!({
        "messaging_product": "whatsapp", "to": telefono,
        "type": "text", "text": { "body": texto }
    })).await;
}

pub async fn enviar_botones(telefono: &str, texto: &str, botones: Vec<&str>) {
    let buttons_json: Vec<serde_json::Value> = botones.iter().map(|&b| {
        json!({ "type": "reply", "reply": { "id": b, "title": b } })
    }).collect();

    llamar_meta(json!({
        "messaging_product": "whatsapp", "to": telefono,
        "type": "interactive",
        "interactive": {
            "type": "button",
            "body": { "text": texto },
            "action": { "buttons": buttons_json }
        }
    })).await;
}

pub async fn enviar_lista(telefono: &str, titulo: &str, cuerpo: &str, boton: &str, opciones: Vec<String>) {
    let rows: Vec<serde_json::Value> = opciones.iter().map(|op| {
        json!({ "id": op, "title": op }) 
    }).collect();

    llamar_meta(json!({
        "messaging_product": "whatsapp", "to": telefono, "type": "interactive",
        "interactive": {
            "type": "list",
            "header": { "type": "text", "text": titulo },
            "body": { "text": cuerpo },
            "action": { "button": boton, "sections": [{ "title": "Opciones", "rows": rows }] }
        }
    })).await;
}

async fn llamar_meta(body: serde_json::Value) {
    let token = std::env::var("WHATSAPP_TOKEN").unwrap_or_default();
    let phone_id = std::env::var("PHONE_NUMBER_ID").unwrap_or_default();
    let url = format!("https://graph.facebook.com/v21.0/{}/messages", phone_id);
    let _ = Client::new().post(url).bearer_auth(token).json(&body).send().await;
}


pub async fn enviar_flow_envio(
    telefono: &str,
    cp: &str,
    info: &crate::database::InfoPostal,
) {
    let phone_number_id = std::env::var("PHONE_NUMBER_ID").unwrap_or_default();
    let access_token = std::env::var("WHATSAPP_TOKEN").unwrap_or_default();
    let flow_id = "1246754296913855"; // Reemplaza con el ID de Meta

    let url = format!("https://graph.facebook.com/v21.0/{}/messages", phone_number_id);

    let body = serde_json::json!({
        "messaging_product": "whatsapp",
        "recipient_type": "individual",
        "to": telefono,
        "type": "interactive",
        "interactive": {
            "type": "flow",
            "header": {
                "type": "text",
                "text": "📝 Datos de Entrega"
            },
            "body": {
                "text": "¡Cobertura confirmada! Por favor, completa los detalles para tu envío a Biotecza."
            },
            "footer": {
                "text": "Biotecza Farmacia"
            },
            // ... dentro de la función enviar_flow_envio ...
            "action": {
                "name": "flow",
                "parameters": {
                    "flow_message_version": "3", // <-- Es buena práctica indicarle la versión del mensaje
                    "flow_token": uuid::Uuid::new_v4().to_string(),
                    "flow_id": flow_id,
                    "flow_cta": "Completar Dirección",
                    "flow_action": "navigate", 
                    "mode": "draft",
                    "flow_action_payload": { // <--- ESTA ES LA LLAVE MÁGICA
                        "screen": "DATOS_ENVIO",
                        "data": {
                            "cp": cp,
                            "municipio": info.municipio,
                            "estado": info.estado,
                            "colonias": info.colonias
                        }
                    }
                }
            }
        }
    });

    let client = reqwest::Client::new();
    let res = client.post(url)
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .await;

    match res {
        Ok(response) => {
            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                eprintln!("Error de Meta al enviar Flow: {}", error_text);
            }
        },
        Err(e) => eprintln!("Error de red al enviar Flow: {:?}", e),
    }
}