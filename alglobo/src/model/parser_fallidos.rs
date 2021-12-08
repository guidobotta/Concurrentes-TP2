use std::fs;
use std::fs::File;
use common::error::Resultado;
use regex::Regex;
use std::{
    io::{self, prelude::*},
};
use super::pago::Pago;

/// Representa un log system.
pub struct ParserFallidos {
    archivo: File,
    matcher: Regex,
}

impl ParserFallidos {
    /// Genera una instancia de la clase.
    /// Recibe un path donde dicho archivo debe ser construido.
    pub fn new(path: String) -> Resultado<Self> {
        Ok(ParserFallidos {
            archivo: fs::OpenOptions::new()
                    .write(true)
                    .read(true)
                    .append(true)
                    .create(true)
                    .open(path)?,
            matcher: Regex::new(r"^(\d+),(\d+\.\d{2}),(\d+\.\d{2})$")?,
        })
    }

    pub fn parsear_fallido(&mut self, id: usize) -> Resultado<Option<Pago>> {
        self.archivo.seek(io::SeekFrom::Start(0))?;
        let lector = io::BufReader::new(&self.archivo);

        for line in lector.lines() {
            let linea = line?;
            let cap = match self.matcher.captures(&linea) {
                None => continue,
                Some(value) => value,
            };

            if cap[1].parse::<usize>()? == id {
                return Ok(Some(Pago::new(
                    cap[1].parse::<usize>().unwrap(),
                    cap[2].parse::<f64>().unwrap(),
                    cap[3].parse::<f64>().unwrap(),
                )));
            }
        }

        Ok(None)
    }

    pub fn escribir_fallido(&mut self, pago: Pago) {
        self.archivo.seek(io::SeekFrom::End(0));
        let salida = self.formatear_pago(pago);

        match writeln!(self.archivo, "{}", salida) {
            Ok(v) => v,
            Err(e) => println!(
                "[ParserFallidos] No se pudo escribir en el archivo : {}",
                e
            ),
        }
    }
    
    fn formatear_pago(&self, pago: Pago) -> String {
        format!("{},{:.2},{:.2}", pago.get_id(), pago.get_monto_aerolinea(), pago.get_monto_hotel())
    }
}