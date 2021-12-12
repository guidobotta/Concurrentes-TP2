/// DNS implementa la traduccion de direcciones segun el id y el tipo de
/// proceso.
pub struct DNS {}

impl DNS {
    /// Devuelve la dirección asociada al webservice correspondiente al id
    /// pasado por parámetro.
    pub fn direccion_webservice(id: &usize) -> String {
        format!("127.0.0.1:500{}", *id)
    }

    /// Devuelve la dirección asociada al nodo de alglobo correspondiente al id
    /// pasado por parámetro. Esta dirección es la utilizada para la
    /// comunicación con los webservices.
    pub fn direccion_alglobo(id: &usize) -> String {
        format!("127.0.0.1:600{}", *id)
    }

    /// Devuelve la dirección asociada al nodo de alglobo correspondiente al id
    /// pasado por parámetro. Esta dirección es la utilizada para la
    /// comunicación con otros nodos de alglobo.
    pub fn direccion_lider(id: &usize) -> String {
        format!("127.0.0.1:700{}", *id)
    }
}
