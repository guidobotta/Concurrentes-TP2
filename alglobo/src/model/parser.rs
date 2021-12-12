use common::error::Resultado;
use super::pago::Pago;
use regex::Regex;

/// Parser implementa el parseo de los request que se encuentran en un archivo
/// dado.
#[derive(Debug)]
pub struct Parser {
    lector: io::BufReader<File>,
    matcher: Regex,
    posicion: usize,
}

use std::{
    fs::File,
    io::{self, prelude::*},
};

impl Parser {
    /// Devuelve una instancia de Parser.
    /// Recibe la ruta del archivo a ser procesado.
    pub fn new(path: impl AsRef<std::path::Path>) -> Resultado<Parser> {
        let file = File::open(path)?;
        let parser = Parser {
            lector: io::BufReader::new(file),
            matcher: Regex::new(r"^(\d+),(\d+\.\d{2}),(\d+\.\d{2})$")?,
            posicion: 0,
        };

        Ok(parser)
    }

    /// Parsea el archivo de request.
    /// Metodo bloqueante, finaliza al terminar de procesar los requests.
    pub fn parsear_nuevo(&mut self, id: Option<usize>) -> Resultado<Option<Pago>> {
        
        loop {
            let mut buffer = String::new();
            let bytes = self.lector.read_line(&mut buffer)?;

            if bytes == 0 {
                return Ok(None);
            }

            buffer = buffer.replace("\n", "");

            let cap = match self.matcher.captures(&buffer) {
                None => continue,
                Some(value) => value,
            };

            

            self.posicion = cap[1].parse::<usize>().unwrap();
            if let Some(id_buscado) = id {
                if id_buscado > self.posicion {continue}
            }
            
            println!("[Parser] Nuevo pago de id '{}' con un monto de aerolinea '{}' y monto de hotel de '{}'",
                    &cap[1], &cap[2], &cap[3]);

            //Si pasa la regex sabemos el casteo no fallara.
            let pago = Pago::new(
                self.posicion,
                cap[2].parse::<f64>().unwrap(),
                cap[3].parse::<f64>().unwrap(),
            );

            return Ok(Some(pago));
        }
    }
}
