use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Write, BufReader};
use std::sync::Arc;
use common::error::Resultado;
use regex::Regex;
use std::io::{prelude::*};

use super::pago::Pago;

#[derive(Clone, PartialEq)]
pub enum EstadoTransaccion {
    Prepare,
    Commit,
    Abort
}

#[derive(Clone)]
pub struct Transaccion {
    pub id: usize,
    pub id_pago: usize,
    pub id_pago_prox: usize,
    pub estado: EstadoTransaccion,
    pub reintento: bool,
    pub pago: Option<Pago>
}

impl Transaccion {
    pub fn new(id: usize, id_pago: usize, id_pago_prox: usize, estado: EstadoTransaccion, reintento: bool) -> Self {
        Self { id, id_pago, id_pago_prox, estado, reintento, pago: None }
    }

    pub fn default(id: usize) -> Self { 
        Self { 
            id, 
            id_pago: 0, 
            id_pago_prox: 0, 
            estado: EstadoTransaccion::Prepare, 
            reintento: false, 
            pago: None
        } 
    }

    pub fn get_pago(&self) -> Option<Pago> {
        self.pago.as_ref().and_then(|p| Some(p.clone()))
    }

    pub fn prepare(&mut self) -> &Self { 
        self.estado = EstadoTransaccion::Prepare;
        self
    }

    pub fn commit(&mut self) -> &Self { 
        self.estado = EstadoTransaccion::Commit;
        self
    }

    pub fn abort(&mut self) -> &Self { 
        self.estado = EstadoTransaccion::Abort;
        self
    }

    pub fn es_reintento(&self) -> bool {
        self.reintento
    }
}


//TODOOOOOOOOOO: HAY QUE METERLE UN LOCK A TODO EL LOG O A EL HASH Y ULTIMA_TRANS PORQUE SE ESTAN COPIANDO AL CLONAR
//ESTO HACE QUE NO SE TENGA REFERENCIA EN MEMORIA DE LAS TRANSACCIONES INSERTADAS (LO IDEAL SERIA METERLE UN LOCK A TODO EL LOG)



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
        
        let mut archivo = fs::OpenOptions::new()
                         .read(true)                 
                         .write(true)
                         .append(true)
                         .create(true)
                         .open(path)?;
        
        let mut log = Log {
            archivo: archivo,
            siguiente_id: 1,
            log: HashMap::new(),
            ultima_trans: None
        };

        log.leer_archivo();
        
        Ok(log)
    }    

    pub fn nueva_transaccion(&self, id_pago: usize) -> Transaccion {
        //La idea es que devuelva una transaccion semi inicializada, con el id seteado.
        //Luego habra que cargarle los demas campos
        let id = self.ultima_trans.as_ref().and_then(|t| Some(t.id)).unwrap_or(0);
        let mut transaccion = Transaccion::default(id + 1);
        transaccion.id_pago = id_pago;
        transaccion.id_pago_prox = id_pago + 1;
        println!("Se crea una transaccion con id {}", transaccion.id);

        transaccion
    }

    pub fn obtener(&self, id: &usize) -> Option<Transaccion> {
        self.log.get(id).and_then(|t| Some(t.clone()))
    }

    pub fn insertar(&mut self, transaccion: &Transaccion) {        
        if let Some(t) = self.obtener(&transaccion.id) {
            if t.estado == transaccion.estado { return; }
        }
        let salida = self.formatear_transaccion(transaccion);
        self.log.insert(transaccion.id, transaccion.clone());
        writeln!(self.archivo, "{}", salida).unwrap();
        self.ultima_trans = Some(transaccion.clone());
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

    fn leer_archivo(&mut self) {
        let matcher = Regex::new(r"^(\d+),(\d+),(\d+),(COMMIT|ABORT|PREPARE),(N|R)$").unwrap();
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


    fn parsear_transaccion(&self, argumentos: regex::Captures) -> Resultado<Transaccion> {
        let trans_id = argumentos[1].parse::<usize>()?;
        let pago_id = argumentos[2].parse::<usize>()?;
        let prox_pago_id = argumentos[3].parse::<usize>()?;
        let operacion = &argumentos[4];
        let reintento = &argumentos[5] == "R";
        
        let estado = match operacion {
             "COMMIT" => EstadoTransaccion::Commit,
             "ABORT" => EstadoTransaccion::Abort,
             "PREPARE" => EstadoTransaccion::Prepare,
             _ => panic!("Estado erroneo")
        };

         Ok(Transaccion::new(trans_id, pago_id, prox_pago_id, estado, reintento))
    }

    pub fn ultima_transaccion(&self) -> Option<Transaccion> {
        self.ultima_trans.clone()
    }

    pub fn clone(&self) -> Self {
        Log { 
            archivo: self.archivo.try_clone().unwrap(),
            log: self.log.clone(),
            siguiente_id: self.siguiente_id.clone(),
            ultima_trans: self.ultima_trans.clone()
        }
    }
}