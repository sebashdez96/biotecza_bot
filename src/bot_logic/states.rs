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
    
    // Usuario / Perfil
    EsperandoPrimerNombre,
    EsperandoApellidoPaterno,
    EsperandoApellidoMaterno,
    EsperandoEmail,
    EsperandoCurp,
    EsperandoGenero,
    EsperandoDireccion,
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
            UserState::EsperandoPrimerNombre => "ESPERANDO_PRIMER_NOMBRE".to_string(),
            UserState::EsperandoApellidoPaterno => "ESPERANDO_APELLIDO_PATERNO".to_string(),
            UserState::EsperandoApellidoMaterno => "ESPERANDO_APELLIDO_MATERNO".to_string(),
            UserState::EsperandoEmail => "ESPERANDO_EMAIL".to_string(),
            UserState::EsperandoCurp => "ESPERANDO_CURP".to_string(),
            UserState::EsperandoGenero => "ESPERANDO_GENERO".to_string(),
            UserState::EsperandoDireccion => "ESPERANDO_DIRECCION".to_string(),
            UserState::EsperandoReceta => "ESPERANDO_RECETA".to_string(),
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
            "CONFIRMANDO_PEDIDO" => Ok(UserState::ConfirmandoPedido),
            "ESPERANDO_PRIMER_NOMBRE" => Ok(UserState::EsperandoPrimerNombre),
            "ESPERANDO_APELLIDO_PATERNO" => Ok(UserState::EsperandoApellidoPaterno),
            "ESPERANDO_APELLIDO_MATERNO" => Ok(UserState::EsperandoApellidoMaterno),
            "ESPERANDO_EMAIL" => Ok(UserState::EsperandoEmail),
            "ESPERANDO_CURP" => Ok(UserState::EsperandoCurp),
            "ESPERANDO_GENERO" => Ok(UserState::EsperandoGenero),
            "ESPERANDO_DIRECCION" => Ok(UserState::EsperandoDireccion),
            "ESPERANDO_RECETA" => Ok(UserState::EsperandoReceta),
            _ => Err(format!("Estado desconocido: {}", s)),
        }
    }
}
