use std::fs;
use std::fs::File;
use std::io::Write;
use common::error::Resultado;

use super::pago::Pago;

/// Representa un log system.
pub struct EscritorFallidos {
    archivo: File
}

impl EscritorFallidos {
    /// Genera una instancia de la clase.
    /// Recibe un path donde dicho archivo debe ser construido.
    pub fn new(path: String) -> Resultado<Self> {
        Ok(EscritorFallidos {
            archivo: fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(path)?
        })
    }

    pub fn escribir_fallido(&mut self, pago: Pago) {
        let salida = self.formatear_pago(pago);
        match writeln!(self.archivo, "{}", salida) {
            Ok(v) => v,
            Err(e) => println!(
                "[EscritorFallidos] No se pudo escribir en el archivo : {}",
                e
            ),
        }
    }
    
    fn formatear_pago(&self, pago: Pago) -> String {
        format!("{},{:.2},{:.2}", pago.get_id(), pago.get_monto_aerolinea(), pago.get_monto_hotel())
    }
}