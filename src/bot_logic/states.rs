use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserState {
    // Estados iniciales
    Inicio,
    
    // Laboratorio
    SeleccionandoExamen,
    
    // Farmacia
    MenuFarmacia,
    EsperandoCategoria,
    AgregandoProducto,
    EsperandoBusqueda,
    ConfirmandoPedido,
    ConfirmandoSeleccion,
    AgregandoProductoFinal,
    ConfirmandoTicket,
    ValidandoCp,
    
    // Usuario / Perfil
    EsperandoDatosFlow,
    EsperandoReceta,
}

impl ToString for UserState {
    fn to_string(&self) -> String {
        match self {
            UserState::Inicio => "INICIO".to_string(),
            UserState::SeleccionandoExamen => "SELECCIONANDO_EXAMEN".to_string(),
            UserState::MenuFarmacia => "MENU_FARMACIA".to_string(),
            UserState::EsperandoCategoria => "ESPERANDO_CATEGORIA".to_string(),
            UserState::AgregandoProducto => "AGREGANDO_PRODUCTO".to_string(),
            UserState::EsperandoBusqueda => "ESPERANDO_BUSQUEDA".to_string(),
            UserState::ConfirmandoPedido => "CONFIRMANDO_PEDIDO".to_string(),
            UserState::ValidandoCp => "VALIDANDO_CP".to_string(),
            UserState::EsperandoDatosFlow => "ESPERANDO_DATOS_FLOW".to_string(),
            UserState::EsperandoReceta => "ESPERANDO_RECETA".to_string(),
            UserState::ConfirmandoSeleccion => "CONFIRMANDO_SELECCION".to_string(),
            UserState::AgregandoProductoFinal => "AGREGANDO_PRODUCTO_FINAL".to_string(),
            UserState::ConfirmandoTicket => "CONFIRMANDO_TICKET".to_string(),
        }
    }
}

impl FromStr for UserState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "INICIO" => Ok(UserState::Inicio),
            "SELECCIONANDO_EXAMEN" => Ok(UserState::SeleccionandoExamen),
            "MENU_FARMACIA" => Ok(UserState::MenuFarmacia),
            "ESPERANDO_CATEGORIA" => Ok(UserState::EsperandoCategoria),
            "AGREGANDO_PRODUCTO" => Ok(UserState::AgregandoProducto),
            "ESPERANDO_BUSQUEDA" => Ok(UserState::EsperandoBusqueda),
            "CONFIRMANDO_SELECCION" => Ok(UserState::ConfirmandoSeleccion),
            "AGREGANDO_PRODUCTO_FINAL" => Ok(UserState::AgregandoProductoFinal),
            "CONFIRMANDO_TICKET" => Ok(UserState::ConfirmandoTicket),
            "CONFIRMANDO_PEDIDO" => Ok(UserState::ConfirmandoPedido),
            "VALIDANDO_CP" => Ok(UserState::ValidandoCp),
            "ESPERANDO_DATOS_FLOW" => Ok(UserState::EsperandoDatosFlow),
            "ESPERANDO_RECETA" => Ok(UserState::EsperandoReceta),
            _ => Err(format!("Estado desconocido: {}", s)),
        }
    }
}
