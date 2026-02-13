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
