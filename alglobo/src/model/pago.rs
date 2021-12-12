// TODO: Documentacion
#[derive(Clone)]
pub struct Pago {
    id: usize,
    monto_aerolinea: f64,
    monto_hotel: f64,
}

// TODO: Documentacion
impl Pago {
    pub fn new(id: usize, monto_aerolinea: f64, monto_hotel: f64) -> Pago {
        let pago = Pago {
            id,
            monto_aerolinea,
            monto_hotel,
        };
        pago
    }

    // TODO: Documentacion
    pub fn get_id(&self) -> usize {
        self.id
    }

    // TODO: Documentacion
    pub fn get_monto_aerolinea(&self) -> f64 {
        self.monto_aerolinea
    }

    // TODO: Documentacion
    pub fn get_monto_hotel(&self) -> f64 {
        self.monto_hotel
    }
}
