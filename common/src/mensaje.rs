use super::error::{ErrorApp, ErrorInterno, Resultado};

#[derive(Clone, PartialEq)]
pub enum CodigoMensaje {
    PREPARE { monto: f64 },
    READY,
    COMMIT,
    ABORT
}

#[derive(Clone)]
pub struct Mensaje {
    pub codigo: CodigoMensaje,
    pub id_emisor: usize,
    pub id_op: usize
}

use std::any::type_name;

fn type_of<T>(_: T) -> &'static str {
    type_name::<T>()
}

impl Mensaje {
    pub fn new(codigo: CodigoMensaje, id_emisor: usize, id_op: usize) -> Self { 
        Self { codigo, id_emisor, id_op } 
    }

    pub fn codificar(&self) -> String {
        match &self.codigo {
            CodigoMensaje::PREPARE { monto } => format!("PREPARE {} {} {}", self.id_emisor, self.id_op, monto),
            CodigoMensaje::COMMIT => format!("COMMIT {} {}", self.id_emisor, self.id_op),
            CodigoMensaje::READY => format!("READY {} {}", self.id_emisor, self.id_op),
            CodigoMensaje::ABORT => format!("ABORT {} {}", self.id_emisor, self.id_op),
        }
    }

    pub fn decodificar(mensaje_codificado: &String) -> Resultado<Mensaje> {
        let parseado = mensaje_codificado.split(' ').collect::<Vec<&str>>();
        let codigo = match parseado[0] {
            "PREPARE" => CodigoMensaje::PREPARE { monto: parseado[3].parse::<f64>().unwrap() },
            "COMMIT" => CodigoMensaje::COMMIT,
            "ABORT" => CodigoMensaje::ABORT,
            "READY" => CodigoMensaje::READY,
            _ => return Err(ErrorApp::Interno(ErrorInterno::new(&format!("Mensaje erroneo: {}", parseado[0])))),
        };

        Ok(Mensaje::new(
            codigo, 
            parseado[1].parse::<usize>()?, 
            parseado[2].parse::<usize>()?
        ))
    }
}

impl PartialEq for Mensaje {
    fn eq(&self, otro: &Self) -> bool {
        self.codigo == otro.codigo && self.id_op == self.id_op
    }
}