use common::error::Resultado;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Write};

use super::config::Config;
use super::pago::Pago;

/// EstadoTransaccion representa el estado de la transaccion.
/// # Variantes
/// Prepare: simboliza el estado de prepare completo. En este estado se puede
/// realizar commit.
/// Commit: simboliza el estado commit completo. Es decir, la transacción fue
/// correctamente completada.
/// Abort: simboliza el estado abort completo. Es decir, la transacción fue
/// correctamente abortada.
#[derive(Clone, PartialEq)]
pub enum EstadoTransaccion {
    Prepare,
    Commit,
    Abort,
    Finalize,
}

/// Representa una transaccion. Contiene información sobre el pago actual y
/// sobre el pago siguiente.
#[derive(Clone)]
pub struct Transaccion {
    pub id: usize,
    pub id_pago: usize,
    pub id_pago_prox: usize,
    pub estado: EstadoTransaccion,
    pub pago: Option<Pago>,
}

impl Transaccion {
    /// Devuelve una instancia de Transaccion.
    /// Recibe el id de la transaccion, el id del pago actual, el id del pago
    /// proximo y el estado de la transaccion.
    pub fn new(id: usize, id_pago: usize, id_pago_prox: usize, estado: EstadoTransaccion) -> Self {
        Self {
            id,
            id_pago,
            id_pago_prox,
            estado,
            pago: None,
        }
    }

    /// Devuelve el pago
    pub fn get_pago(&self) -> Option<Pago> {
        self.pago.as_ref().cloned()
    }

    /// Cambiar el estado de la transacción a Prepare.
    pub fn prepare(&mut self) -> &Self {
        self.estado = EstadoTransaccion::Prepare;
        self
    }

    /// Cambiar el estado de la transacción a Commit.
    pub fn commit(&mut self) -> &Self {
        self.estado = EstadoTransaccion::Commit;
        self
    }

    /// Cambiar el estado de la transacción a Abort.
    pub fn abort(&mut self) -> &Self {
        self.estado = EstadoTransaccion::Abort;
        self
    }

    /// Cambiar el estado de la transacción a Finalize.
    pub fn finalize(&mut self) -> &Self {
        self.estado = EstadoTransaccion::Finalize;
        self
    }
}

/// Representa un log system.
pub struct Log {
    archivo: File,
    log: HashMap<usize, Transaccion>,
    siguiente_id: usize,
    ultima_trans: Option<Transaccion>,
}

impl Log {
    /// Genera una instancia de la clase.
    /// Recibe un path donde dicho archivo debe ser construido.
    pub fn new() -> Resultado<Self> {
        let archivo = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .append(true)
            .create(true)
            .open(Config::ruta_logs())?;

        let mut log = Log {
            archivo,
            siguiente_id: 1,
            log: HashMap::new(),
            ultima_trans: None,
        };

        log.leer_archivo();

        Ok(log)
    }

    /// Crea una nueva transaccion inicializada y la devuelve
    pub fn nueva_transaccion(&self, id_pago: usize, id_prox_pago: usize) -> Transaccion {
        //La idea es que devuelva una transaccion semi inicializada, con el id seteado.
        //Luego habra que cargarle los demas campos
        let id = self.ultima_trans.as_ref().map(|t| t.id).unwrap_or(0);
        Transaccion::new(id + 1, id_pago, id_prox_pago, EstadoTransaccion::Prepare)
    }

    /// Recibe un id y devuelve una transacción si lo contiene o None si no.
    pub fn obtener(&self, id: &usize) -> Option<Transaccion> {
        self.log.get(id).cloned()
    }

    /// Inserta una transacción en el log de transacciones.
    pub fn insertar(&mut self, transaccion: &Transaccion) {
        if let Some(t) = self.obtener(&transaccion.id) {
            if t.estado == transaccion.estado {
                return;
            }
        }
        let salida = self.formatear_transaccion(transaccion);
        self.log.insert(transaccion.id, transaccion.clone());
        writeln!(self.archivo, "{}", salida).expect("Error al escribir en el archivo de log");
        self.ultima_trans = Some(transaccion.clone());
    }

    /// Recibe una transaccion y devuelve un String formateado
    fn formatear_transaccion(&self, t: &Transaccion) -> String {
        let estado = match &t.estado {
            EstadoTransaccion::Commit => "COMMIT",
            EstadoTransaccion::Abort => "ABORT",
            EstadoTransaccion::Prepare => "PREPARE",
            EstadoTransaccion::Finalize => "FINALIZE",
        };

        format!("{},{},{},{}", t.id, t.id_pago, t.id_pago_prox, estado)
    }

    // TODO: Documentacion?? Es privada
    fn leer_archivo(&mut self) {
        let matcher = Regex::new(r"^(\d+),(\d+),(\d+),(COMMIT|ABORT|PREPARE|FINALIZE)$")
            .expect("Error al crear la regex, posiblemente es invalida");
        let reader = BufReader::new(&self.archivo);

        let mut ultimo_id = 0;

        for linea in reader.lines() {
            let linea = &linea.expect("Error al leer del archivo de log");
            let cap = match matcher.captures(linea) {
                None => continue,
                Some(value) => value,
            };
            //No deberia fallar si ya paso la regex
            let transaccion = self
                .parsear_transaccion(cap)
                .expect("Error al parsear transaccion");
            self.siguiente_id = std::cmp::max(self.siguiente_id - 1, transaccion.id) + 1;
            ultimo_id = transaccion.id;
            self.log.insert(transaccion.id, transaccion);
        }

        self.ultima_trans = self.log.get(&ultimo_id).cloned();
    }

    /// Recibe argumentos para crear una transaccion y la devuelve
    fn parsear_transaccion(&self, argumentos: regex::Captures) -> Resultado<Transaccion> {
        let trans_id = argumentos[1].parse::<usize>()?;
        let pago_id = argumentos[2].parse::<usize>()?;
        let prox_pago_id = argumentos[3].parse::<usize>()?;
        let operacion = &argumentos[4];

        let estado = match operacion {
            "COMMIT" => EstadoTransaccion::Commit,
            "ABORT" => EstadoTransaccion::Abort,
            "PREPARE" => EstadoTransaccion::Prepare,
            "FINALIZE" => EstadoTransaccion::Finalize,
            _ => panic!("Estado erroneo"),
        };

        Ok(Transaccion::new(trans_id, pago_id, prox_pago_id, estado))
    }

    /// Devuelve la última transacción.
    pub fn ultima_transaccion(&self) -> Option<Transaccion> {
        self.ultima_trans.clone()
    }
}
