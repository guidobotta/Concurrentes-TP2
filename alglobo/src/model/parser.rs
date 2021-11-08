use super::logger::Logger;
use super::request::Request;
use super::{env::Configuration, error::AppResult};
use rand::Rng;
use regex::Regex;
use std::ops::Range;
use std::{thread, time};

/// Clase utilizada para parsear los distintos request recibidos mediante texto.
#[derive(Debug)]
pub struct Parser {
    reader: io::BufReader<File>,
    matcher: Regex,
    logger: Logger,
    req_arrival_range: Range<usize>,
}

use std::{
    fs::File,
    io::{self, prelude::*},
};

impl Parser {
    /// Devuelve una instancia de Parser.
    /// Recibe el archivo del que debe leer los request y el logger donde debe notificar lo ejecutado.
    pub fn open(
        path: impl AsRef<std::path::Path>,
        in_logger: Logger,
        config: Configuration,
    ) -> AppResult<Self> {
        let file = File::open(path)?;

        let parser = Parser {
            reader: io::BufReader::new(file),
            matcher: Regex::new(r"^([A-Z]{3}),([A-Z]{3}),([A-z]+),([PV])$")?,
            logger: in_logger.clone(),
            req_arrival_range: Range {
                start: config.parser_min_req_arrival_time,
                end: config.parser_max_req_arrival_time,
            },
        };

        in_logger.log_info(String::from(
            "[Parser] CSV with requests successfully opened",
        ));
        Ok(parser)
    }

    /// Parsea el archivo de request.
    /// Metodo bloqueante, finaliza al terminar de procesar los requests.
    pub fn parse_request(&mut self) -> AppResult<Option<Request>> {
        loop {
            let mut buffer = vec![];

            let bytes = self.reader.read_until(b'\n', &mut buffer)?;

            if bytes == 0 {
                return Ok(None);
            }

            let buffer = String::from_utf8(buffer)?.replace("\n", "");

            let cap = match self.matcher.captures(&buffer) {
                None => {
                    //Si no matchea se ignora el pedido
                    self.logger.log_warning(String::from(
                        "[Parser] Invalid line on Requests CSV, continuing anyway",
                    ));
                    continue;
                }
                Some(value) => value
            };

            // Simulacion tiempo de arribo
            let mut rng = rand::thread_rng();
            let arrival_time = rng.gen_range(self.req_arrival_range.clone());
            thread::sleep(time::Duration::from_millis(arrival_time as u64));

            self.logger.log_info(format!("[Parser] Request read from '{}' to '{}' flying with '{}' requesting hotel '{}' ",
                    &cap[1], &cap[2], &cap[3], &cap[4]=="P"));

            let request = Request::new(&cap[1], &cap[2], &cap[3], &cap[4] == "P")?;
            return Ok(Some(request));
        }
    }
}
