pub struct DNS {}

impl DNS {
    pub fn direccion_webservice(id: &usize) -> String {
        format!("127.0.0.1:500{}", *id)
    }

    pub fn direccion_alglobo(id: &usize) -> String {
        format!("127.0.0.1:600{}", *id)
    }

    pub fn direccion_lider(id: &usize) -> String {
        format!("127.0.0.1:700{}", *id)
    }
}
