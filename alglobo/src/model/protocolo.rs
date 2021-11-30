use super::error::{ErrorApp, ErrorInterno, Resultado};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

static TAM_BUFFER: usize = 128;

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

impl Mensaje {
    pub fn new(codigo: CodigoMensaje, id_emisor: usize, id_op: usize) -> Self { Self { codigo, id_emisor, id_op } }

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
            "PREPARE" => CodigoMensaje::PREPARE { monto: parseado[3].parse::<f64>()? },
            "COMMIT" => CodigoMensaje::COMMIT,
            "ABORT" => CodigoMensaje::ABORT,
            _ => return Err(ErrorApp::Interno(ErrorInterno::new("Mensaje erroneo"))),
        };

        Ok(Mensaje::new(codigo, 
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

pub struct Protocolo {
    skt: UdpSocket,
}

impl Protocolo {
    pub fn new(direccion: String) -> Resultado<Protocolo> {
        Ok(Protocolo {
            skt: UdpSocket::bind(direccion)?,
        })
    }

    pub fn enviar(&mut self, mensaje: &Mensaje, direccion: String) -> Resultado<()> {
        let mensaje = mensaje.codificar();
        self.skt.send_to(mensaje.as_bytes(), direccion)?;
        Ok(())
    }

    pub fn recibir(&mut self, timeout: Option<Duration>) -> Resultado<Mensaje> {
        let mut buffer = Vec::with_capacity(TAM_BUFFER);
        self.skt.set_read_timeout(timeout);
        let (recibido, _) = self.skt.recv_from(&mut buffer)?;
        if recibido == 0 {
            return Err(ErrorApp::Interno(ErrorInterno::new("Timeout en recepcion")));
        }

        Mensaje::decodificar(&String::from_utf8(buffer)?)
    }

    fn clone(&self) -> Self {
        Protocolo {
            skt: self.skt.try_clone().unwrap(),
        }
    }
}
