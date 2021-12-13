use super::pago::Pago;
use common::error::Resultado;
use regex::Regex;
use std::fs;
use std::fs::File;
use std::io::{self, prelude::*};

/// ParserFallidos implementa el parseo de los request fallidos que se
/// encuentran en un archivo dado.
pub struct ParserFallidos {
    archivo: File,
    matcher: Regex,
}

impl ParserFallidos {
    /// Devuelve una instancia de ParserFallidos.
    /// Recibe la ruta del archivo a ser procesado.
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

    /// Parsea un pago correspondiente al id pasado por parámetro.
    /// Devuelve el pago parseado si lo encuentra o None si llega al final
    /// del archivo.
    pub fn parsear(&mut self, id: usize) -> Resultado<Option<Pago>> {
        self.archivo.seek(io::SeekFrom::Start(0))?;
        let lector = io::BufReader::new(&self.archivo);
        let mut pago = None;

        let lines = lector.lines().map(|linea| {
            let linea = linea.expect("Error al leer del archivo de fallidos");

            if let Some(cap) = self.matcher.captures(&linea) {

                //Los parseos no deberian fallar si ya se paso la regex

                if cap[1].parse::<usize>().expect("Error al parsear id de pago") == id {
                    println!("[ParserFallidos] Reintento de pago de id '{}' con un monto de aerolinea '{}' y monto de hotel de '{}'",
                        &cap[1], &cap[2], &cap[3]);
                    pago = Some(Pago::new(
                        cap[1].parse::<usize>().expect("Error al parsear id de pago"),
                        cap[2].parse::<f64>().expect("Error al parsear monto de aerolinea"),
                        cap[3].parse::<f64>().expect("Error al parsear monto de hotel"),
                    ));

                    "".to_string()
                } else {
                    linea + "\n"
                }
            } else {
                "".to_string()
            }
        }).collect::<Vec<String>>().join("");

        if pago.is_some() {
            // TODO: ver esto, cambiar el path o si se puede hacer distinto
            fs::write("./files/fallidos.csv", lines).expect("Error al escribir en el archivo de fallidos"); 
        }
        Ok(pago)
    }

    // Escribe un pago fallido en el archivo de fallidos.
    pub fn escribir_fallido(&mut self, pago: Pago) {
        let _ = self.archivo.seek(io::SeekFrom::End(0));
        let salida = self.formatear_pago(pago);

        match writeln!(self.archivo, "{}", salida) {
            Ok(v) => v,
            Err(e) => println!("[ParserFallidos] No se pudo escribir en el archivo : {}", e),
        }
    }

    // TODO: Documentacion?? Es privada
    fn formatear_pago(&self, pago: Pago) -> String {
        format!(
            "{},{:.2},{:.2}",
            pago.get_id(),
            pago.get_monto_aerolinea(),
            pago.get_monto_hotel()
        )
    }
}
