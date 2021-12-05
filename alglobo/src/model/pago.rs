#[derive(Clone)]
pub struct Pago {
    id: usize,
    monto_aerolinea: f64,
    monto_hotel: f64,
}

impl Pago {
    pub fn new(id: usize, monto_aerolinea: f64, monto_hotel: f64) -> Pago {
        let pago = Pago {
            id,
            monto_aerolinea,
            monto_hotel,
        };
        pago
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn get_monto_aerolinea(&self) -> f64 {
        self.monto_aerolinea
    }

    pub fn get_monto_hotel(&self) -> f64 {
        self.monto_hotel
    }
}
