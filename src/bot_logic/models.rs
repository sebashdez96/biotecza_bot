use uuid::Uuid;
use rust_decimal::Decimal;

/// Modelo para usuario de la aplicación
#[derive(Debug, Clone)]
pub struct User {
    pub user_id: uuid::Uuid,
    pub first_name: String,
    pub paternal_last_name: String,
    pub maternal_last_name: String,
    pub email: String,
    pub phone: String,
}

/// Modelo para paciente (extensión de usuario)
#[derive(Debug, Clone)]
pub struct Patient {
    pub patient_id: Uuid,
    pub user_id: Uuid,
    pub curp: Option<String>,
    pub whatsapp_number: String,
    pub gender: Option<char>,
}

/// Modelo para datos completos de usuario (usuario + paciente)
#[derive(Debug, Clone)]
pub struct UserProfile {
    pub user: User,
    pub patient: Patient,
}

/// Modelo para medicamento/producto de farmacia
#[derive(Debug, Clone)]
pub struct Medication {
    pub med_id: Uuid,
    pub brand_name: String,
    pub active_compound: String,
    pub presentation: Option<String>,
    pub price: Decimal,
    pub category: Option<String>,
    pub stock: bool,
}

/// Modelo para estudio/prueba de laboratorio
#[derive(Debug, Clone)]
pub struct LabTest {
    pub test_id: Option<Uuid>,
    pub test_name: String,
    pub instructions: String,
    pub price: Decimal,
    pub category: Option<String>,
}

/// Modelo para orden/pedido
#[derive(Debug, Clone)]
pub struct Order {
    pub order_id: Uuid,
    pub patient_id: Uuid,
    pub order_type: String, // "medication" o "lab"
    pub total_amount: Decimal,
    pub payment_method: String,
    pub status: String,
}

/// Modelo para item de medicamento en una orden
#[derive(Debug, Clone)]
pub struct MedicationOrderItem {
    pub item_id: Option<Uuid>,
    pub order_id: Uuid,
    pub med_id: Uuid,
    pub quantity: i32,
    pub unit_price: Decimal,
}

/// Modelo para carrito de compras temporal
#[derive(Debug, Clone)]
pub struct Cart {
    pub items: Vec<CartItem>,
    pub total: Decimal,
}

/// Item en el carrito
#[derive(Debug, Clone)]
pub struct CartItem {
    pub med_id: Uuid,
    pub brand_name: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub subtotal: Decimal,
}

impl CartItem {
    pub fn new(med_id: Uuid, brand_name: String, quantity: i32, unit_price: Decimal) -> Self {
        let subtotal = unit_price * Decimal::from(quantity);
        CartItem {
            med_id,
            brand_name,
            quantity,
            unit_price,
            subtotal,
        }
    }
}

impl Cart {
    pub fn new() -> Self {
        Cart {
            items: Vec::new(),
            total: Decimal::ZERO,
        }
    }

    pub fn add_item(&mut self, item: CartItem) {
        self.total += item.subtotal;
        self.items.push(item);
    }

    pub fn remove_item(&mut self, med_id: Uuid) {
        if let Some(pos) = self.items.iter().position(|item| item.med_id == med_id) {
            let item = self.items.remove(pos);
            self.total -= item.subtotal;
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.total = Decimal::ZERO;
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

/// Modelo para sesión de usuario
#[derive(Debug, Clone)]
pub struct UserSession {
    pub telefono: String,
    pub estado: String,
    pub ultima_actualizacion: i64, // Unix timestamp
}
