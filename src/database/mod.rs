// Módulos
pub mod users;
pub mod pharmacy;
pub mod lab;

// Re-exportar funciones de users
pub use users::{
    obtener_estado, cambiar_estado, obtener_usuario_por_telefono,
    obtener_patient_id_por_telefono, registrar_paciente_completo,
    finalizar_pedido_con_datos_flow
    
};

// Re-exportar tipos y funciones de pharmacy
pub use pharmacy::{
    obtener_detalle_med_por_nombre, agregar_al_carrito, obtener_o_crear_orden,
    buscar_medicamentos_similares,
    obtener_orden_farmacia_pendiente, cancelar_pedido_farmacia,
    guardar_ultimo_termino_busqueda, obtener_ultimo_termino_busqueda,
    guardar_producto_seleccionado, obtener_producto_seleccionado,
    obtener_resumen_carrito, obtener_info_por_cp, InfoPostal
};

// Re-exportar funciones de lab
pub use lab::{obtener_nombres_estudios, obtener_detalle_estudio, obtener_orden_lab_pendiente, cancelar_pedido_lab};
