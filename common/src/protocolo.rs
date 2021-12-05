use super::error::{ErrorApp, ErrorInterno, Resultado};
use std::net::UdpSocket;
use std::time::Duration;
use super::mensaje::Mensaje;

static TAM_BUFFER: usize = 256;

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
        let mut buffer = vec![0; TAM_BUFFER];
        self.skt.set_read_timeout(timeout); // TODO: manejar resultado
        let (recibido, src) = self.skt.recv_from(&mut buffer)?;

        if recibido == 0 {
            return Err(ErrorApp::Interno(ErrorInterno::new("Timeout en recepcion")));
        }
        Mensaje::decodificar(&String::from_utf8(buffer[..recibido].to_vec()).unwrap())
    }

    pub fn clone(&self) -> Self {
        Protocolo {
            skt: self.skt.try_clone().unwrap(),
        }
    }
}
