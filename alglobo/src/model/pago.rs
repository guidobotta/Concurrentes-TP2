/// Pago representa un pago a realizar. Contiene el monto a pagar de la
/// aerolínea y del hotel.
#[derive(Clone)]
pub struct Pago {
    id: usize,
    monto_aerolinea: f64,
    monto_hotel: f64,
}

impl Pago {
    /// Devuelve una instancia de Pago.
    /// Recibe el id del pago, el monto de la aerolinea y el monto del hotel.
    pub fn new(id: usize, monto_aerolinea: f64, monto_hotel: f64) -> Pago {
        Pago {
            id,
            monto_aerolinea,
            monto_hotel,
        }
    }

    /// Devuelve el id del pago.
    pub fn get_id(&self) -> usize {
        self.id
    }

    /// Devuelve el monto de la aerolínea.
    pub fn get_monto_aerolinea(&self) -> f64 {
        self.monto_aerolinea
    }

    /// Devuelve el monto del hotel.
    pub fn get_monto_hotel(&self) -> f64 {
        self.monto_hotel
    }
}
