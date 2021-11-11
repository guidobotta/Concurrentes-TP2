use super::error::Resultado;
use super::pago::Pago;
use regex::Regex;

/// Clase utilizada para parsear los distintos request recibidos mediante texto.
#[derive(Debug)]
pub struct Parser {
    reader: io::BufReader<File>,
    matcher: Regex,
}

use std::{
    fs::File,
    io::{self, prelude::*},
};

impl Parser {
    /// Devuelve una instancia de Parser.
    /// Recibe el archivo del que debe leer los request y el logger donde debe notificar lo ejecutado.
    pub fn new(path: impl AsRef<std::path::Path>) -> Resultado<Parser> {
        let file = File::open(path)?;
        let parser = Parser {
            reader: io::BufReader::new(file),
            matcher: Regex::new(r"^(\d+),(\d+\.\d{2}),(\d+\.\d{2})$")?,
        };

        Ok(parser)
    }

    /// Parsea el archivo de request.
    /// Metodo bloqueante, finaliza al terminar de procesar los requests.
    pub fn parsear_pago(&mut self) -> Resultado<Option<Pago>> {
        let mut buffer = String::new();

        loop {
            let bytes = self.reader.read_line(&mut buffer)?;

            if bytes == 0 {
                return Ok(None);
            }

            buffer = buffer.replace("\n", "");

            let cap = match self.matcher.captures(&buffer) {
                None => continue,
                Some(value) => value,
            };

            println!("[Parser] Nuevo pago de id '{}' con un monto de aerolinea '{}' y monto de hotel de '{}'",
                    &cap[1], &cap[2], &cap[3]);

            //Si pasa la regex sabemos el casteo no fallara.
            let pago = Pago::new(
                cap[1].parse::<usize>().unwrap(),
                cap[2].parse::<f64>().unwrap(),
                cap[3].parse::<f64>().unwrap(),
            );

            return Ok(Some(pago));
        }
    }
}
