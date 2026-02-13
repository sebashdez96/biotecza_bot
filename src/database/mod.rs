// Módulos
pub mod users;
pub mod pharmacy;
pub mod lab;

// Re-exportar funciones de users
pub use users::{
    obtener_estado, cambiar_estado, obtener_usuario_por_telefono,
    obtener_patient_id_por_telefono, registrar_paciente_completo, actualizar_datos_usuario,
    actualizar_email_usuario, guardar_direccion_paciente,
};

// Re-exportar tipos y funciones de pharmacy
pub use pharmacy::{
    obtener_categorias, buscar_productos_categoria, obtener_productos_nombres_y_ids,
    obtener_detalle_med_por_nombre, agregar_al_carrito, obtener_o_crear_orden,
    buscar_medicamentos_similares,
    obtener_resumen_carrito,
};

// Re-exportar funciones de lab
pub use lab::{obtener_nombres_estudios, obtener_detalle_estudio};
