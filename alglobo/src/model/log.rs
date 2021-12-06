use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Write, BufReader};
use common::error::Resultado;
use regex::Regex;
use std::io::{prelude::*};

use super::pago::Pago;
use super::transaccion::Transaccion;

/// Representa un log system.
pub struct Log {
    archivo: File,
    log: HashMap<usize, Transaccion>,
    siguiente_id: usize
}

impl Log {
    /// Genera una instancia de la clase.
    /// Recibe un path donde dicho archivo debe ser construido.
    pub fn new(path: String) -> Resultado<Self> {
        
        let mut archivo = fs::OpenOptions::new()
                         .write(true)
                         .append(true)
                         .create(true)
                         .open(path)?;
        
        let mut log = Log {
            archivo: archivo,
            siguiente_id: 0,
            log: HashMap::new()
        };

        log.leer_archivo(archivo);
        
        Ok(log)
    }
    //log.nueva_transaccion() -> Transaccion
    //log.ultima_transaccion() -> Transaccion
    //log.get(Transaccion.id) -> Transaccion

    

    pub fn nueva_transaccion(&mut self) -> Transaccion {
        //La idea es que devuelva una transaccion semi inicializada, con el id seteado.
        //Luego habra que cargarle los demas campos
        Transaccion::new(self.siguiente_id, 0, 0, EstadoTransaccion::Prepare, false);
    }

    pub fn obtener(&self, id: usize) -> Transaccion {
        self.log.get(&id)
    }

    pub fn insertar(&mut self, transaccion: &Transaccion) {
        let salida = self.formatear_transaccion(transaccion);
        writeln!(self.archivo, "{}", salida).unwrap();
        self.log.insert(transaccion.id, transaccion);
    }

    fn formatear_transaccion(&self, t: &Transaccion) -> String {
        let estado = match &t.estado {
            EstadoTransaccion::Commit => "COMMIT",
            EstadoTransaccion::Abort => "ABORT",
            EstadoTransaccion::Prepare => "PREPARE"
        };

        let tipo = if t.es_reintento() { "R" } else { "N" };

        format!("{},{},{},{},{}", t.id, t.id_pago, t.id_pago_prox, estado, tipo)
    }

    fn leer_archivo(&mut self, mut archivo: File) -> Transaccion {
        let matcher = Regex::new(r"^(\d+),(\d+),(\d+),(COMMIT|ABORT|PREPARE),(N|R)$")?;
        let reader = BufReader::new(archivo);

        for linea in reader.lines() {
            let cap = match matcher.captures(&linea.unwrap()) {
                None => continue,
                Some(value) => value,
            };

            let transaccion = self.parsear_transaccion(cap);
            self.siguiente_id = std::cmp::max(self.siguiente_id - 1, transaccion.id) + 1;
            self.log.insert(transaccion.id, transaccion);
        }
    }


    fn parsear_transaccion(&self, argumentos: regex::Captures) -> Resultado<Transaccion> {
        let trans_id = argumentos[0].parse::<usize>()?;
        let pago_id = argumentos[1].parse::<usize>()?;
        let prox_pago_id = argumentos[2].parse::<usize>()?;
        let operacion = argumentos[3].to_string();
        let repetida = &argumentos[4] == "R";

        Ok(Transaccion::new(trans_id, pago_id, prox_pago_id, operacion, repetida))
    }
}