pub struct DNS {}

impl DNS {
    pub fn direccion_webservice(id: &usize) -> String {
        format!("127.0.0.1:500{}", *id) // TODO: Mejorar
    }

    pub fn direccion_alglobo(id: &usize) -> String {
        format!("127.0.0.1:600{}", *id) // TODO: Mejorar
    }
}
