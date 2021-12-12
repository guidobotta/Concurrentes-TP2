use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Write, BufReader};
use std::sync::Arc;
use common::error::Resultado;
use regex::Regex;
use std::io::{prelude::*};

use super::pago::Pago;

// TODO: Documentacion
#[derive(Clone, PartialEq)]
pub enum EstadoTransaccion {
    Prepare,
    Commit,
    Abort
}

// TODO: Documentacion
#[derive(Clone)]
pub struct Transaccion {
    pub id: usize,
    pub id_pago: usize,
    pub id_pago_prox: usize,
    pub estado: EstadoTransaccion,
    pub pago: Option<Pago>
}

impl Transaccion {
    // TODO: Documentacion
    pub fn new(id: usize, id_pago: usize, id_pago_prox: usize, estado: EstadoTransaccion) -> Self {
        Self { id, id_pago, id_pago_prox, estado, pago: None }
    }

    // TODO: Documentacion
    pub fn get_pago(&self) -> Option<Pago> {
        self.pago.as_ref().and_then(|p| Some(p.clone()))
    }

    // TODO: Documentacion
    pub fn prepare(&mut self) -> &Self { 
        self.estado = EstadoTransaccion::Prepare;
        self
    }

    // TODO: Documentacion
    pub fn commit(&mut self) -> &Self { 
        self.estado = EstadoTransaccion::Commit;
        self
    }

    // TODO: Documentacion
    pub fn abort(&mut self) -> &Self { 
        self.estado = EstadoTransaccion::Abort;
        self
    }
}

/// Representa un log system.
pub struct Log {
    archivo: File,
    log: HashMap<usize, Transaccion>,
    siguiente_id: usize,
    ultima_trans: Option<Transaccion>
}

impl Log {
    /// Genera una instancia de la clase.
    /// Recibe un path donde dicho archivo debe ser construido.
    pub fn new(path: String) -> Resultado<Self> {
        
        let archivo = fs::OpenOptions::new()
                         .read(true)                 
                         .write(true)
                         .append(true)
                         .create(true)
                         .open(path)?;
        
        let mut log = Log {
            archivo,
            siguiente_id: 1,
            log: HashMap::new(),
            ultima_trans: None
        };

        log.leer_archivo();
        
        Ok(log)
    }    

    // TODO: Documentacion
    pub fn nueva_transaccion(&self, id_pago: usize, id_prox_pago: usize) -> Transaccion {
        //La idea es que devuelva una transaccion semi inicializada, con el id seteado.
        //Luego habra que cargarle los demas campos
        let id = self.ultima_trans.as_ref().and_then(|t| Some(t.id)).unwrap_or(0);
        Transaccion::new(id + 1, id_pago, id_prox_pago, EstadoTransaccion::Prepare)
    }

    // TODO: Documentacion
    pub fn obtener(&self, id: &usize) -> Option<Transaccion> {
        self.log.get(id).and_then(|t| Some(t.clone()))
    }

    // TODO: Documentacion
    pub fn insertar(&mut self, transaccion: &Transaccion) {        
        if let Some(t) = self.obtener(&transaccion.id) {
            if t.estado == transaccion.estado { return; }
        }
        let salida = self.formatear_transaccion(transaccion);
        self.log.insert(transaccion.id, transaccion.clone());
        writeln!(self.archivo, "{}", salida).unwrap();
        self.ultima_trans = Some(transaccion.clone());
    }

    // TODO: Documentacion?? Es privada
    fn formatear_transaccion(&self, t: &Transaccion) -> String {
        let estado = match &t.estado {
            EstadoTransaccion::Commit => "COMMIT",
            EstadoTransaccion::Abort => "ABORT",
            EstadoTransaccion::Prepare => "PREPARE"
        };

        format!("{},{},{},{}", t.id, t.id_pago, t.id_pago_prox, estado)
    }

    // TODO: Documentacion?? Es privada
    fn leer_archivo(&mut self) {
        let matcher = Regex::new(r"^(\d+),(\d+),(\d+),(COMMIT|ABORT|PREPARE)$").unwrap();
        let reader = BufReader::new(&self.archivo);

        let mut ultimo_id = 0;

        for linea in reader.lines() {
            let linea_unwrap = &linea.unwrap();
            let cap = match matcher.captures(linea_unwrap) {
                None => continue,
                Some(value) => value,
            };
            let transaccion = self.parsear_transaccion(cap).unwrap();
            self.siguiente_id = std::cmp::max(self.siguiente_id - 1, transaccion.id) + 1;
            ultimo_id = transaccion.id;
            self.log.insert(transaccion.id, transaccion);
        }

        self.ultima_trans = self.log.get(&ultimo_id).and_then(|t| Some(t.clone()));
    }

    // TODO: Documentacion?? Es privada
    fn parsear_transaccion(&self, argumentos: regex::Captures) -> Resultado<Transaccion> {
        let trans_id = argumentos[1].parse::<usize>()?;
        let pago_id = argumentos[2].parse::<usize>()?;
        let prox_pago_id = argumentos[3].parse::<usize>()?;
        let operacion = &argumentos[4];
        
        let estado = match operacion {
             "COMMIT" => EstadoTransaccion::Commit,
             "ABORT" => EstadoTransaccion::Abort,
             "PREPARE" => EstadoTransaccion::Prepare,
             _ => panic!("Estado erroneo")
        };

         Ok(Transaccion::new(trans_id, pago_id, prox_pago_id, estado))
    }

    // TODO: Documentacion
    pub fn ultima_transaccion(&self) -> Option<Transaccion> {
        self.ultima_trans.clone()
    }
}