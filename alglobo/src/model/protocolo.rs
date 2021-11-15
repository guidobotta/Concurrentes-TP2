use super::error::{ErrorApp, ErrorInterno, Resultado};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, UdpSocket};

static TAM_BUFFER: usize = 128;

pub enum Mensaje {
    OK { linea: usize },
    ACT { linea: usize },
}

impl Mensaje {
    pub fn codificar(&self) -> String {
        match &self {
            Mensaje::OK { linea } => format!("OK {}", linea),
            Mensaje::ACT { linea } => format!("ACT {}", linea),
        }
    }

    pub fn decodificar(mensaje_codificado: &String) -> Resultado<Mensaje> {
        let parseado = mensaje_codificado.split(' ').collect::<Vec<&str>>();

        match parseado[0] {
            "ACT" => Ok(Mensaje::ACT {
                linea: parseado[1].parse::<usize>()?,
            }),
            "OK" => Ok(Mensaje::OK {
                linea: parseado[1].parse::<usize>()?,
            }),
            _ => Err(ErrorApp::Interno(ErrorInterno::new("Mensaje erroneo"))),
        }
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

    pub fn recibir(&mut self, timeout: usize) -> Resultado<Mensaje> {
        let mut buffer = Vec::with_capacity(TAM_BUFFER);
        self.skt
            .set_read_timeout(Some(std::time::Duration::from_millis(timeout as u64)));
        let (recibido, _) = self.skt.recv_from(&mut buffer)?;
        if recibido == 0 {
            return Err(ErrorApp::Interno(ErrorInterno::new("Timeout en recepcion")));
        }

        Mensaje::decodificar(&String::from_utf8(buffer)?)
    }
}
